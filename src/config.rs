use anyhow::{Context, Result, ensure};
use console::Style;
use schemars::JsonSchema;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, ErrorKind, Write},
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const SCHEMA_URL: &str = concat!(
    "https://raw.githubusercontent.com/sevenc-nanashi/competitive-programming-cli/refs/tags/v",
    env!("CARGO_PKG_VERSION"),
    "/docs/public/config.schema.json"
);

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
            config: directory("CPG_CONFIG_HOME", "XDG_CONFIG_HOME", ".config", "cpg")?,
            cookies: directory(
                "CPG_COOKIES_HOME",
                "XDG_DATA_HOME",
                ".local/share",
                "cpg/cookies",
            )?,
        })
    }
}

fn prompt(message: &str, interrupted: &AtomicBool) -> Result<String> {
    eprint!(
        "{}",
        Style::new()
            .for_stderr()
            .blue()
            .bright()
            .bold()
            .apply_to(message)
    );
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
        let input = prompt("Workspace root [~/cpg]: ", interrupted)?;
        let root = match input.as_str() {
            "" => "~/cpg",
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
        writeln!(staging, "#:schema {SCHEMA_URL}")?;
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
    let heading = Style::new().blue().bright().bold();
    println!(
        "{} {}\n\n{}",
        heading.apply_to("Workspace root:"),
        root.display(),
        heading.apply_to("Template directories:")
    );
    for (name, description) in templates {
        println!("  {}\n    {description}", paths.config.join(name).display());
    }
    println!(
        "\n{}\n\
         1. Add language settings to {}. For example:\n\n\
         [language.cpp]\n\
         extensions = [\"cpp\"]\n\
         compile = \"g++ -std=c++23 -O2 -o {{binary}} {{input}}\"\n\
         run = \"{{binary}}\"\n\n\
         2. Put your starter solution in problem_template and shared files in workspace_template.\n\
         3. Export Netscape-format cookies from your browser, then import them:\n\
         \x20 cpg login atcoder --cookie-file /path/to/cookies.txt\n\
         4. Download a problem or contest:\n\
         \x20 cpg download <problem-url>\n\
         \x20 cpg prepare <contest-url>\n\
         5. Enter the printed directory (or a problem directory within a contest), then run:\n\
         \x20 cpg test ./solution.cpp\n\
         \x20 cpg submit ./solution.cpp --language <language-id>\n\
         \x20 cpg results --ui\n\n\
         Save submission language IDs in [language.<name>.submit], keyed by service.\n\
         Run cpg submit without a configured language to list the available IDs.",
        heading.apply_to("Next steps:"),
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

/// Configuration for cpg (config.toml).
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "cpg configuration", extend("$id" = SCHEMA_URL))]
pub struct Config {
    /// Workspace root. Expands a leading ~ to HOME; relative paths resolve from the current directory.
    #[schemars(length(min = 1))]
    pub root: Option<PathBuf>,
    /// Shell commands to run after copying each template.
    #[serde(default)]
    pub setup: Setup,
    /// Clipboard backend. Defaults to arboard; command sends the text to a shell command's stdin.
    #[serde(default)]
    pub clipboard: Clipboard,
    /// Languages keyed by name. The executable language customizes the executable-file fallback.
    #[serde(default)]
    pub language: BTreeMap<String, Language>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Clipboard {
    /// Use the system clipboard through arboard.
    Arboard {},
    /// Pipe the text to a shell command.
    Command {
        /// Shell command receiving UTF-8 text on stdin, without an added newline.
        #[schemars(length(min = 1))]
        command: String,
    },
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::Arboard {}
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Setup {
    /// Run after copying the workspace template, in the standalone problem or contest root.
    #[serde(deserialize_with = "setup_commands")]
    #[schemars(with = "SetupCommands")]
    pub workspace: Vec<String>,
    /// Run after copying the problem template, in each problem directory.
    #[serde(deserialize_with = "setup_commands")]
    #[schemars(with = "SetupCommands")]
    pub problem: Vec<String>,
    /// Run after copying the contest template, in the contest root.
    #[serde(deserialize_with = "setup_commands")]
    #[schemars(with = "SetupCommands")]
    pub contest: Vec<String>,
    /// Run after copying the single problem template, in the standalone problem root.
    #[serde(deserialize_with = "setup_commands")]
    #[schemars(with = "SetupCommands")]
    pub single_problem: Vec<String>,
}

/// One shell command or multiple commands run in order in separate shells. Use [] to run none.
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
enum SetupCommands {
    Single(String),
    Multiple(Vec<String>),
}

fn setup_commands<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Ok(match SetupCommands::deserialize(deserializer)? {
        SetupCommands::Single(command) => vec![command],
        SetupCommands::Multiple(commands) => commands,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Language {
    /// File extensions without the leading dot. Use [] for the executable fallback.
    pub extensions: Vec<String>,
    /// Shell command before compilation, execution, or submission. Receives source on stdin and via {input}; writes transformed UTF-8 source to {processed} when present, otherwise stdout. Paths are shell-quoted.
    pub preprocess: Option<String>,
    /// Shell command before submission, after preprocess. Receives source on stdin and via {input}; writes transformed UTF-8 source to {processed} when present, otherwise stdout. Paths are shell-quoted.
    pub presubmit: Option<String>,
    /// Compilation shell command with shell-quoted {input} and {binary} paths. Omit for interpreted languages.
    pub compile: Option<String>,
    /// Execution shell command with shell-quoted {input} and {binary} paths.
    pub run: String,
    /// Named compile/run overrides selected with --profile. Omitted commands inherit language settings.
    #[serde(default)]
    pub profile: BTreeMap<String, Profile>,
    /// Submission language IDs keyed by service (atcoder or yukicoder). AtCoder Problems uses atcoder. IDs must be strings.
    #[serde(default)]
    pub submit: BTreeMap<String, String>,
}

static EXECUTABLE: LazyLock<Language> = LazyLock::new(|| Language {
    extensions: Vec::new(),
    preprocess: None,
    presubmit: None,
    compile: None,
    run: "{input}".into(),
    profile: BTreeMap::new(),
    submit: BTreeMap::new(),
});

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Override the compilation shell command. Supports {input} and {binary}.
    pub compile: Option<String>,
    /// Override the execution shell command. Supports {input} and {binary}.
    pub run: Option<String>,
}

const SCHEMA_PATTERN: &str = "#:schema https://raw.githubusercontent.com/sevenc-nanashi/competitive-programming-cli/refs/tags/v";

impl Config {
    pub fn schema() -> schemars::Schema {
        schemars::generate::SchemaSettings::draft07()
            .into_generator()
            .into_root_schema_for::<Self>()
    }

    pub fn load(paths: &Paths) -> Result<Self> {
        let path = paths.config.join("config.toml");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("Cannot read {}", path.display())),
        };
        if text.contains(SCHEMA_PATTERN) && !text.contains(env!("CARGO_PKG_VERSION")) {
            tracing::warn!("Configuration schema version does not match the current version.");
            tracing::warn!(
                "Consider updating your configuration to match the current schema: {}",
                SCHEMA_URL
            );
        }

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
        if let Some(extension) = path.extension().and_then(|v| v.to_str()) {
            let mut matches = self
                .language
                .values()
                .filter(|v| v.extensions.iter().any(|e| e == extension));
            let language = matches.next();
            ensure!(
                matches.next().is_none(),
                "Multiple languages are configured for .{extension}"
            );
            if language.is_some() {
                return Ok(language);
            }
        }
        let metadata = fs::metadata(path)?;
        if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
            Ok(Some(self.language.get("executable").unwrap_or(&EXECUTABLE)))
        } else {
            Ok(None)
        }
    }
}
