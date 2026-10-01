use super::{Http, ServiceBackend, form_action, form_fields, pre_text, required, selector, text};
use crate::model::*;
use anyhow::{Context, Result, bail, ensure};
use scraper::{ElementRef, Html};
use url::Url;

pub(super) struct CodeforcesBackend {
    pub http: Http,
}

impl CodeforcesBackend {
    fn get(&self, url: &Url) -> Result<(Url, Html)> {
        let mut url = url.clone();
        url.query_pairs_mut().append_pair("locale", "en");
        let (url, document) = self.http.get(&url)?;
        check_page(&document)?;
        Ok((url, document))
    }

    fn submission_page(&self, problem: &ProblemRef) -> Result<(Url, Html)> {
        self.get(&contest_url(&problem.url)?.join("submit")?)
    }
}

impl ServiceBackend for CodeforcesBackend {
    fn whoami(&self, _service: &ServiceId) -> Result<(String, Option<Url>)> {
        let (url, _) = self.get(&Url::parse("https://codeforces.com/profile")?)?;
        let user = url
            .path()
            .strip_prefix("/profile/")
            .filter(|user| !user.is_empty() && !user.contains('/'))
            .context(
                "Codeforces session expired; import fresh cookies with cpg login codeforces",
            )?;
        let profile = Url::parse("https://codeforces.com/")?.join(&format!("profile/{user}"))?;
        Ok((user.to_owned(), Some(profile)))
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        resolve_url(url)
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let (page, document) = self.get(&problem.url)?;
        let contest = contest_url(&problem.url)?;
        // Some Gym problems provide statements only as contest attachments, not HTML samples.
        if page.path() == contest.join("attachments")?.path() {
            let (_, dashboard) = self.get(&contest)?;
            for link in dashboard.select(&selector("table.problems td:not(.id) a[href]")) {
                let url = contest.join(link.value().attr("href").expect("selected href"))?;
                if url.path() == problem.url.path() && url.origin() == problem.url.origin() {
                    tracing::warn!(
                        "Codeforces provides this statement as an attachment at {page}; add samples manually"
                    );
                    return Ok(Problem {
                        reference: problem.clone(),
                        title: text(link),
                        samples: Vec::new(),
                    });
                }
            }
            bail!("Codeforces attachment problem is missing from its contest dashboard");
        }
        Ok(Problem {
            reference: problem.clone(),
            title: text(required(&document, ".problem-statement .header .title")?),
            samples: samples(&document)?,
        })
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let (url, document) = self.get(&contest.url)?;
        parse_contest(&document, &url, contest)
    }

    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        let (_, document) = self.submission_page(problem)?;
        languages(submission_form(&document)?)
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        let scope = Metadata::Problem {
            reference: request.problem.clone(),
            title: String::new(),
            template_checksums: Default::default(),
        };
        let before = self
            .submissions(&scope, 1)?
            .first()
            .map(|s| s.id.parse::<u64>())
            .transpose()?;
        let (page, document) = self.submission_page(request.problem)?;
        let (action, fields) = submission_fields(&document, &page, request)?;
        let (url, document) = self
            .http
            .post(&action, fields, None, None)
            .context("Submission outcome is unknown; check results before submitting again")?;
        check_page(&document)?;
        let errors = document
            .select(&selector(".error"))
            .map(text)
            .filter(|message| !message.is_empty())
            .collect::<Vec<_>>();
        ensure!(
            errors.is_empty(),
            "Codeforces rejected submission: {}",
            errors.join("; ")
        );
        let my = contest_url(&request.problem.url)?.join("my")?;
        ensure!(
            url.path() == my.path(),
            "Codeforces did not confirm submission; check results before submitting again. Check contest registration or CAPTCHA in your browser; use submit --clipboard --open for manual submission"
        );
        let submissions = parse_submissions(&document, &url)?;
        identify_submission(submissions, request.problem, before)
    }

    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        let (url, problem) = match scope {
            Metadata::Problem { reference, .. } => (&reference.url, Some(reference.id.as_str())),
            Metadata::Contest(contest) => (&contest.reference.url, None),
        };
        let base = contest_url(url)?.join("my")?;
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for page in 1.. {
            let mut url = if page == 1 {
                base.clone()
            } else {
                Url::parse(&format!("{base}/page/{page}"))?
            };
            url.query_pairs_mut()
                .append_pair("order", "BY_ARRIVED_DESC");
            let (actual_url, document) = self.get(&url)?;
            ensure!(
                actual_url.path() == url.path(),
                "Codeforces submissions unavailable; check your cookies and contest registration"
            );
            results.extend(
                parse_submissions(&document, &url)?
                    .into_iter()
                    .filter(|submission| problem.is_none_or(|id| submission.problem_id == id))
                    .filter(|submission| seen.insert(submission.id.clone())),
            );
            if results.len() >= limit || !has_next_page(&document, &base, page)? {
                break;
            }
        }
        results.truncate(limit);
        Ok(results)
    }
}

fn resolve_url(url: &Url) -> Result<ResourceRef> {
    ensure!(
        ServiceId::from_url(url)? == ServiceId::Codeforces,
        "Expected a Codeforces URL"
    );
    let segments: Vec<_> = url.path().trim_matches('/').split('/').collect();
    let (kind, contest, index) = match segments.as_slice() {
        [kind @ ("contest" | "gym"), contest] => (*kind, *contest, None),
        [kind @ ("contest" | "gym"), contest, "problem", index] => (*kind, *contest, Some(*index)),
        ["problemset", "problem", contest, index] => ("contest", *contest, Some(*index)),
        _ => {
            bail!("Unsupported Codeforces URL: {url}; expected contest, problemset/problem, or gym")
        }
    };
    let contest: u64 = contest.parse().context("Invalid Codeforces contest ID")?;
    ensure!(contest > 0, "Invalid Codeforces contest ID");
    match index {
        Some(index) => {
            // Validate a single path component without changing numeric indices such as 0.
            ensure!(
                !index.is_empty() && index.bytes().all(|c| c.is_ascii_alphanumeric()),
                "Invalid Codeforces problem index"
            );
            let index = index.to_ascii_uppercase();
            let separator = if index.starts_with(|c: char| c.is_ascii_digit()) {
                "_"
            } else {
                ""
            };
            Ok(ResourceRef::Problem(ProblemRef {
                service: ServiceId::Codeforces,
                id: format!("{contest}{separator}{index}"),
                url: Url::parse(&format!(
                    "https://codeforces.com/{kind}/{contest}/problem/{index}"
                ))?,
                contest_id: Some(contest.to_string()),
                internal_id: None,
            }))
        }
        None => Ok(ResourceRef::Contest(ContestRef {
            service: ServiceId::Codeforces,
            id: contest.to_string(),
            url: Url::parse(&format!("https://codeforces.com/{kind}/{contest}"))?,
        })),
    }
}

fn contest_url(url: &Url) -> Result<Url> {
    let resolved = resolve_url(url)?;
    let url = match resolved {
        ResourceRef::Problem(problem) => problem.url,
        ResourceRef::Contest(contest) => contest.url,
    };
    let mut parts = url.path_segments().expect("Codeforces URL path");
    let kind = parts.next().expect("contest kind");
    let contest = parts.next().expect("contest ID");
    Ok(Url::parse(&format!(
        "https://codeforces.com/{kind}/{contest}/"
    ))?)
}

fn check_page(document: &Html) -> Result<()> {
    ensure!(document.select(&selector("#challenge-form, #cf-challenge-running, .g-recaptcha, iframe[src*='challenges.cloudflare.com']")).next().is_none(),
        "Codeforces requires a browser challenge or CAPTCHA; open the page in your browser and import fresh cookies. For manual submission use submit --clipboard --open");
    ensure!(
        document
            .select(&selector("#enterForm, input[name='handleOrEmail']"))
            .next()
            .is_none(),
        "Codeforces session expired; import fresh cookies with cpg login codeforces"
    );
    Ok(())
}

fn samples(document: &Html) -> Result<Vec<Sample>> {
    let statement = required(document, ".problem-statement")?;
    let inputs: Vec<_> = statement.select(&selector(".sample-test .input")).collect();
    let outputs: Vec<_> = statement
        .select(&selector(".sample-test .output"))
        .collect();
    ensure!(
        inputs.len() == outputs.len(),
        "Codeforces sample input/output count mismatch"
    );
    inputs
        .into_iter()
        .zip(outputs)
        .map(|(input, output)| {
            let pre = selector("pre");
            Ok(Sample {
                input: sample_text(
                    input
                        .select(&pre)
                        .next()
                        .context("Codeforces sample input has no pre")?,
                ),
                output: sample_text(
                    output
                        .select(&pre)
                        .next()
                        .context("Codeforces sample output has no pre")?,
                ),
            })
        })
        .collect()
}

fn sample_text(pre: ElementRef<'_>) -> String {
    let lines: Vec<_> = pre
        .children()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "div")
        .collect();
    let mut output = if lines.is_empty() {
        pre_text(pre)
    } else {
        lines
            .into_iter()
            .map(|line| {
                let mut line = pre_text(line);
                if !line.ends_with('\n') {
                    line.push('\n');
                }
                line
            })
            .collect()
    };
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn parse_contest(document: &Html, page: &Url, contest: &ContestRef) -> Result<Contest> {
    let table = required(document, "table.problems")?;
    let mut problems = Vec::new();
    for row in table.select(&selector("tr")) {
        let Some(link) = row.select(&selector("td.id a[href]")).next() else {
            continue;
        };
        let url = page.join(link.value().attr("href").expect("selected href"))?;
        let problem = resolve_url(&url)?.problem()?;
        ensure!(
            problem.contest_id.as_deref() == Some(&contest.id),
            "Problem belongs to another contest"
        );
        problems.push(problem);
    }
    ensure!(
        !problems.is_empty(),
        "Codeforces contest has no accessible problems; check contest registration"
    );
    let title = text(required(document, "title")?);
    let title = title
        .strip_prefix("Dashboard - ")
        .and_then(|title| title.strip_suffix(" - Codeforces"))
        .context("Codeforces contest title format changed")?;
    Ok(Contest {
        reference: contest.clone(),
        title: title.into(),
        problems,
    })
}

fn submission_form(document: &Html) -> Result<ElementRef<'_>> {
    check_page(document)?;
    document.select(&selector("form")).find(|form|
        form.select(&selector("select[name='programTypeId']")).next().is_some())
        .context("Codeforces submission form unavailable; check cookies and contest registration in your browser. Use submit --clipboard --open for manual submission")
}

fn languages(form: ElementRef<'_>) -> Result<Vec<SubmissionLanguage>> {
    let languages: Vec<_> = form
        .select(&selector("select[name='programTypeId'] option[value]"))
        .filter_map(|option| {
            let id = option.value().attr("value").expect("selected value");
            (!id.is_empty() && option.value().attr("disabled").is_none()).then(|| {
                SubmissionLanguage {
                    id: id.into(),
                    name: text(option),
                }
            })
        })
        .collect();
    ensure!(
        !languages.is_empty(),
        "No Codeforces submission languages available"
    );
    Ok(languages)
}

fn submission_fields(
    document: &Html,
    page: &Url,
    request: &SubmissionRequest<'_>,
) -> Result<(Url, Vec<(String, String)>)> {
    let form = submission_form(document)?;
    ensure!(
        languages(form)?
            .iter()
            .any(|language| language.id == request.language),
        "Codeforces submission language is unavailable"
    );
    let mut fields = form_fields(form)?;
    let token = required(document, "meta[name='X-Csrf-Token'][content]")?
        .value()
        .attr("content")
        .expect("selected content");
    ensure!(
        !token.is_empty(),
        "Codeforces submission page has no CSRF token"
    );
    let mut action = form_action(form, page)?;
    action.query_pairs_mut().append_pair("csrf_token", token);
    let problem = resolve_url(&request.problem.url)?.problem()?;
    let index = problem
        .url
        .path_segments()
        .expect("problem URL")
        .next_back()
        .expect("problem index");
    fields.retain(|(key, _)| {
        ![
            "action",
            "submittedProblemIndex",
            "programTypeId",
            "source",
            "tabSize",
            "csrf_token",
            "ftaa",
            "bfaa",
        ]
        .contains(&key.as_str())
    });
    fields.extend([
        ("csrf_token".into(), token.into()),
        ("action".into(), "submitSolutionFormSubmitted".into()),
        ("submittedProblemIndex".into(), index.into()),
        ("programTypeId".into(), request.language.into()),
        ("source".into(), request.source.into()),
        ("tabSize".into(), "4".into()),
    ]);
    // Codeforces fills these hidden inputs from server-provided JavaScript values.
    for name in ["ftaa", "bfaa"] {
        let marker = format!("window._{name}");
        let value = document
            .select(&selector("script"))
            .find_map(|script| {
                let script = script.text().collect::<String>();
                let (_, assignment) = script.split_once(&marker)?;
                let value = assignment.trim_start().strip_prefix('=')?.trim_start();
                serde_json::Deserializer::from_str(value)
                    .into_iter::<String>()
                    .next()
            })
            .with_context(|| format!("Codeforces page has no {name} submission token"))??;
        ensure!(
            !value.is_empty(),
            "Codeforces {name} submission token is empty"
        );
        fields.push((name.into(), value));
    }
    Ok((action, fields))
}

fn parse_submissions(document: &Html, page: &Url) -> Result<Vec<Submission>> {
    check_page(document)?;
    required(document, "#header .lang-chooser a[href^='/profile/']")
        .context("Codeforces submissions require a logged-in session; import fresh cookies")?;
    let table = required(document, "table.status-frame-datatable")?;
    let mut submissions = Vec::new();
    for row in table.select(&selector("tr[data-submission-id]")) {
        let id = row
            .value()
            .attr("data-submission-id")
            .expect("selected submission ID");
        id.parse::<u64>()
            .context("Invalid Codeforces submission ID")?;
        let cells: Vec<_> = row.select(&selector("td")).collect();
        ensure!(
            cells.len() == 8,
            "Codeforces submissions table format changed"
        );
        let problem_link = cells[3]
            .select(&selector("a[href]"))
            .next()
            .context("Missing submitted problem")?;
        let problem_url = page.join(problem_link.value().attr("href").expect("selected href"))?;
        let problem = resolve_url(&problem_url)?.problem()?;
        let verdict = cells[5]
            .select(&selector(".submissionVerdictWrapper"))
            .next()
            .context("Missing submission verdict")?;
        let status = match verdict.value().attr("submissionverdict") {
            _ if row.select(&selector("[waiting='true']")).next().is_some() => "WJ".into(),
            Some("OK") => "AC".into(),
            Some("WRONG_ANSWER") => "WA".into(),
            Some("TIME_LIMIT_EXCEEDED") => "TLE".into(),
            Some("MEMORY_LIMIT_EXCEEDED") => "MLE".into(),
            Some("RUNTIME_ERROR") => "RE".into(),
            Some("COMPILATION_ERROR") => "CE".into(),
            Some("TESTING" | "SUBMITTED") => "WJ".into(),
            _ => text(verdict),
        };
        submissions.push(Submission {
            id: id.into(),
            url: contest_url(&problem.url)?.join(&format!("submission/{id}"))?,
            problem_id: problem.id,
            submitted_at: text(cells[1]),
            language: text(cells[4]),
            status,
            time: text(cells[6]).replace('\u{a0}', " "),
        });
    }
    Ok(submissions)
}

fn has_next_page(document: &Html, base: &Url, page: usize) -> Result<bool> {
    let next = format!("{}/page/{}", base.path(), page + 1);
    for link in document.select(&selector(".pagination a[href]")) {
        let url = base.join(link.value().attr("href").expect("selected href"))?;
        if url.origin() == base.origin() && url.path() == next {
            return Ok(true);
        }
    }
    Ok(false)
}

fn identify_submission(
    submissions: Vec<Submission>,
    problem: &ProblemRef,
    before: Option<u64>,
) -> Result<Submission> {
    let mut new = Vec::new();
    for submission in submissions {
        let id: u64 = submission.id.parse()?;
        if submission.problem_id == problem.id && before.is_none_or(|before| id > before) {
            new.push(submission);
        }
    }
    ensure!(
        new.len() == 1,
        "Cannot uniquely identify the submitted solution; check results before submitting again"
    );
    Ok(new.remove(0))
}
