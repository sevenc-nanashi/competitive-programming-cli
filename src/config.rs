use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
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
