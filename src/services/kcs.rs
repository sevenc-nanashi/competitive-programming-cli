use super::{Http, ServiceBackend, required, selector, text};
use crate::{model::*, services::FetchMethod};
use anyhow::{Context, Result, bail, ensure};
use scraper::Html;
use url::Url;

pub(super) struct KcsBackend {
    pub http: Http,
}

impl KcsBackend {
    fn submission_page(&self, problem: &ProblemRef) -> Result<(Url, Html)> {
        self.http
            .get(&Url::parse(&format!("{}/submit", problem.url))?)
    }

    fn submission_ids(&self, url: &Url) -> Result<Vec<String>> {
        let (_, document) = self.http.get(url)?;
        required(&document, "#submissions-list")?;
        let prefix = url
            .path()
            .strip_suffix("mine")
            .context("Invalid KCS submissions URL")?;
        let mut ids = Vec::new();
        for link in document.select(&selector("#submissions-list a[href]")) {
            let href = link.value().attr("href").expect("selected href");
            if let Some(id) = href.strip_prefix(prefix)
                && id.parse::<u64>().is_ok()
                && !ids.iter().any(|existing| existing == id)
            {
                ids.push(id.to_owned());
            }
        }
        Ok(ids)
    }
}

impl ServiceBackend for KcsBackend {
    fn whoami(&self, _service: &ServiceId) -> Result<(String, Option<Url>)> {
        let (_, document) = self
            .http
            .get(&Url::parse("https://kcs.miz-miz.biz/settings")?)?;
        let user = text(required(
            &document,
            "section[aria-labelledby='user-id-heading'] li span",
        )?);
        ensure!(!user.is_empty(), "KCS user ID is empty");
        Ok((user, None))
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        ensure!(
            ServiceId::from_url(url)? == ServiceId::Kcs,
            "Expected a KCS URL"
        );
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        let (contest, problem) = match parts.as_slice() {
            ["contests" | "contest", contest] => (*contest, None),
            ["contests", contest, "problems"] => (*contest, None),
            ["contests", contest, "problems", problem] | ["contest", contest, "view", problem]
                if !problem.is_empty() =>
            {
                (*contest, Some(*problem))
            }
            _ => bail!("Unrecognized KCS problem/contest URL: {url}"),
        };
        let contest: u64 = contest.parse().context("Invalid KCS contest ID")?;
        let base = format!("https://kcs.miz-miz.biz/contests/{contest}");
        Ok(match problem {
            Some(problem) => ResourceRef::Problem(ProblemRef {
                service: ServiceId::Kcs,
                id: problem.into(),
                url: Url::parse(&format!("{base}/problems/{problem}"))?,
                contest_id: Some(contest.to_string()),
                internal_id: None,
            }),
            None => ResourceRef::Contest(ContestRef {
                service: ServiceId::Kcs,
                id: contest.to_string(),
                url: Url::parse(&base)?,
            }),
        })
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let (_, document) = self.http.get(&problem.url)?;
        Ok(Problem {
            reference: problem.clone(),
            title: text(required(&document, "main h1")?),
            samples: Vec::new(),
        })
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let (_, document) = self.http.get(&contest.url)?;
        let title = text(required(&document, "main h1")?);
        let url = Url::parse(&format!("{}/problems", contest.url))?;
        let (_, document) = self.http.get(&url)?;
        let table = required(&document, "main table tbody")?;
        let mut problems = Vec::new();
        for row in table.select(&selector("tr")) {
            let link = row
                .select(&selector("a[href*='/problems/']"))
                .next()
                .context("KCS contest row has no problem link")?;
            let target = url.join(link.value().attr("href").expect("selected href"))?;
            problems.push(self.resolve_url(&target)?.problem()?);
        }
        ensure!(
            !problems.is_empty(),
            "KCS contest has no accessible problems"
        );
        Ok(Contest {
            reference: contest.clone(),
            title,
            problems,
        })
    }

    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        let (_, document) = self.submission_page(problem)?;
        let form = required(&document, "form#submission-form")?;
        let languages: Vec<_> = form
            .select(&selector("li[role=option][data-id][data-label]"))
            .map(|option| SubmissionLanguage {
                id: option.value().attr("data-id").expect("selected ID").into(),
                name: option
                    .value()
                    .attr("data-label")
                    .expect("selected label")
                    .into(),
            })
            .collect();
        ensure!(
            !languages.is_empty(),
            "No KCS submission languages available"
        );
        Ok(languages)
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        let (page, document) = self.submission_page(request.problem)?;
        let form = required(&document, "form#submission-form[hx-post]")?;
        let action = page.join(form.value().attr("hx-post").expect("selected action"))?;
        ensure!(action == page, "Unexpected KCS submission form action");
        let csrf = required(&document, "meta[name=csrf-token]")?
            .value()
            .attr("content")
            .filter(|value| !value.is_empty())
            .context("Missing KCS CSRF token")?;
        let contest = request
            .problem
            .contest_id
            .as_ref()
            .context("Missing KCS contest ID")?;
        let listing = Url::parse(&format!(
            "https://kcs.miz-miz.biz/contests/{contest}/submissions/mine"
        ))?;
        let before = self.submission_ids(&listing)?;
        let response = self
            .http
            .client
            .post(self.http.request_url(&action)?)
            .header(reqwest::header::REFERER, page.as_str())
            .header(reqwest::header::ORIGIN, page.origin().ascii_serialization())
            .header("HX-Request", "true")
            .header("X-CSRF-Token", csrf)
            .form(&[("language_id", request.language), ("code", request.source)])
            .send()
            .context("Submission outcome is unknown; check results before submitting again")?;
        self.http
            .response(&action, response)
            .context("KCS did not confirm submission; check results before submitting again")?;
        let mut new: Vec<_> = self
            .submission_ids(&listing)
            .context("Submission outcome is unknown; check results before submitting again")?
            .into_iter()
            .filter(|id| !before.contains(id))
            .collect();
        ensure!(
            new.len() == 1,
            "Cannot uniquely identify the KCS submission; check results before submitting again"
        );
        let id = new.remove(0);
        Ok(Submission {
            url: listing.join(&id)?,
            id,
            problem_id: request.problem.id.clone(),
            submitted_at: String::new(),
            language: request.language.into(),
            status: "WJ".into(),
            time: String::new(),
        })
    }

    fn submissions(
        &self,
        _scope: &SubmissionScope,
        _limit: usize,
    ) -> Result<(Vec<Submission>, FetchMethod)> {
        bail!("KCS result monitoring is not supported; view submissions in your browser")
    }
}
