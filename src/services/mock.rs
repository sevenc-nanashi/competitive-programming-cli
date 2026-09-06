use super::{ServiceBackend, sort_submissions};
use crate::{config::expand_path, model::*, workspace::safe_id};
use anyhow::{Context, Result, bail, ensure};
use reqwest::cookie::{CookieStore, Jar};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

pub(super) struct MockBackend {
    pub cookies: Arc<Jar>,
}

#[derive(Deserialize)]
struct Settings {
    user: String,
    session: String,
    languages: Vec<SubmissionLanguage>,
}
#[derive(Deserialize)]
struct ProblemData {
    title: String,
}
#[derive(Deserialize)]
struct ContestData {
    title: String,
    problems: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct StoredSubmission {
    user: String,
    source: String,
    #[serde(flatten)]
    submission: Submission,
}

fn root() -> Result<PathBuf> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .context("mock requires CARGO_MANIFEST_DIR (set automatically by cargo run/test)")?;
    let manifest = expand_path(PathBuf::from(manifest))?;
    ensure!(
        manifest.is_absolute(),
        "CARGO_MANIFEST_DIR must be absolute"
    );
    Ok(manifest.join("mock_service"))
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    toml::from_str(
        &fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?,
    )
    .with_context(|| format!("Invalid mock data: {}", path.display()))
}

impl MockBackend {
    fn authenticated(&self) -> Result<Settings> {
        let settings: Settings = read(&root()?.join("service.toml"))?;
        let cookies = self
            .cookies
            .cookies(&Url::parse("https://mock.local/")?)
            .context("Mock session missing; run cpg login mock")?;
        let expected = format!("session={}", settings.session);
        ensure!(
            cookies
                .to_str()?
                .split(';')
                .any(|cookie| cookie.trim() == expected),
            "Mock session expired; import fresh cookies"
        );
        Ok(settings)
    }

    fn problem(id: &str) -> Result<ProblemRef> {
        safe_id(id)?;
        Ok(ProblemRef {
            service: ServiceId::Mock,
            id: id.into(),
            url: Url::parse(&format!("https://mock.local/problems/{id}"))?,
            contest_id: None,
            internal_id: None,
        })
    }
}

impl ServiceBackend for MockBackend {
    fn service(&self) -> ServiceId {
        ServiceId::Mock
    }
    fn auth_service(&self) -> ServiceId {
        ServiceId::Mock
    }
    fn check_auth(&self) -> Result<()> {
        self.authenticated().map(|_| ())
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        ensure!(
            ServiceId::from_url(url)? == self.service(),
            "Expected a mock URL"
        );
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        match parts.as_slice() {
            ["problems", id] => Ok(ResourceRef::Problem(Self::problem(id)?)),
            ["contests", id] => {
                safe_id(id)?;
                Ok(ResourceRef::Contest(ContestRef {
                    service: self.service(),
                    id: (*id).into(),
                    url: url.clone(),
                }))
            }
            _ => bail!("Expected https://mock.local/problems/<id> or /contests/<id>"),
        }
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let directory = root()?.join("problems").join(safe_id(&problem.id)?);
        let data: ProblemData = read(&directory.join("problem.toml"))?;
        let mut inputs = Vec::new();
        for entry in fs::read_dir(directory.join("test"))? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry.path().extension().is_some_and(|ext| ext == "in")
            {
                inputs.push(entry.path());
            }
        }
        inputs.sort();
        let samples = inputs
            .into_iter()
            .map(|input| {
                Ok(Sample {
                    output: fs::read_to_string(input.with_extension("out"))?,
                    input: fs::read_to_string(input)?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Problem {
            reference: problem.clone(),
            title: data.title,
            samples,
        })
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let data: ContestData = read(
            &root()?
                .join("contests")
                .join(safe_id(&contest.id)?)
                .join("contest.toml"),
        )?;
        let problems = data
            .problems
            .iter()
            .map(|id| {
                let mut problem = Self::problem(id)?;
                problem.contest_id = Some(contest.id.clone());
                Ok(problem)
            })
            .collect::<Result<_>>()?;
        Ok(Contest {
            reference: contest.clone(),
            title: data.title,
            problems,
        })
    }

    fn languages(&self, _problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        Ok(read::<Settings>(&root()?.join("service.toml"))?.languages)
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        let settings = self.authenticated()?;
        self.fetch_problem(request.problem)?;
        let language = settings
            .languages
            .iter()
            .find(|language| language.id == request.language)
            .context("Unknown mock language")?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let id = now.as_nanos().to_string();
        let submission = Submission {
            url: Url::parse(&format!("https://mock.local/submissions/{id}"))?,
            id,
            problem_id: request.problem.id.clone(),
            submitted_at: format!("{:020}", now.as_secs()),
            language: language.name.clone(),
            status: "WJ".into(),
            time: String::new(),
        };
        let directory = root()?.join("submissions");
        fs::create_dir_all(&directory)?;
        let stored = StoredSubmission {
            user: settings.user,
            source: request.source.into(),
            submission: submission.clone(),
        };
        let mut staging = tempfile::Builder::new()
            .prefix(".cpg-")
            .tempfile_in(&directory)?;
        staging.write_all(toml::to_string_pretty(&stored)?.as_bytes())?;
        staging.persist_noclobber(directory.join(format!("{}.toml", submission.id)))?;
        Ok(submission)
    }

    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        let settings = self.authenticated()?;
        let directory = root()?.join("submissions");
        if !directory.try_exists()? {
            return Ok(Vec::new());
        }
        let mut submissions = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || entry.path().extension().is_none_or(|ext| ext != "toml")
            {
                continue;
            }
            let stored: StoredSubmission = read(&entry.path())?;
            let matching = match scope {
                Metadata::Problem { reference, .. } => reference.id == stored.submission.problem_id,
                Metadata::Contest(contest) => contest
                    .problems
                    .iter()
                    .any(|p| p.id == stored.submission.problem_id),
            };
            if stored.user == settings.user && matching {
                submissions.push(stored.submission);
            }
        }
        sort_submissions(&mut submissions, limit);
        Ok(submissions)
    }
}
