use super::{Http, ServiceBackend, pre_text, required, selector, sort_submissions, text};
use crate::model::*;
use anyhow::{Context, Result, bail, ensure};
use scraper::Html;
use serde::Deserialize;
use url::Url;

pub(super) struct ShareOjBackend {
    pub http: Http,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProblem {
    title: String,
    has_samples: bool,
}

#[derive(Deserialize)]
struct ApiContest {
    title: String,
    problems: Vec<ApiId>,
}

#[derive(Deserialize)]
struct ApiId {
    id: String,
}

#[derive(Deserialize)]
struct Profile {
    handle: String,
}

#[derive(Deserialize)]
struct Account {
    user: Option<ApiId>,
}

#[derive(Deserialize)]
struct ProfileResponse {
    profile: Option<Profile>,
}

#[derive(Deserialize)]
struct Runtime {
    id: String,
    label: String,
}

#[derive(Deserialize)]
struct Runtimes {
    items: Vec<Runtime>,
    maintenance: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiSubmission {
    id: String,
    problem_id: String,
    created_at: String,
    runtime: String,
    status: String,
    easy_test: bool,
    result: Option<Judgement>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Judgement {
    verdict: String,
    cpu_time_ms: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionPage {
    items: Vec<ApiSubmission>,
    has_more: bool,
}

fn url(path: &str) -> Result<Url> {
    Ok(Url::parse("https://www.share-oj.net/")?.join(path)?)
}

fn api_url(problem: &ProblemRef) -> Result<Url> {
    url(&format!("api{}", problem.url.path()))
}

impl ShareOjBackend {
    fn problem_submissions(&self, problem: &ProblemRef, limit: usize) -> Result<Vec<Submission>> {
        let mut submissions = Vec::new();
        let mut offset = 0;
        while submissions.len() < limit {
            let mut endpoint = url(&format!("api{}/submissions", problem.url.path()))?;
            endpoint
                .query_pairs_mut()
                .append_pair("mine", "1")
                .append_pair("offset", &offset.to_string());
            let page: SubmissionPage = self.http.json(&endpoint)?;
            for item in page.items {
                if !item.easy_test {
                    submissions.push(item.into_submission()?);
                }
            }
            if !page.has_more {
                break;
            }
            offset += 50;
        }
        sort_submissions(&mut submissions, limit);
        Ok(submissions)
    }
}

impl ServiceBackend for ShareOjBackend {
    fn whoami(&self, _service: &ServiceId) -> Result<(String, Option<Url>)> {
        let account: Account = self.http.json(&url("api/auth/me")?)?;
        ensure!(
            account.user.is_some(),
            "ShareOJ session expired; import fresh cookies with cpg login share-oj"
        );
        let response: ProfileResponse = self.http.json(&url("api/my/profile")?)?;
        let profile = response.profile.context("Register your ShareOJ profile at https://www.share-oj.net/my/settings before using cpg login share-oj")?;
        let profile_url = url(&format!("users/{}", profile.handle))?;
        Ok((profile.handle, Some(profile_url)))
    }

    fn resolve_url(&self, input: &Url) -> Result<ResourceRef> {
        ensure!(
            ServiceId::from_url(input)? == ServiceId::ShareOj,
            "Expected a ShareOJ URL"
        );
        let parts: Vec<_> = input.path().trim_matches('/').split('/').collect();
        let (id, contest_id) = match parts.as_slice() {
            ["problems", id] => (*id, None),
            ["contests", id] => {
                validate_id(id)?;
                return Ok(ResourceRef::Contest(ContestRef {
                    service: ServiceId::ShareOj,
                    id: (*id).into(),
                    url: url(&format!("contests/{id}"))?,
                }));
            }
            ["contests", contest, "problems", id] => {
                validate_id(contest)?;
                (*id, Some((*contest).to_owned()))
            }
            _ => bail!("Unsupported ShareOJ URL: {input}; expected a problem or contest URL"),
        };
        validate_id(id)?;
        Ok(ResourceRef::Problem(ProblemRef {
            service: ServiceId::ShareOj,
            id: id.into(),
            url: url(&parts.join("/"))?,
            contest_id,
            internal_id: None,
        }))
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let data: ApiProblem = self.http.json(&api_url(problem)?)?;
        let (_, document) = self.http.get(&problem.url)?;
        let samples = samples(&document)?;
        if data.has_samples && samples.is_empty() {
            tracing::warn!(
                "ShareOJ samples could not be extracted from this problem's statement; add samples manually"
            );
        }
        Ok(Problem {
            reference: problem.clone(),
            title: data.title,
            samples,
        })
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let data: ApiContest = self
            .http
            .json(&url(&format!("api/contests/{}", contest.id))?)?;
        ensure!(
            !data.problems.is_empty() && data.problems.iter().all(|p| !p.id.is_empty()),
            "ShareOJ contest problems are not available yet; wait until publication or check your access in the browser"
        );
        let problems = data
            .problems
            .into_iter()
            .map(|p| {
                self.resolve_url(&url(&format!("contests/{}/problems/{}", contest.id, p.id))?)?
                    .problem()
            })
            .collect::<Result<_>>()?;
        Ok(Contest {
            reference: contest.clone(),
            title: data.title,
            problems,
        })
    }

    fn languages(&self, _problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        let data: Runtimes = self.http.json(&url("api/runtimes")?)?;
        ensure!(
            !data.maintenance,
            "ShareOJ judging is under maintenance; submissions are unavailable"
        );
        Ok(data
            .items
            .into_iter()
            .map(|r| SubmissionLanguage {
                id: r.id,
                name: r.label,
            })
            .collect())
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        ensure!(
            !request.source.trim().is_empty() && request.source.len() <= 65536,
            "ShareOJ source must contain non-whitespace text and be at most 65,536 bytes"
        );
        let mut body = serde_json::json!({
            "problemId": request.problem.id,
            "runtime": request.language,
            "source": request.source,
        });
        if let Some(contest) = &request.problem.contest_id {
            body["contestId"] = contest.clone().into();
        }
        let response: ApiId = self.http.post_json(&url("api/my/submissions")?, &body)?;
        ensure!(
            !response.id.is_empty(),
            "ShareOJ did not confirm submission; check results before submitting again"
        );
        Ok(Submission {
            url: url(&format!("my/submissions/{}", response.id))?,
            id: response.id,
            problem_id: request.problem.id.clone(),
            submitted_at: String::new(),
            language: request.language.into(),
            status: "WJ".into(),
            time: String::new(),
        })
    }

    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        self.whoami(&ServiceId::ShareOj)?;
        match scope {
            Metadata::Problem { reference, .. } => self.problem_submissions(reference, limit),
            Metadata::Contest(contest) => {
                // ponytail: one paginated fetch per problem; use a contest-wide mine API if one becomes available.
                let mut submissions = Vec::new();
                for problem in &contest.problems {
                    submissions.extend(self.problem_submissions(problem, limit)?);
                }
                sort_submissions(&mut submissions, limit);
                Ok(submissions)
            }
        }
    }
}

fn validate_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "Invalid ShareOJ resource ID"
    );
    Ok(())
}

impl ApiSubmission {
    fn into_submission(self) -> Result<Submission> {
        let (status, time) = if self.status == "DONE" {
            let result = self
                .result
                .context("ShareOJ completed submission has no result")?;
            let time = match result.cpu_time_ms {
                Some(ms) => format!("{} ms", ms.ceil()),
                None => String::new(),
            };
            (result.verdict, time)
        } else {
            ("WJ".into(), String::new())
        };
        Ok(Submission {
            url: url(&format!("my/submissions/{}", self.id))?,
            id: self.id,
            problem_id: self.problem_id,
            submitted_at: self.created_at,
            language: self.runtime,
            status,
            time,
        })
    }
}

fn samples(document: &Html) -> Result<Vec<Sample>> {
    let body = required(document, ".problem-body .markdown-body")?;
    let mut pairs: Vec<(u32, Option<String>, Option<String>)> = Vec::new();
    let mut section: Option<(u32, u32)> = None;
    let mut pending = None;
    for element in body.select(&selector("h1, h2, h3, h4, h5, h6, pre")) {
        if element.value().name() == "pre" {
            if let Some((id, input)) = pending.take() {
                let index = match pairs.iter().position(|p| p.0 == id) {
                    Some(index) => index,
                    None => {
                        pairs.push((id, None, None));
                        pairs.len() - 1
                    }
                };
                let slot = if input {
                    &mut pairs[index].1
                } else {
                    &mut pairs[index].2
                };
                ensure!(slot.is_none(), "Duplicate ShareOJ sample {id}");
                *slot = Some(pre_text(element));
            }
            continue;
        }
        ensure!(
            pending.is_none(),
            "ShareOJ sample heading has no code block"
        );
        let level: u32 = element.value().name()[1..].parse()?;
        if section.is_some_and(|(parent, _)| level <= parent) {
            section = None;
        }
        let heading = text(element);
        if let Some(number) = heading.strip_prefix("サンプル") {
            let id = number
                .trim()
                .parse()
                .context("Invalid ShareOJ sample number")?;
            ensure!(
                !pairs.iter().any(|pair| pair.0 == id),
                "Duplicate ShareOJ sample {id}"
            );
            pairs.push((id, None, None));
            section = Some((level, id));
        } else if let Some(number) = heading.strip_prefix("入力例") {
            pending = Some((number.trim().parse::<u32>()?, true));
        } else if let Some(number) = heading.strip_prefix("出力例") {
            pending = Some((number.trim().parse::<u32>()?, false));
        } else if let Some((_, id)) = section {
            match heading.as_str() {
                "入力" => pending = Some((id, true)),
                "出力" => pending = Some((id, false)),
                _ => (),
            }
        }
    }
    ensure!(
        pending.is_none(),
        "ShareOJ sample heading has no code block"
    );
    pairs
        .into_iter()
        .map(|(id, input, output)| {
            Ok(Sample {
                input: input.with_context(|| format!("ShareOJ sample {id} has no input"))?,
                output: output.with_context(|| format!("ShareOJ sample {id} has no output"))?,
            })
        })
        .collect()
}

pub(super) fn api_error(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    let detail = serde_json::from_str::<serde_json::Value>(body).ok();
    let code = detail
        .as_ref()
        .and_then(|v| v.pointer("/data/code"))
        .and_then(|v| v.as_str());
    let message = match code {
        Some("judge_maintenance") => "Judging is under maintenance",
        Some("submission_rate_limited") => {
            "Submission rate limit reached; wait before trying again"
        }
        Some("profile_required") => "Register your profile at https://www.share-oj.net/my/settings",
        Some("contest_participation_required") => {
            "Join the contest in your browser before submitting"
        }
        Some("tests_not_ready" | "judging_unavailable") => {
            "The problem's judge or test cases are not ready"
        }
        _ if status == reqwest::StatusCode::UNAUTHORIZED => {
            "Session expired; import fresh cookies with cpg login share-oj"
        }
        Some(code) => code,
        None => "Request failed; check access and problem availability in your browser",
    };
    anyhow::anyhow!(
        "ShareOJ ({status}): {message}. If submitting, check results before submitting again"
    )
}
