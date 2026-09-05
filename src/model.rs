use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, usage::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceId {
    Atcoder,
    AtcoderProblems,
    Yukicoder,
    #[cfg(feature = "mock")]
    Mock,
}

impl ServiceId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Atcoder => "atcoder",
            Self::AtcoderProblems => "atcoder-problems",
            Self::Yukicoder => "yukicoder",
            #[cfg(feature = "mock")]
            Self::Mock => "mock",
        }
    }

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
            _ => bail!("Unsupported judge URL: {url}"),
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
    pub fn contest(self) -> Result<ContestRef> {
        match self {
            Self::Contest(c) => Ok(c),
            _ => bail!("Expected a contest URL; use download for a problem"),
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
            Self::Problem { reference, .. } => reference.service,
            Self::Contest(c) => c.reference.service,
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
