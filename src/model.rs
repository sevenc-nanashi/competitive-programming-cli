use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceId {
    Atcoder,
    AtcoderProblems,
    Yukicoder,
    Oj(String),
    #[cfg(feature = "mock")]
    Mock,
}
impl std::fmt::Display for ServiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Atcoder => write!(f, "atcoder"),
            Self::AtcoderProblems => write!(f, "atcoder-problems"),
            Self::Yukicoder => write!(f, "yukicoder"),
            Self::Oj(host) => write!(f, "oj+{host}"),
            #[cfg(feature = "mock")]
            Self::Mock => write!(f, "mock"),
        }
    }
}
impl serde::Serialize for ServiceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl std::str::FromStr for ServiceId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "atcoder" => Ok(Self::Atcoder),
            "atcoder-problems" => Ok(Self::AtcoderProblems),
            "yukicoder" => Ok(Self::Yukicoder),
            #[cfg(feature = "mock")]
            "mock" => Ok(Self::Mock),
            _ if s.starts_with("oj+") => Ok(Self::Oj(s[3..].to_string())),
            _ => bail!("Unknown service ID: {s}"),
        }
    }
}
impl<'de> serde::Deserialize<'de> for ServiceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

pub static SERVICE_ID_COMPLETIONS: &[(&str, &str)] = &[
    ("atcoder", "AtCoder"),
    ("atcoder-problems", "AtCoder Problems"),
    ("yukicoder", "Yukicoder"),
    ("oj+", "Other Online Judges via `oj`"),
    #[cfg(feature = "mock")]
    ("mock", "Mock"),
];
impl ServiceId {
    pub fn from_url(url: &Url) -> Result<Self> {
        anyhow::ensure!(
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none(),
            "Expected an HTTPS judge URL without credentials or a custom port"
        );
        match url.host_str() {
            Some("atcoder.jp") => Ok(Self::Atcoder),
            Some("kenkoooo.com") => Ok(Self::AtcoderProblems),
            Some("yukicoder.me") => Ok(Self::Yukicoder),
            #[cfg(feature = "mock")]
            Some("mock.local") => Ok(Self::Mock),
            // _ => bail!("Unsupported judge URL: {url}"),
            // TODO: Add list of `oj` services?
            Some(host) => Ok(Self::Oj(host.to_string())),
            None => bail!("Unsupported judge URL: {url}"),
        }
    }

    pub fn service_root(&self) -> String {
        match self {
            Self::Atcoder => "https://atcoder.jp".to_string(),
            Self::AtcoderProblems => "https://kenkoooo.com/atcoder".to_string(),
            Self::Yukicoder => "https://yukicoder.me".to_string(),
            Self::Oj(host) => format!("https://{host}"),
            #[cfg(feature = "mock")]
            Self::Mock => "https://mock.local".to_string(),
        }
    }

    pub fn id(&self) -> String {
        match self {
            Self::Atcoder => "atcoder".to_string(),
            Self::AtcoderProblems => "atcoder-problems".to_string(),
            Self::Yukicoder => "yukicoder".to_string(),
            Self::Oj(host) => format!("oj+{}", host.replace(".", "--").replace("/", "---")),
            #[cfg(feature = "mock")]
            Self::Mock => "mock".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemRef {
    pub service: ServiceId,
    pub id: String,
    pub url: Url,
    pub contest_id: Option<String>,
    pub internal_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestRef {
    pub service: ServiceId,
    pub id: String,
    pub url: Url,
}

#[derive(Debug, Clone)]
pub enum ResourceRef {
    Problem(ProblemRef),
    Contest(ContestRef),
}

impl ResourceRef {
    pub fn problem(self) -> Result<ProblemRef> {
        match self {
            Self::Problem(p) => Ok(p),
            _ => bail!("Expected a problem URL; use prepare for a contest"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub reference: ProblemRef,
    pub title: String,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contest {
    #[serde(flatten)]
    pub reference: ContestRef,
    pub title: String,
    pub problems: Vec<ProblemRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Metadata {
    Problem {
        #[serde(flatten)]
        reference: ProblemRef,
        title: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        template_checksums: BTreeMap<PathBuf, String>,
    },
    Contest(Contest),
}

impl Metadata {
    pub fn service(&self) -> ServiceId {
        match self {
            Self::Problem { reference, .. } => reference.service.clone(),
            Self::Contest(c) => c.reference.service.clone(),
        }
    }
    pub fn is_contest(&self) -> bool {
        matches!(self, Self::Contest(_))
    }
}

pub type SubmissionScope = Metadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionLanguage {
    pub id: String,
    pub name: String,
}

pub struct SubmissionRequest<'a> {
    pub problem: &'a ProblemRef,
    pub language: &'a str,
    pub source: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: String,
    pub url: Url,
    pub problem_id: String,
    pub submitted_at: String,
    pub language: String,
    pub status: String,
    pub time: String,
}
