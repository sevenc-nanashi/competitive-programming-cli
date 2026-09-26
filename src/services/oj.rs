use std::io::Write as _;

use super::ServiceBackend;
use crate::model::*;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OjProblem {
    url: Url,
    name: Option<String>,
    context: OjProblemContext,
    memory_limit: Option<u64>,
    time_limit: Option<u64>,
    tests: Vec<OjTestCase>,
    available_languages: Option<Vec<OjLanguage>>,
    raw: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct OjProblemContext {
    contest: Option<OjContestContext>,
    alphabet: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OjContestContext {
    url: Option<Url>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OjTestCase {
    name: Option<String>,
    input: String,
    output: String,
}

#[derive(Debug, Deserialize)]
struct OjLanguage {
    id: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct OjContest {
    url: Url,
    name: Option<String>,
    problems: Vec<OjContestProblem>,
    raw: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct OjContestProblem {
    url: Url,
    name: String,
    context: OjProblemContext,
}

#[derive(Debug, Deserialize)]
struct OjService {
    url: Url,
    name: String,
    contests: Option<Vec<OjServiceContest>>,
}

#[derive(Debug, Deserialize)]
struct OjServiceContest {
    url: Url,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OjLoginCheck {
    logged_in: bool,
}

#[derive(Debug, Deserialize)]
struct OjSubmission {
    url: Url,
}

#[derive(Debug, Deserialize)]
struct OjGuessedLanguage {
    #[serde(flatten)]
    language: OjLanguage,
    context: Option<OjLanguageContext>,
}

#[derive(Debug, Deserialize)]
struct OjLanguageContext {
    problem: Option<OjProblemLink>,
}

#[derive(Debug, Deserialize)]
struct OjProblemLink {
    url: Url,
}

#[derive(Debug, Deserialize)]
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
            // NOTE: online-judge-api-client 10.10.1 cannot run `oj-api login-service --check`, so use
            // latest commit instead of the latest release.
            .arg("--from=git+https://github.com/online-judge-tools/api-client@615c345")
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
                Ok(ResourceRef::Problem(ProblemRef {
                    service: ServiceId::Oj(url.host_str().context("URL has no host")?.to_string()),
                    id: problem.url.path().trim_start_matches('/').replace("/", "-"),
                    url: problem.url,
                    contest_id: problem.context.contest.and_then(|c| c.name),
                    internal_id: None,
                }))
            }
            (Err(_), Ok(contest)) => Ok(ResourceRef::Contest(ContestRef {
                service: ServiceId::Oj(url.host_str().context("URL has no host")?.to_string()),
                id: contest.url.path().trim_start_matches('/').to_string(),
                url: contest.url,
            })),
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
        todo!()
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        todo!()
    }

    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        todo!()
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        todo!()
    }

    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        todo!()
    }
}
