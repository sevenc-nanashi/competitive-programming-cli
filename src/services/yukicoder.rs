use super::{
    Http, ServiceBackend, form_action, form_fields, next_page, pre_text, required, selector, text,
};
use crate::model::*;
use anyhow::{Context, Result, bail, ensure};
use scraper::{ElementRef, Html};
use serde::Deserialize;
use url::Url;

pub(super) struct YukicoderBackend {
    pub http: Http,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiProblem {
    #[serde(rename = "No")]
    number: Option<u64>,
    problem_id: u64,
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiContest {
    name: String,
    problem_id_list: Vec<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiLanguage {
    id: String,
    name: String,
    status: String,
}

fn sample_text(element: ElementRef<'_>) -> String {
    let mut text = pre_text(element);
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

impl YukicoderBackend {
    fn problem_data(&self, id: u64) -> Result<ApiProblem> {
        self.http.json(&Url::parse(&format!(
            "https://yukicoder.me/api/v1/problems/{id}"
        ))?)
    }

    fn reference(data: &ApiProblem) -> Result<ProblemRef> {
        let (id, path) = match data.number {
            Some(number) => (number.to_string(), format!("no/{number}")),
            None => (
                format!("id-{}", data.problem_id),
                data.problem_id.to_string(),
            ),
        };
        Ok(ProblemRef {
            service: ServiceId::Yukicoder,
            id,
            url: Url::parse(&format!("https://yukicoder.me/problems/{path}"))?,
            contest_id: None,
            internal_id: Some(data.problem_id),
        })
    }

    fn authenticated(document: &Html) -> Result<()> {
        let header = required(document, "#header")?;
        ensure!(
            header
                .select(&selector("a[href^='/users/']"))
                .next()
                .is_some(),
            "yukicoder session expired; import fresh cookies"
        );
        Ok(())
    }

    fn parse_submissions(&self, document: &Html, page: &Url) -> Result<Vec<Submission>> {
        required(document, "table tbody")?;
        let mut submissions = Vec::new();
        for row in document.select(&selector("table tbody tr")) {
            let cells: Vec<_> = row.select(&selector("td")).collect();
            if cells.is_empty() {
                continue;
            }
            ensure!(
                cells.len() >= 9,
                "yukicoder submissions table format changed"
            );
            let link = cells[0]
                .select(&selector("a[href^='/submissions/']"))
                .next()
                .context("Missing submission link")?;
            let url = page.join(link.value().attr("href").expect("selected href"))?;
            let id = text(link);
            id.parse::<u64>()
                .context("Invalid yukicoder submission ID")?;
            let problem = cells[4]
                .select(&selector("a[href^='/problems/']"))
                .next()
                .context("Missing problem link")?;
            let problem = self
                .resolve_url(&page.join(problem.value().attr("href").expect("selected href"))?)?
                .problem()?;
            submissions.push(Submission {
                id,
                url,
                problem_id: problem.id,
                submitted_at: text(cells[1]),
                language: text(cells[5]),
                status: text(cells[6]),
                time: text(cells[7]),
            });
        }
        Ok(submissions)
    }
}

impl ServiceBackend for YukicoderBackend {
    fn service(&self) -> ServiceId {
        ServiceId::Yukicoder
    }
    fn auth_service(&self) -> ServiceId {
        ServiceId::Yukicoder
    }

    fn check_auth(&self) -> Result<()> {
        let (_, document) = self.http.get(&Url::parse("https://yukicoder.me/")?)?;
        Self::authenticated(&document)
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        ensure!(
            ServiceId::from_url(url)? == self.service(),
            "Expected a yukicoder URL"
        );
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        match parts.as_slice() {
            ["problems", "no", number] => {
                let number: u64 = number.parse().context("Invalid yukicoder problem number")?;
                Ok(ResourceRef::Problem(ProblemRef {
                    service: self.service(),
                    id: number.to_string(),
                    url: Url::parse(&format!("https://yukicoder.me/problems/no/{number}"))?,
                    contest_id: None,
                    internal_id: None,
                }))
            }
            ["problems", id] => Ok(ResourceRef::Problem(Self::reference(
                &self.problem_data(id.parse().context("Invalid yukicoder problem ID")?)?,
            )?)),
            ["contests", id] => {
                let id: u64 = id.parse().context("Invalid yukicoder contest ID")?;
                Ok(ResourceRef::Contest(ContestRef {
                    service: self.service(),
                    id: id.to_string(),
                    url: Url::parse(&format!("https://yukicoder.me/contests/{id}"))?,
                }))
            }
            _ => bail!("Unrecognized yukicoder problem/contest URL: {url}"),
        }
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let (_, document) = self.http.get(&problem.url)?;
        let content = required(&document, "#content[data-problem-id]")?;
        let id: u64 = content
            .value()
            .attr("data-problem-id")
            .expect("selected problem ID")
            .parse()?;
        let data = self.problem_data(id)?;
        let mut reference = Self::reference(&data)?;
        reference.contest_id = problem.contest_id.clone();
        let mut samples = Vec::new();
        for sample in document.select(&selector(".sample")) {
            let pre = selector("pre");
            let mut blocks = sample.select(&pre);
            let input = blocks.next().context("yukicoder sample input is missing")?;
            let output = blocks
                .next()
                .context("yukicoder sample output is missing")?;
            samples.push(Sample {
                input: sample_text(input),
                output: sample_text(output),
            });
        }
        Ok(Problem {
            reference,
            title: data.title,
            samples,
        })
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let remote: ApiContest = self.http.json(&Url::parse(&format!(
            "https://yukicoder.me/api/v1/contest/id/{}",
            contest.id
        ))?)?;
        let mut problems = Vec::new();
        for id in remote.problem_id_list {
            let mut reference = Self::reference(&self.problem_data(id)?)?;
            reference.contest_id = Some(contest.id.clone());
            problems.push(reference);
        }
        ensure!(!problems.is_empty(), "Contest has no accessible problems");
        Ok(Contest {
            reference: contest.clone(),
            title: remote.name,
            problems,
        })
    }

    fn languages(&self, _problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        let languages: Vec<ApiLanguage> = self
            .http
            .json(&Url::parse("https://yukicoder.me/api/v1/languages")?)?;
        Ok(languages
            .into_iter()
            .filter(|language| language.status == "enable")
            .map(|language| SubmissionLanguage {
                id: language.id,
                name: language.name,
            })
            .collect())
    }

    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission> {
        let page = Url::parse(&format!(
            "{}/submit",
            request.problem.url.as_str().trim_end_matches('/')
        ))?;
        let (page, document) = self.http.get(&page)?;
        let form = required(&document, "form#submit_form")
            .context("yukicoder submission form is unavailable; check your cookies")?;
        let csrf = required(&document, "meta[name='csrf-token']")?
            .value()
            .attr("content")
            .context("Missing yukicoder CSRF token")?;
        let mut fields = form_fields(form)?;
        fields.retain(|(key, _)| !["lang", "source", "file"].contains(&key.as_str()));
        fields.push(("lang".into(), request.language.into()));
        let action = form_action(form, &page)?;
        let (url, _) = self
            .http
            .post(&action, fields, Some(request.source), Some(csrf))?;
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        let ["submissions", id] = parts.as_slice() else {
            bail!("yukicoder did not confirm submission; check results before submitting again")
        };
        id.parse::<u64>()
            .context("Invalid submission confirmation")?;
        Ok(Submission {
            id: (*id).into(),
            url: url.clone(),
            problem_id: request.problem.id.clone(),
            submitted_at: String::new(),
            language: request.language.into(),
            status: "WJ".into(),
            time: String::new(),
        })
    }

    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        let base = match scope {
            Metadata::Problem { reference, .. } => &reference.url,
            Metadata::Contest(contest) => &contest.reference.url,
        };
        let mut results = Vec::new();
        for page in 1.. {
            let mut url = Url::parse(&format!(
                "{}/submissions",
                base.as_str().trim_end_matches('/')
            ))?;
            url.query_pairs_mut()
                .append_pair("my_submission", "enabled")
                .append_pair("page", &page.to_string());
            let (_, document) = self.http.get(&url)?;
            Self::authenticated(&document)?;
            results.extend(self.parse_submissions(&document, &url)?);
            if results.len() >= limit || !next_page(&document, &url, page)? {
                break;
            }
        }
        results.truncate(limit);
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_text_adds_missing_newline() {
        for (html, expected) in [
            ("<pre>1 2</pre>", "1 2\n"),
            ("<pre>1 &lt; 2<br><span>3</span></pre>", "1 < 2\n3\n"),
            ("<pre>1 2\n</pre>", "1 2\n"),
            ("<pre>1 2\n\n</pre>", "1 2\n\n"),
            ("<pre> 1 2 </pre>", " 1 2 \n"),
            ("<pre></pre>", ""),
        ] {
            let document = Html::parse_document(html);
            assert_eq!(sample_text(required(&document, "pre").unwrap()), expected);
        }
    }

    #[test]
    fn problem_numbers_and_submission_pages() {
        let backend = YukicoderBackend {
            http: Http::from_cookies(&[], ServiceId::Yukicoder).unwrap(),
        };
        let data: ApiProblem =
            serde_json::from_str(r#"{"No":1586,"ProblemId":6690,"Title":"Problem"}"#).unwrap();
        let reference = YukicoderBackend::reference(&data).unwrap();
        assert_eq!(reference.id, "1586");
        assert_eq!(reference.internal_id, Some(6690));
        let unpublished: ApiProblem =
            serde_json::from_str(r#"{"No":null,"ProblemId":6690,"Title":"Problem"}"#).unwrap();
        assert_eq!(
            YukicoderBackend::reference(&unpublished)
                .unwrap()
                .url
                .path(),
            "/problems/6690"
        );
        let page = Url::parse("https://yukicoder.me/problems/no/1586/submissions").unwrap();
        let document = Html::parse_document(
            "<div id=header><a href='/users/1'>User</a></div><table><tbody><tr><td><a href='/submissions/42'>42</a></td><td>2026-01-01 12:00:00</td><td>User</td><td></td><td><a href='/problems/no/1586'>Problem</a></td><td>Ruby</td><td>AC</td><td>10 ms</td><td>32 KB</td></tr></tbody></table><a href='?page=2'>Next</a>",
        );
        YukicoderBackend::authenticated(&document).unwrap();
        let results = backend.parse_submissions(&document, &page).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            (
                &*results[0].id,
                &*results[0].problem_id,
                &*results[0].language,
                &*results[0].time
            ),
            ("42", "1586", "Ruby", "10 ms")
        );
        assert!(next_page(&document, &page, 1).unwrap());
        assert!(!next_page(&document, &page, 2).unwrap());
        assert!(
            YukicoderBackend::authenticated(&Html::parse_document(
                "<div id=header><a class=login-btn href='/auth/github'>Login</a></div>"
            ))
            .is_err()
        );
        let sample = Html::parse_document("<pre>1 &lt; 2<br>3\n</pre>");
        assert_eq!(pre_text(required(&sample, "pre").unwrap()), "1 < 2\n3\n");
    }
}
