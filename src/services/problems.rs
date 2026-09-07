use super::{ServiceBackend, atcoder::AtCoderBackend, sort_submissions};
use crate::model::*;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

pub(super) struct AtCoderProblemsBackend {
    pub atcoder: AtCoderBackend,
}

#[derive(Deserialize)]
struct VirtualContest {
    info: Info,
    problems: Vec<Item>,
}
#[derive(Deserialize)]
struct Info {
    title: String,
}
#[derive(Deserialize)]
struct Item {
    id: String,
    order: Option<usize>,
}
#[derive(Deserialize)]
struct ProblemData {
    id: String,
    contest_id: String,
}

impl ServiceBackend for AtCoderProblemsBackend {
    fn service(&self) -> ServiceId {
        ServiceId::AtcoderProblems
    }
    fn auth_service(&self) -> ServiceId {
        self.atcoder.auth_service()
    }
    fn whoami(&self) -> Result<(String, Url)> {
        self.atcoder.whoami()
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        ensure!(
            ServiceId::from_url(url)? == self.service(),
            "Expected an AtCoder Problems URL"
        );
        ensure!(
            url.path().trim_end_matches('/') == "/atcoder",
            "Unrecognized AtCoder Problems URL"
        );
        let fragment = url
            .fragment()
            .context("Missing virtual contest URL fragment")?;
        let parts: Vec<_> = fragment.trim_matches('/').split('/').collect();
        let ["contest", "show", id] = parts.as_slice() else {
            anyhow::bail!("Expected #/contest/show/<id>")
        };
        ensure!(!id.is_empty(), "Missing virtual contest ID");
        Ok(ResourceRef::Contest(ContestRef {
            service: self.service(),
            id: (*id).into(),
            url: url.clone(),
        }))
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let mut remote: VirtualContest = self.atcoder.http.json(&Url::parse(&format!(
            "https://kenkoooo.com/atcoder/internal-api/contest/get/{}",
            contest.id
        ))?)?;
        let catalog: Vec<ProblemData> = self.atcoder.http.json(&Url::parse(
            "https://kenkoooo.com/atcoder/resources/problems.json",
        )?)?;
        let catalog: HashMap<_, _> = catalog.into_iter().map(|p| (p.id, p.contest_id)).collect();
        // Legacy contests have null order; preserve their response order after explicitly ordered items.
        remote
            .problems
            .sort_by_key(|p| p.order.map_or((1, 0), |n| (0, n)));
        let mut problems = Vec::new();
        for item in remote.problems {
            let original_contest = catalog.get(&item.id).with_context(|| {
                format!("Problem {} missing from AtCoder Problems catalog", item.id)
            })?;
            let url = Url::parse(&format!(
                "https://atcoder.jp/contests/{original_contest}/tasks/{}",
                item.id
            ))?;
            problems.push(self.atcoder.resolve_url(&url)?.problem()?);
        }
        ensure!(!problems.is_empty(), "Virtual contest has no problems");
        Ok(Contest {
            reference: contest.clone(),
            title: remote.info.title,
            problems,
        })
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        self.atcoder.fetch_problem(problem)
    }
    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        self.atcoder.languages(problem)
    }
    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        self.atcoder.submit(request)
    }
    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        match scope {
            Metadata::Problem { .. } => self.atcoder.submissions(scope, limit),
            Metadata::Contest(contest) => {
                let mut results = Vec::new();
                for problem in &contest.problems {
                    results.extend(self.atcoder.submissions(
                        &Metadata::Problem {
                            reference: problem.clone(),
                            title: String::new(),
                            template_checksums: Default::default(),
                        },
                        limit,
                    )?);
                }
                sort_submissions(&mut results, limit);
                Ok(results)
            }
        }
    }
}
