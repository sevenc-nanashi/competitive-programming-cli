use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, ErrorKind, Write},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

#[derive(Debug)]
pub struct Paths {
    pub config: PathBuf,
    pub cookies: PathBuf,
}

pub fn expand_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    let Ok(suffix) = path.strip_prefix("~") else {
        return Ok(path.to_owned());
    };
    let home = PathBuf::from(env::var_os("HOME").context("HOME must be set to expand ~")?);
    ensure!(home.is_absolute(), "HOME must be absolute to expand ~");
    Ok(home.join(suffix))
}

fn directory(override_var: &str, xdg_var: &str, default: &str, suffix: &str) -> Result<PathBuf> {
    if let Some(value) = env::var_os(override_var) {
        ensure!(!value.is_empty(), "{override_var} must not be empty");
        return expand_path(PathBuf::from(value));
    }
    xdg_directory(xdg_var, default, suffix)
}

fn xdg_directory(xdg_var: &str, default: &str, suffix: &str) -> Result<PathBuf> {
    let base = match env::var_os(xdg_var).filter(|v| !v.is_empty()) {
        Some(value) => expand_path(PathBuf::from(value))?,
        None => PathBuf::from(env::var_os("HOME").context("HOME is not set")?).join(default),
    };
    ensure!(base.is_absolute(), "{xdg_var} must be absolute");
    Ok(base.join(suffix))
}

impl Paths {
    pub fn discover() -> Result<Self> {
        Ok(Self {
            config: directory("CPCLI_CONFIG_HOME", "XDG_CONFIG_HOME", ".config", "cpcli")?,
            cookies: directory(
                "CPCLI_COOKIES_HOME",
                "XDG_DATA_HOME",
                ".local/share",
                "cpcli/cookies",
            )?,
        })
    }
}

fn prompt(message: &str, interrupted: &AtomicBool) -> Result<String> {
    eprint!("{message}");
    io::stderr().flush()?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = (|| -> Result<String> {
            let mut input = String::new();
            ensure!(
                io::stdin().read_line(&mut input)? > 0,
                "Initialization cancelled: no input"
            );
            Ok(input.trim().to_owned())
        })();
        let _ = sender.send(result);
    });
    loop {
        ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(error) => return Err(error.into()),
        }
    }
}

pub fn init(paths: &Paths, args: &crate::cli::Init, interrupted: &AtomicBool) -> Result<()> {
    let config_path = paths.config.join("config.toml");
    let existing = config_path.try_exists()?;
    let root = if existing {
        Config::load(paths)?.root().with_context(|| {
            format!(
                "Configuration already exists: {}; set its root manually",
                config_path.display()
            )
        })?
    } else {
        let input = prompt("Workspace root [~/cpcli]: ", interrupted)?;
        let root = match input.as_str() {
            "" => "~/cpcli",
            root => root,
        };
        Config {
            root: Some(root.into()),
            ..Config::default()
        }
        .root()?
    };
    let source = match &args.from_oj {
        Some(path) => Some(expand_path(path)?),
        None => {
            let path = xdg_directory("XDG_CONFIG_HOME", ".config", "online-judge-tools")?
                .join("prepare.config.toml");
            if path.try_exists()? {
                let answer = prompt(
                    &format!("Import [templates] from {}? [Y/n]: ", path.display()),
                    interrupted,
                )?;
                match answer.to_ascii_lowercase().as_str() {
                    "" | "y" | "yes" => Some(path),
                    "n" | "no" => None,
                    _ => anyhow::bail!("Answer yes or no to the template migration prompt"),
                }
            } else {
                None
            }
        }
    };
    let imports = match &source {
        Some(path) => read_oj_templates(path)?,
        None => Vec::new(),
    };
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    let templates = [
        (
            "workspace_template",
            "shared workspace files, e.g. Gemfile or Cargo.toml",
        ),
        (
            "problem_template",
            "solution files for every problem, e.g. solution.cpp",
        ),
        ("contest_template", "files for the contest directory"),
        (
            "single_problem_template",
            "overrides for standalone problems",
        ),
    ];
    fs::create_dir_all(&root).with_context(|| format!("Cannot create {}", root.display()))?;
    fs::create_dir_all(&paths.config)?;
    for (name, _) in templates {
        let directory = paths.config.join(name);
        fs::create_dir_all(&directory)
            .with_context(|| format!("Cannot create {}", directory.display()))?;
    }
    if existing {
        tracing::info!("Keeping existing configuration: {}", config_path.display());
    } else {
        let mut staging = tempfile::NamedTempFile::new_in(&paths.config)?;
        staging.write_all(toml::to_string(&BTreeMap::from([("root", &root)]))?.as_bytes())?;
        staging.as_file().sync_all()?;
        staging
            .persist_noclobber(&config_path)
            .with_context(|| format!("Cannot create {}", config_path.display()))?;
        tracing::info!("Created configuration: {}", config_path.display());
    }
    for template in imports {
        ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
        let destination = paths.config.join("problem_template").join(&template.path);
        let mut parent = paths.config.join("problem_template");
        // Do not follow destination symlinks outside the template directory.
        for component in template.path.components() {
            ensure!(
                fs::symlink_metadata(&parent)?.is_dir(),
                "Template destination is not a directory or is a symlink: {}",
                parent.display()
            );
            parent.push(component);
            if parent != destination {
                match fs::create_dir(&parent) {
                    Ok(()) => (),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => (),
                    Err(error) => return Err(error.into()),
                }
            }
        }
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                tracing::info!("Keeping existing template: {}", destination.display());
                continue;
            }
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        let mut staging = tempfile::NamedTempFile::new_in(
            destination.parent().context("Template has no parent")?,
        )?;
        staging.write_all(&template.contents)?;
        staging.as_file().set_permissions(template.permissions)?;
        staging
            .persist_noclobber(&destination)
            .with_context(|| format!("Cannot create {}", destination.display()))?;
        tracing::info!("Imported template: {}", destination.display());
    }
    println!(
        "Workspace root: {}\n\nTemplate directories:",
        root.display()
    );
    for (name, description) in templates {
        println!("  {}\n    {description}", paths.config.join(name).display());
    }
    println!(
        "\nNext steps:\n\
         1. Add language settings to {}. For example:\n\n\
         [language.cpp]\n\
         extensions = [\"cpp\"]\n\
         compile = \"g++ -std=c++23 -O2 -o {{binary}} {{input}}\"\n\
         run = \"{{binary}}\"\n\n\
         2. Put your starter solution in problem_template and shared files in workspace_template.\n\
         3. Export Netscape-format cookies from your browser, then import them:\n\
         \x20 cpcli login atcoder --cookie-file /path/to/cookies.txt\n\
         4. Download a problem or contest:\n\
         \x20 cpcli download <problem-url>\n\
         \x20 cpcli prepare <contest-url>\n\
         5. Enter the printed directory (or a problem directory within a contest), then run:\n\
         \x20 cpcli test ./solution.cpp\n\
         \x20 cpcli submit ./solution.cpp --language <language-id>\n\
         \x20 cpcli results --ui\n\n\
         Save submission language IDs in [language.<name>.submit], keyed by service.\n\
         Run cpcli submit without a configured language to list the available IDs.",
        config_path.display()
    );
    Ok(())
}

struct OjTemplate {
    path: PathBuf,
    contents: Vec<u8>,
    permissions: fs::Permissions,
}

fn read_oj_templates(config_path: &Path) -> Result<Vec<OjTemplate>> {
    #[derive(Deserialize)]
    struct OjConfig {
        templates: BTreeMap<PathBuf, PathBuf>,
    }
    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("Cannot read {}", config_path.display()))?;
    let config: OjConfig = toml::from_str(&contents).with_context(|| {
        format!(
            "Invalid online-judge-tools configuration: {}",
            config_path.display()
        )
    })?;
    let directory = config_path
        .parent()
        .context("Configuration has no parent directory")?;
    config
        .templates
        .into_iter()
        .map(|(path, source)| {
            ensure!(
                path.file_name().is_some()
                    && path
                        .components()
                        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir)),
                "Template destination must be a relative path without '..': {}",
                path.display()
            );
            let source = expand_path(&source)?;
            let source = if source.is_absolute() {
                source
            } else if source.components().count() == 1 {
                directory.join("template").join(source)
            } else {
                directory.join(source)
            };
            let metadata = fs::metadata(&source)
                .with_context(|| format!("Cannot read template {}", source.display()))?;
            ensure!(
                metadata.is_file(),
                "Template is not a file: {}",
                source.display()
            );
            Ok(OjTemplate {
                path,
                contents: fs::read(&source)
                    .with_context(|| format!("Cannot read template {}", source.display()))?,
                permissions: metadata.permissions(),
            })
        })
        .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub root: Option<PathBuf>,
    #[serde(default)]
    pub language: BTreeMap<String, Language>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Language {
    pub extensions: Vec<String>,
    pub preprocess: Option<String>,
    pub presubmit: Option<String>,
    pub compile: Option<String>,
    pub run: String,
    #[serde(default)]
    pub profile: BTreeMap<String, Profile>,
    #[serde(default)]
    pub submit: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub compile: Option<String>,
    pub run: Option<String>,
}

impl Config {
    pub fn load(paths: &Paths) -> Result<Self> {
        let path = paths.config.join("config.toml");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("Cannot read {}", path.display())),
        };
        toml::from_str(&text).with_context(|| format!("Invalid configuration: {}", path.display()))
    }

    pub fn root(&self) -> Result<PathBuf> {
        let root = self
            .root
            .as_ref()
            .context("Set root in config.toml before downloading or listing problems")?;
        ensure!(!root.as_os_str().is_empty(), "root must not be empty");
        Ok(std::path::absolute(expand_path(root)?)?)
    }

    pub fn language(&self, path: &Path) -> Result<&Language> {
        self.match_language(path)?
            .with_context(|| format!("No language configured for {}", path.display()))
    }

    pub fn match_language(&self, path: &Path) -> Result<Option<&Language>> {
        let Some(extension) = path.extension().and_then(|v| v.to_str()) else {
            return Ok(None);
        };
        let mut matches = self
            .language
            .values()
            .filter(|v| v.extensions.iter().any(|e| e == extension));
        let language = matches.next();
        ensure!(
            matches.next().is_none(),
            "Multiple languages are configured for .{extension}"
        );
        Ok(language)
    }
}
