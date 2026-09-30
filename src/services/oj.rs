use std::io::Write as _;

use super::ServiceBackend;
use crate::model::*;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use url::Url;

static OJ_DEPENDENCY_URL: &str =
    "git+https://github.com/sevenc-nanashi/online-judge-tools-api-client@0b63972";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct OjProblem {
    problem_id: Option<String>,
    contest_id: Option<String>,
    url: Url,
    name: Option<String>,
    context: OjProblemContext,
    memory_limit: Option<u64>,
    time_limit: Option<u64>,
    tests: Vec<OjTestCase>,
    available_languages: Option<Vec<OjLanguage>>,
    raw: Option<serde_json::Value>,
}

impl From<OjProblem> for ProblemRef {
    fn from(problem: OjProblem) -> Self {
        ProblemRef {
            service: ServiceId::Oj(problem.url.host_str().expect("URL has no host").to_string()),
            id: problem
                .problem_id
                .or_else(|| {
                    problem
                        .context
                        .contest
                        .as_ref()
                        .and_then(|contest| contest.url.as_ref())
                        .and_then(|url| url.path_segments())
                        .and_then(|mut segments| segments.next_back())
                        .map(|s| s.to_string())
                })
                .expect("`oj-api` returned no problem_id"),
            url: problem.url,
            contest_id: problem.contest_id,
            internal_id: None,
        }
    }
}
impl From<OjProblem> for Problem {
    fn from(problem: OjProblem) -> Self {
        let reference = ProblemRef::from(problem.clone());
        let samples = problem
            .tests
            .into_iter()
            .map(|test| Sample {
                input: test.input,
                output: test.output,
            })
            .collect();
        Problem {
            reference,
            title: problem.name.unwrap_or_else(|| "<unknown>".to_string()),
            samples,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjProblemContext {
    contest: Option<OjContestContext>,
    alphabet: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjContestContext {
    url: Option<Url>,
    name: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjTestCase {
    name: Option<String>,
    input: String,
    output: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjLanguage {
    id: String,
    description: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjContest {
    contest_id: Option<String>,
    url: Url,
    name: Option<String>,
    problems: Vec<OjContestProblem>,
    raw: Option<serde_json::Value>,
}
impl From<OjContest> for ContestRef {
    fn from(contest: OjContest) -> Self {
        ContestRef {
            service: ServiceId::Oj(contest.url.host_str().expect("URL has no host").to_string()),
            id: contest
                .contest_id
                .or_else(|| {
                    contest
                        .url
                        .path_segments()
                        .and_then(|mut segments| segments.next_back())
                        .map(|s| s.to_string())
                })
                .expect("`oj-api` returned no contest_id"),
            url: contest.url,
        }
    }
}
impl From<OjContest> for Contest {
    fn from(contest: OjContest) -> Self {
        let reference = ContestRef::from(contest.clone());
        let problems = contest.problems.into_iter().map(ProblemRef::from).collect();
        Contest {
            reference,
            title: contest.name.unwrap_or_else(|| "<unknown>".to_string()),
            problems,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OjContestProblem {
    problem_id: Option<String>,
    contest_id: Option<String>,
    url: Url,
    name: String,
    context: OjProblemContext,
}
impl From<OjContestProblem> for ProblemRef {
    fn from(problem: OjContestProblem) -> Self {
        ProblemRef {
            service: ServiceId::Oj(problem.url.host_str().expect("URL has no host").to_string()),
            id: problem
                .problem_id
                .or_else(|| {
                    problem
                        .context
                        .contest
                        .as_ref()
                        .and_then(|contest| contest.url.as_ref())
                        .and_then(|url| url.path_segments())
                        .and_then(|mut segments| segments.next_back())
                        .map(|s| s.to_string())
                })
                .expect("`oj-api` returned no problem_id"),
            url: problem.url,
            contest_id: problem.contest_id,
            internal_id: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjService {
    url: Url,
    name: String,
    contests: Option<Vec<OjServiceContest>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjServiceContest {
    url: Url,
    name: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OjLoginCheck {
    logged_in: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjSubmission {
    url: Url,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjGuessedLanguage {
    #[serde(flatten)]
    language: OjLanguage,
    context: Option<OjLanguageContext>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjLanguageContext {
    problem: Option<OjProblemLink>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjProblemLink {
    url: Url,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct OjApiResponse<T> {
    status: String,
    messages: Vec<String>,
    result: T,
}

pub(super) struct OjBackend {
    pub cookie_dir: std::path::PathBuf,
    pub cookie_override: Option<Vec<u8>>,
}

impl OjBackend {
    fn call_oj_api<T: for<'de> Deserialize<'de>>(&self, host: &str, args: &[&str]) -> Result<T> {
        let cookie_bin = if let Some(cookie_override) = &self.cookie_override {
            Some(cookie_override.clone())
        } else {
            let cookie_txt_path = self.cookie_dir.join(format!(
                "{}.txt",
                crate::model::ServiceId::Oj(host.to_string()).id()
            ));
            cookie_txt_path
                .exists()
                .then(|| {
                    std::fs::read(&cookie_txt_path).with_context(|| {
                        format!("Failed to read cookie file: {}", cookie_txt_path.display())
                    })
                })
                .transpose()?
        };
        // TODO: Support non-uv environment
        let temp_cookie_base_path = tempfile::Builder::new()
            .prefix("cpg-oj-cookie")
            .disable_cleanup(true)
            .tempdir_in(&self.cookie_dir)
            .context("Failed to create temporary directory for cookie")?;
        let temp_cookie_netscape_path = if let Some(cookie_bin) = &cookie_bin {
            let temp_cookie_netscape_path =
                temp_cookie_base_path.path().join("cookie.netscape.txt");
            std::fs::write(&temp_cookie_netscape_path, cookie_bin)
                .context("Failed to write temporary cookie file")?;
            temp_cookie_netscape_path
        } else {
            "<none>".into()
        };
        let temp_cookie_lwp_path = temp_cookie_base_path.path().join("cookie.lwp.txt");
        tracing::debug!(
            "Converting cookie file from {} to {}",
            temp_cookie_netscape_path.display(),
            temp_cookie_lwp_path.display()
        );

        let convert_process = std::process::Command::new("uv")
            .arg("run")
            .arg("--script")
            .arg("-")
            .arg(temp_cookie_netscape_path.to_str().unwrap())
            .arg(temp_cookie_lwp_path.to_str().unwrap())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .spawn()
            .context("Failed to run `uv` to convert cookie file")?;
        let mut convert_stdin = convert_process.stdin.as_ref().unwrap();
        convert_stdin
            .write_all(include_bytes!("./mozilla_cookie_to_lwp_cookie.py"))
            .context("Failed to write to `uv` stdin")?;
        let convert_output = convert_process
            .wait_with_output()
            .context("Failed to wait for `uv` process")?;
        if !convert_output.status.success() {
            bail!(
                "`uv` failed to convert cookie file: {}",
                String::from_utf8_lossy(&convert_output.stderr)
            );
        }

        let process = std::process::Command::new("uvx")
            // NOTE: use my customized fork
            .arg(format!("--from={OJ_DEPENDENCY_URL}"))
            .arg("oj-api")
            .arg(concat!("--user-agent=cpg/", env!("CARGO_PKG_VERSION")))
            .args(["--cookie", temp_cookie_lwp_path.to_str().unwrap()])
            .args(args)
            .output()
            .context("Failed to run `oj-api`; Make sure `uvx` is installed")?;
        if !process.status.success() {
            bail!(
                "`oj-api` failed: {}",
                String::from_utf8_lossy(&process.stderr)
            );
        }
        let output = String::from_utf8(process.stdout).context("`oj-api` output is not UTF-8")?;
        let response: OjApiResponse<T> =
            serde_json::from_str(&output).context("Failed to parse `oj-api` output")?;
        if response.status != "ok" {
            bail!("`oj-api` returned error: {}", response.messages.join("\n"));
        }
        Ok(response.result)
    }
}
impl ServiceBackend for OjBackend {
    fn whoami(&self, service: &ServiceId) -> Result<(String, Url)> {
        tracing::warn!(
            "login for oj backend is not fully supported for oj+<host> services, and is not fully tested."
        );
        let host = match service {
            ServiceId::Oj(host) => host,
            _ => bail!("whoami is only supported for oj+<host> services"),
        };
        let output: OjLoginCheck =
            self.call_oj_api(host, &["login-service", "--check", &service.service_root()])?;
        if !output.logged_in {
            bail!("Not logged in to {}", service.id());
        }
        Ok((
            "<unknown but logged in>".into(),
            Url::parse(&format!("https://{}/", service.service_root()))?,
        ))
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        let as_problem = self.call_oj_api::<OjProblem>(
            url.host_str().context("URL has no host")?,
            &["get-problem", "--full", url.as_str()],
        );
        let as_contest = self.call_oj_api::<OjContest>(
            url.host_str().context("URL has no host")?,
            &["get-contest", "--full", url.as_str()],
        );
        match (as_problem, as_contest) {
            (Ok(problem), contest) => {
                if contest.is_ok() {
                    tracing::warn!("URL is both a problem and a contest; treating as problem");
                }
                Ok(ResourceRef::Problem(ProblemRef::from(problem)))
            }
            (Err(_), Ok(contest)) => Ok(ResourceRef::Contest(ContestRef::from(contest))),
            (Err(problem_err), Err(contest_err)) => {
                bail!(
                    "Failed to resolve URL as problem or contest: problem error: {}, contest error: {}",
                    problem_err,
                    contest_err
                );
            }
        }
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let oj_problem = self.call_oj_api::<OjProblem>(
            problem.url.host_str().context("Problem URL has no host")?,
            &["get-problem", "--full", problem.url.as_str()],
        )?;
        Ok(Problem::from(oj_problem))
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let oj_contest = self.call_oj_api::<OjContest>(
            contest.url.host_str().context("Contest URL has no host")?,
            &["get-contest", "--full", contest.url.as_str()],
        )?;
        Ok(Contest::from(oj_contest))
    }

    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        let oj_problem = self.call_oj_api::<OjProblem>(
            problem.url.host_str().context("Problem URL has no host")?,
            &["get-problem", "--full", problem.url.as_str()],
        )?;
        let languages = oj_problem
            .available_languages
            .unwrap_or_default()
            .into_iter()
            .map(|lang| SubmissionLanguage {
                id: lang.id,
                name: lang.description,
            })
            .collect::<Vec<_>>();
        if languages.is_empty() {
            bail!(
                "No available languages information for problem {}",
                problem.id
            );
        }
        Ok(languages)
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        let problem = &request.problem;
        let language = &request.language;
        let source_code = &request.source;
        let host = problem.url.host_str().context("Problem URL has no host")?;
        let temp_source_file = tempfile::Builder::new()
            .prefix("cpg-oj-source")
            .suffix(".txt")
            .tempfile()
            .context("Failed to create temporary source file")?;
        std::fs::write(temp_source_file.path(), source_code)
            .context("Failed to write source code to temporary file")?;
        let oj_submission = self.call_oj_api::<OjSubmission>(
            host,
            &[
                "submit",
                "--language",
                language,
                "--file",
                temp_source_file.path().to_str().unwrap(),
                problem.url.as_str(),
            ],
        )?;
        let problem = self.fetch_problem(problem)?;
        Ok(Submission {
            id: oj_submission.url.to_string(),
            url: oj_submission.url,
            problem_id: problem.reference.id,
            submitted_at: chrono::Utc::now().to_rfc3339(),
            language: language.to_string(),
            status: "??".to_string(),
            time: "??".to_string(),
        })
    }

    fn submissions(&self, _scope: &SubmissionScope, _limit: usize) -> Result<Vec<Submission>> {
        bail!("Fetching submissions is not supported for oj backend");
    }
}
