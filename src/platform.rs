use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn home() -> Result<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    Ok(std::env::var_os(variable)
        .with_context(|| format!("{variable} is not set"))?
        .into())
}

pub fn is_executable(path: &Path, metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = path;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        let _ = metadata;
        path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("com")
        })
    }
}

pub fn private_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(windows)]
    {
        fs::create_dir_all(path)?;
        // Protect both the directory and newly created cookie files with the current user's ACL.
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", r#"
$ErrorActionPreference = 'Stop'
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = [System.Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
Set-Acl -LiteralPath $env:CPG_PRIVATE_DIRECTORY -AclObject $acl
"#])
            .env("CPG_PRIVATE_DIRECTORY", path)
            // Windows PowerShell cannot load PowerShell 7's modules.
            .env_remove("PSModulePath")
            .output().context("Cannot secure the cookie directory using PowerShell")?;
        anyhow::ensure!(
            output.status.success(),
            "Cannot secure cookie directory: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

// Publish atomically without replacing even an empty directory created by another process.
pub fn publish_directory(from: &Path, to: &Path) -> Result<()> {
    #[cfg(unix)]
    let success = {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let from = CString::new(from.as_os_str().as_bytes())?;
        let to = CString::new(to.as_os_str().as_bytes())?;
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                from.as_ptr(),
                libc::AT_FDCWD,
                to.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(target_os = "macos")]
        let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
        result == 0
    };
    #[cfg(windows)]
    let success = {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(from.as_ptr(), to.as_ptr(), 0) != 0
        }
    };
    if !success {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[derive(Default)]
pub struct MemoryMonitor {
    #[cfg(not(target_os = "linux"))]
    system: sysinfo::System,
    #[cfg(windows)]
    members: std::collections::HashMap<sysinfo::Pid, u64>,
}

impl MemoryMonitor {
    pub fn usage(&mut self, group: u32) -> Result<u64> {
        #[cfg(target_os = "linux")]
        {
            let mut memory = 0;
            // ponytail: polling misses brief peaks; use delegated cgroups for strict accounting.
            for process in procfs::process::all_processes()? {
                let stat = match process.and_then(|p| p.stat()) {
                    Ok(stat) => stat,
                    Err(procfs::ProcError::NotFound(_))
                    | Err(procfs::ProcError::PermissionDenied(_)) => continue,
                    Err(error) => return Err(error.into()),
                };
                if stat.pgrp == group as i32 {
                    memory += stat.rss * procfs::page_size();
                }
            }
            Ok(memory)
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                sysinfo::ProcessRefreshKind::nothing().with_memory(),
            );
            #[cfg(target_os = "macos")]
            return Ok(self
                .system
                .processes()
                .values()
                .filter(|process| unsafe {
                    libc::getpgid(process.pid().as_u32() as i32) == group as i32
                })
                .map(sysinfo::Process::memory)
                .sum());
            #[cfg(windows)]
            {
                self.members.retain(|pid, started| {
                    self.system
                        .process(*pid)
                        .is_some_and(|process| process.start_time() == *started)
                });
                // ponytail: polling can miss descendants orphaned between samples; Job Object accounting can provide stricter limits.
                loop {
                    let before = self.members.len();
                    for process in self.system.processes().values() {
                        if process.pid().as_u32() == group
                            || process
                                .parent()
                                .is_some_and(|parent| self.members.contains_key(&parent))
                        {
                            self.members.insert(process.pid(), process.start_time());
                        }
                    }
                    if before == self.members.len() {
                        break;
                    }
                }
                Ok(self
                    .system
                    .processes()
                    .values()
                    .filter(|process| self.members.contains_key(&process.pid()))
                    .map(sysinfo::Process::memory)
                    .sum())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_does_not_replace_existing_directory() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let target = directory.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("solution"), "source").unwrap();
        fs::create_dir(&target).unwrap();
        assert!(publish_directory(&source, &target).is_err());
        assert!(source.join("solution").is_file());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        fs::remove_dir(&target).unwrap();
        publish_directory(&source, &target).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("solution")).unwrap(),
            "source"
        );
    }
}
