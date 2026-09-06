use crate::{
    cli::ListMode,
    config::{Config, Paths},
    model::{Metadata, Problem, ResourceRef},
    runner,
    services::Services,
};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::CString,
    fs,
    io::ErrorKind,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const METADATA: &str = ".cpg.toml";

pub fn read_metadata(directory: &Path) -> Result<Metadata> {
    let path = directory.join(METADATA);
    toml::from_str(
        &fs::read_to_string(&path).with_context(|| format!("Cannot read {}", path.display()))?,
    )
    .with_context(|| format!("Invalid metadata: {}", path.display()))
}

pub fn locate(directory: &Path) -> Result<Metadata> {
    find_metadata(directory)?
        .map(|(_, metadata)| metadata)
        .with_context(|| {
            format!(
                "No .cpg.toml found in {} or its parents",
                directory.display()
            )
        })
}

pub fn find_metadata(directory: &Path) -> Result<Option<(PathBuf, Metadata)>> {
    for parent in directory.ancestors() {
        match fs::symlink_metadata(parent.join(METADATA)) {
            Ok(_) => return Ok(Some((parent.to_owned(), read_metadata(parent)?))),
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(None)
}

pub fn checksum(source: &[u8]) -> String {
    format!("{:x}", Sha256::digest(source))
}

fn template_checksums(directory: &Path) -> Result<BTreeMap<PathBuf, String>> {
    let mut checksums = BTreeMap::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            for (path, checksum) in template_checksums(&entry.path())? {
                checksums.insert(PathBuf::from(entry.file_name()).join(path), checksum);
            }
        } else if entry.file_type()?.is_file() && entry.file_name() != METADATA {
            checksums.insert(
                PathBuf::from(entry.file_name()),
                checksum(&fs::read(entry.path())?),
            );
        }
    }
    Ok(checksums)
}

fn write_metadata(directory: &Path, metadata: &Metadata) -> Result<()> {
    fs::write(directory.join(METADATA), toml::to_string_pretty(metadata)?)?;
    Ok(())
}

fn template(
    paths: &Paths,
    name: &str,
    destination: &Path,
    setup: &[String],
    metadata: &Metadata,
    interrupted: &AtomicBool,
) -> Result<()> {
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    let source = paths.config.join(format!("{name}_template"));
    match fs::metadata(&source) {
        Ok(_) => copy_tree(&source, destination)?,
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
    }
    write_metadata(destination, metadata)?;
    for command in setup {
        tracing::info!("Running [setup.{name}]: {command}");
        runner::setup(command, destination, interrupted)
            .with_context(|| format!("[setup.{name}] failed in {}", destination.display()))?;
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    ensure!(
        !fs::symlink_metadata(source)?.file_type().is_symlink(),
        "Template symlinks are not supported: {}",
        source.display()
    );
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "Template symlinks are not supported: {}",
            entry.path().display()
        );
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            ensure!(
                kind.is_file(),
                "Unsupported template file: {}",
                entry.path().display()
            );
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

pub(crate) fn safe_id(id: &str) -> Result<&str> {
    let mut components = Path::new(id).components();
    ensure!(
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
        "Invalid directory ID: {id}"
    );
    Ok(id)
}

fn write_problem(
    paths: &Paths,
    config: &Config,
    destination: &Path,
    problem: Problem,
    single: bool,
    interrupted: &AtomicBool,
) -> Result<()> {
    fs::create_dir_all(destination)?;
    let metadata = Metadata::Problem {
        reference: problem.reference.clone(),
        title: problem.title.clone(),
        template_checksums: BTreeMap::new(),
    };
    if single {
        template(
            paths,
            "workspace",
            destination,
            &config.setup.workspace,
            &metadata,
            interrupted,
        )?;
    }
    template(
        paths,
        "problem",
        destination,
        &config.setup.problem,
        &metadata,
        interrupted,
    )?;
    if single {
        template(
            paths,
            "single_problem",
            destination,
            &config.setup.single_problem,
            &metadata,
            interrupted,
        )?;
    }
    let template_checksums = template_checksums(destination)?;
    fs::create_dir_all(destination.join("test"))?;
    for (i, sample) in problem.samples.iter().enumerate() {
        let name = format!("sample-{}", i + 1);
        fs::write(
            destination.join("test").join(format!("{name}.in")),
            &sample.input,
        )?;
        fs::write(
            destination.join("test").join(format!("{name}.out")),
            &sample.output,
        )?;
    }
    tracing::info!(
        "Saved {} sample case(s) for {} ({})",
        problem.samples.len(),
        problem.reference.id,
        problem.title
    );
    write_metadata(
        destination,
        &Metadata::Problem {
            reference: problem.reference,
            title: problem.title,
            template_checksums,
        },
    )
}

pub fn download(
    paths: &Paths,
    config: &Config,
    services: &Services,
    resource: ResourceRef,
    interrupted: &AtomicBool,
) -> Result<PathBuf> {
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    let root = config.root()?;
    let (service, category, id) = match &resource {
        ResourceRef::Problem(p) => (p.service, "problems", &p.id),
        ResourceRef::Contest(c) => (c.service, "contests", &c.id),
    };
    let parent = root.join(service.as_str()).join(category);
    let destination = parent.join(safe_id(id)?);
    let _span = tracing::info_span!("download", service = service.as_str(), id).entered();
    ensure!(
        !destination.try_exists()?,
        "Destination already exists: {}",
        destination.display()
    );
    fs::create_dir_all(&parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".cpg-")
        .tempdir_in(&parent)?;
    match resource {
        ResourceRef::Problem(p) => {
            tracing::info!("Downloading problem {}...", p.url);
            write_problem(
                paths,
                config,
                staging.path(),
                services.backend(p.service).fetch_problem(&p)?,
                true,
                interrupted,
            )?;
        }
        ResourceRef::Contest(c) => {
            tracing::info!("Fetching contest {}...", c.url);
            let contest = services.backend(c.service).fetch_contest(&c)?;
            tracing::info!(
                "Preparing {} problem(s) from {}...",
                contest.problems.len(),
                contest.title
            );
            let metadata = Metadata::Contest(contest.clone());
            template(
                paths,
                "workspace",
                staging.path(),
                &config.setup.workspace,
                &metadata,
                interrupted,
            )?;
            template(
                paths,
                "contest",
                staging.path(),
                &config.setup.contest,
                &metadata,
                interrupted,
            )?;
            let zfill_length = contest.problems.len().to_string().len();
            for (i, p) in contest.problems.iter().enumerate() {
                tracing::info!(
                    "Downloading {} ({}/{})...",
                    p.url,
                    i + 1,
                    contest.problems.len()
                );
                let destination = staging.path().join(format!(
                    "{:0zfill$}_{}",
                    i + 1,
                    safe_id(&p.id)?,
                    zfill = zfill_length
                ));
                write_problem(
                    paths,
                    config,
                    &destination,
                    services.backend(p.service).fetch_problem(p)?,
                    false,
                    interrupted,
                )?;
            }
            write_metadata(staging.path(), &Metadata::Contest(contest))?;
        }
    }
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    let from = CString::new(staging.path().as_os_str().as_bytes())?;
    let to = CString::new(destination.as_os_str().as_bytes())?;
    // RENAME_NOREPLACE also protects a destination created while the download was running.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("Cannot publish downloaded directory");
    }
    tracing::info!("Created workspace: {}", destination.display());
    Ok(destination)
}

pub fn list(config: &Config, mode: ListMode) -> Result<Vec<PathBuf>> {
    let root = config.root()?;
    let mut found = Vec::new();
    if !root.try_exists()? {
        return Ok(found);
    }
    fn visit(path: &Path, mode: ListMode, found: &mut Vec<PathBuf>) -> Result<()> {
        if path.join(METADATA).try_exists()? {
            let metadata = read_metadata(path)?;
            let include = match mode {
                ListMode::Workspace => true,
                ListMode::Contests => metadata.is_contest(),
                ListMode::Problems | ListMode::AllProblems => !metadata.is_contest(),
            };
            if include {
                found.push(path.to_owned());
            }
            if !metadata.is_contest() || !matches!(mode, ListMode::AllProblems) {
                return Ok(());
            }
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && !entry.file_name().as_bytes().starts_with(b".") {
                visit(&entry.path(), mode, found)?;
            }
        }
        Ok(())
    }
    visit(&root, mode, &mut found)?;
    found.sort();
    Ok(found)
}
