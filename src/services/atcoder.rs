use super::{
    Http, ServiceBackend, form_action, form_fields, next_page, pre_text, required, selector, text,
};
use crate::model::*;
use anyhow::{Context, Result, bail, ensure};
use scraper::{ElementRef, Html};
use std::collections::BTreeMap;
use url::Url;

#[derive(Clone)]
pub(super) struct AtCoderBackend {
    pub http: Http,
}

impl AtCoderBackend {
    fn submission_page(&self, problem: &ProblemRef) -> Result<(Url, Html)> {
        let contest = problem
            .contest_id
            .as_ref()
            .context("AtCoder problem is missing its contest ID")?;
        self.http.get(&Url::parse(&format!(
            "https://atcoder.jp/contests/{contest}/submit"
        ))?)
    }

    fn submission_form(document: &Html) -> Result<ElementRef<'_>> {
        document
            .select(&selector("form"))
            .find(|form| {
                form.select(&selector("div[data-name='data.LanguageId']"))
                    .next()
                    .is_some()
            })
            .context(
                "AtCoder submission form is unavailable; check your cookies and contest access",
            )
    }
}

impl ServiceBackend for AtCoderBackend {
    fn service(&self) -> ServiceId {
        ServiceId::Atcoder
    }
    fn auth_service(&self) -> ServiceId {
        ServiceId::Atcoder
    }

    fn check_auth(&self) -> Result<()> {
        let (url, _) = self.http.get(&Url::parse("https://atcoder.jp/settings")?)?;
        ensure!(
            url.path().starts_with("/settings"),
            "AtCoder session expired; import fresh cookies"
        );
        Ok(())
    }

    fn resolve_url(&self, url: &Url) -> Result<ResourceRef> {
        ensure!(
            ServiceId::from_url(url)? == self.service(),
            "Expected an AtCoder URL"
        );
        let segments: Vec<_> = url.path().trim_matches('/').split('/').collect();
        match segments.as_slice() {
            ["contests", contest, "tasks", problem]
                if !contest.is_empty() && !problem.is_empty() =>
            {
                Ok(ResourceRef::Problem(ProblemRef {
                    service: self.service(),
                    id: (*problem).into(),
                    url: Url::parse(&format!(
                        "https://atcoder.jp/contests/{contest}/tasks/{problem}"
                    ))?,
                    contest_id: Some((*contest).into()),
                    internal_id: None,
                }))
            }
            ["contests", contest] | ["contests", contest, "tasks"] if !contest.is_empty() => {
                Ok(ResourceRef::Contest(ContestRef {
                    service: self.service(),
                    id: (*contest).into(),
                    url: Url::parse(&format!("https://atcoder.jp/contests/{contest}"))?,
                }))
            }
            _ => bail!("Unrecognized AtCoder problem/contest URL: {url}"),
        }
    }

    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem> {
        let (_, document) = self.http.get(&problem.url)?;
        let title = problem_title(&document)?;
        let statement = required(&document, "#task-statement")?;
        let japanese = statement.select(&selector(".lang-ja")).next();
        let english = statement.select(&selector(".lang-en")).next();
        let section = match (japanese, english) {
            (Some(ja), _) => ja,
            (None, Some(en)) => en,
            (None, None) => statement,
        };
        let mut cases: BTreeMap<usize, (Option<String>, Option<String>)> = BTreeMap::new();
        for heading in section.select(&selector("h3")) {
            let heading_text = text(heading);
            let kind = [
                ("入力例", true),
                ("出力例", false),
                ("Sample Input", true),
                ("Sample Output", false),
            ]
            .iter()
            .find_map(|(prefix, input)| {
                heading_text
                    .strip_prefix(prefix)
                    .map(|number| (number.trim(), *input))
            });
            let Some((number, input)) = kind else {
                continue;
            };
            let number: usize = number.parse().context("Invalid AtCoder sample number")?;
            let pre = heading
                .next_siblings()
                .filter_map(ElementRef::wrap)
                .take_while(|e| e.value().name() != "h3")
                .find_map(|element| {
                    if element.value().name() == "pre" {
                        Some(element)
                    } else {
                        element.select(&selector("pre")).next()
                    }
                })
                .context("AtCoder sample has no preformatted text")?;
            let pair = cases.entry(number).or_default();
            let target = if input { &mut pair.0 } else { &mut pair.1 };
            ensure!(target.is_none(), "Duplicate AtCoder sample number {number}");
            *target = Some(pre_text(pre));
        }
        let samples = cases
            .into_values()
            .map(|(input, output)| {
                Ok(Sample {
                    input: input.context("Sample input missing")?,
                    output: output.context("Sample output missing")?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Problem {
            reference: problem.clone(),
            title,
            samples,
        })
    }

    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest> {
        let url = Url::parse(&format!("https://atcoder.jp/contests/{}/tasks", contest.id))?;
        let (_, document) = self.http.get(&url)?;
        required(&document, "table tbody")?;
        let mut problems = Vec::new();
        for row in document.select(&selector("table tbody tr")) {
            let link = row
                .select(&selector("a[href*='/tasks/']"))
                .next()
                .context("Contest row has no problem link")?;
            let target = url.join(link.value().attr("href").expect("selected href"))?;
            problems.push(self.resolve_url(&target)?.problem()?);
        }
        ensure!(!problems.is_empty(), "Contest has no accessible problems");
        let title = text(required(&document, "a.contest-title")?);
        Ok(Contest {
            reference: contest.clone(),
            title,
            problems,
        })
    }

    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>> {
        let (_, document) = self.submission_page(problem)?;
        let form = Self::submission_form(&document)?;
        let mut languages = std::collections::HashMap::new();
        for option in form.select(&selector("div[data-name='data.LanguageId'] option[value]")) {
            let id = option.value().attr("value").expect("selected value");
            if !id.is_empty() {
                languages.insert(
                    id.to_owned(),
                    SubmissionLanguage {
                        id: id.into(),
                        name: text(option),
                    },
                );
            }
        }
        ensure!(
            !languages.is_empty(),
            "No AtCoder submission languages available"
        );
        let mut languages = languages.into_values().collect::<Vec<_>>();
        languages.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(languages)
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
        let form = Self::submission_form(&document)?;
        let mut fields = form_fields(form)?;
        ensure!(
            fields
                .iter()
                .any(|(k, v)| k == "csrf_token" && !v.is_empty()),
            "AtCoder form has no CSRF token"
        );
        fields.retain(|(key, _)| {
            !["data.TaskScreenName", "data.LanguageId", "sourceCode"].contains(&key.as_str())
        });
        fields.extend([
            ("data.TaskScreenName".into(), request.problem.id.clone()),
            ("data.LanguageId".into(), request.language.into()),
            ("sourceCode".into(), request.source.into()),
        ]);
        let action = form_action(form, &page)?;
        let (url, document) = self.http.post(&action, fields, None, None)?;
        ensure!(
            url.path().contains("/submissions/"),
            concat!(
                "AtCoder did not confirm submission; check results before submitting again. ",
                "Note that AtCoder has captchas for past contests, which cannot be bypassed by cpcli.",
                "Please submit manually if this is the case."
            )
        );
        let submissions = parse_submissions(&document, &url)?;
        let mut new = Vec::new();
        for submission in submissions {
            if submission.problem_id == request.problem.id
                && before.is_none_or(|id| submission.id.parse::<u64>().is_ok_and(|n| n > id))
            {
                new.push(submission);
            }
        }
        ensure!(
            new.len() == 1,
            "Cannot uniquely identify the submitted solution; check results before submitting again"
        );
        Ok(new.remove(0))
    }

    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>> {
        let (contest, problem) = match scope {
            Metadata::Problem { reference, .. } => (
                reference
                    .contest_id
                    .as_ref()
                    .context("Missing AtCoder contest ID")?
                    .as_str(),
                Some(reference.id.as_str()),
            ),
            Metadata::Contest(contest) => (contest.reference.id.as_str(), None),
        };
        let mut results = Vec::new();
        for page in 1.. {
            let mut url = Url::parse(&format!(
                "https://atcoder.jp/contests/{contest}/submissions/me"
            ))?;
            url.query_pairs_mut().append_pair("page", &page.to_string());
            if let Some(problem) = problem {
                url.query_pairs_mut().append_pair("f.Task", problem);
            }
            let (_, document) = self.http.get(&url)?;
            results.extend(
                parse_submissions(&document, &url)?
                    .into_iter()
                    .filter(|s| problem.is_none_or(|id| s.problem_id == id)),
            );
            if results.len() >= limit || !next_page(&document, &url, page)? {
                break;
            }
        }
        results.truncate(limit);
        Ok(results)
    }
}

fn problem_title(document: &Html) -> Result<String> {
    let mut title = String::new();
    for node in required(document, "span.h2")?.descendants() {
        if let scraper::Node::Text(value) = node.value()
            && !node
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|element| element.value().name() == "a")
        {
            title.push_str(value);
        }
    }
    Ok(title.trim().to_owned())
}

fn parse_submissions(document: &Html, page: &Url) -> Result<Vec<Submission>> {
    // AtCoder omits the table when this filter has no submissions.
    if document.select(&selector("table tbody")).next().is_none() {
        required(document, "select[name='f.Task']")?;
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    for row in document.select(&selector("table tbody tr")) {
        let cells: Vec<_> = row.select(&selector("td")).collect();
        if cells.is_empty() {
            continue;
        }
        let cells = match cells.len() {
            8 | 10 => &cells[..],
            9 | 11 => &cells[1..], // Contest administrators see an extra leading column.
            _ => bail!("AtCoder submissions table format changed"),
        };
        let detail = cells[cells.len() - 1]
            .select(&selector("a[href*='/submissions/']"))
            .next()
            .context("Missing submission detail link")?;
        let url = page.join(detail.value().attr("href").expect("selected href"))?;
        let id = url
            .path_segments()
            .context("Invalid submission URL")?
            .next_back()
            .context("Missing submission ID")?
            .to_owned();
        id.parse::<u64>().context("Invalid submission ID")?;
        let task = cells[1]
            .select(&selector("a[href]"))
            .next()
            .context("Missing submitted problem link")?;
        let task_url = page.join(task.value().attr("href").expect("selected href"))?;
        let problem_id = task_url
            .path_segments()
            .context("Invalid task URL")?
            .next_back()
            .context("Missing task ID")?
            .to_owned();
        let time = match cells.get(7) {
            Some(cell) if cells.len() >= 10 => text(*cell),
            _ => String::new(),
        };
        results.push(Submission {
            id,
            url,
            problem_id,
            submitted_at: text(cells[0]),
            language: text(cells[3]),
            status: text(cells[6]),
            time,
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_excludes_links_and_their_children() {
        for html in [
            "<span class='h2'> A - <b>A &amp; B</b> </span>",
            "<span class='h2'> A - <b>A &amp; B</b> <a href='/editorial'>解説</a> </span>",
            "<span class='h2'> A - <b>A &amp; B</b> <a href='/editorial'><span>Editorial</span></a> </span>",
        ] {
            assert_eq!(
                problem_title(&Html::parse_document(html)).unwrap(),
                "A - A & B"
            );
        }
    }

    #[test]
    fn submission_pages_and_forms() {
        let page = Url::parse("https://atcoder.jp/contests/abc100/submissions/me").unwrap();
        // Language links filter the listing and precede the numeric submission detail link.
        let row = "<td>2026-01-01 12:00:00+0900</td><td><a href='/contests/abc100/tasks/abc100_a'>A</a></td><td>user</td><td><a href='/contests/abc100/submissions/me?f.LanguageName=C%2B%2B23'>C++23</a></td><td>100</td><td>32 Byte</td><td>AC</td><td>1 ms</td><td>32 KB</td><td><a href='/contests/abc100/submissions/42'>Detail</a></td>";
        for prefix in ["", "<td>admin</td>"] {
            let document = Html::parse_document(&format!(
                "<table><tbody><tr>{prefix}{row}</tr></tbody></table>"
            ));
            let submissions = parse_submissions(&document, &page).unwrap();
            assert_eq!(submissions.len(), 1);
            let result = &submissions[0];
            assert_eq!(
                (
                    &*result.id,
                    &*result.problem_id,
                    &*result.status,
                    &*result.time
                ),
                ("42", "abc100_a", "AC", "1 ms")
            );
        }
        let pending = row.replace("<td>AC</td><td>1 ms</td><td>32 KB</td>", "<td>WJ</td>");
        let document =
            Html::parse_document(&format!("<table><tbody><tr>{pending}</tr></tbody></table>"));
        assert!(
            parse_submissions(&document, &page).unwrap()[0]
                .time
                .is_empty()
        );
        assert!(
            parse_submissions(
                &Html::parse_document("<select name='f.Task'></select>"),
                &page
            )
            .unwrap()
            .is_empty()
        );
        assert!(parse_submissions(&Html::parse_document("<h2>Sign In</h2>"), &page).is_err());
        let document = Html::parse_document(
            "<form action='/contests/abc100/submit'><input type=hidden name=csrf_token value='test-token'><div data-name='data.LanguageId'><select><option value='1'>Ruby</option></select></div></form>",
        );
        let form = AtCoderBackend::submission_form(&document).unwrap();
        assert_eq!(
            form_fields(form).unwrap(),
            vec![("csrf_token".into(), "test-token".into())]
        );
        assert_eq!(
            form_action(form, &page).unwrap().path(),
            "/contests/abc100/submit"
        );
        let document = Html::parse_document("<form action='https://example.com/submit'></form>");
        assert!(form_action(required(&document, "form").unwrap(), &page).is_err());
    }
}
