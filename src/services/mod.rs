mod atcoder;
#[cfg(feature = "mock")]
mod mock;
mod problems;
mod yukicoder;

use self::{
    atcoder::AtCoderBackend, problems::AtCoderProblemsBackend, yukicoder::YukicoderBackend,
};
use crate::{
    config::{Paths, expand_path},
    model::*,
};
use anyhow::{Context, Result, ensure};
use reqwest::{blocking::Client, cookie::Jar};
use scraper::{ElementRef, Html, Selector};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, ErrorKind, Write},
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use url::Url;

pub trait ServiceBackend {
    fn service(&self) -> ServiceId;
    fn auth_service(&self) -> ServiceId;
    fn check_auth(&self) -> Result<()>;
    fn resolve_url(&self, url: &Url) -> Result<ResourceRef>;
    fn fetch_problem(&self, problem: &ProblemRef) -> Result<Problem>;
    fn fetch_contest(&self, contest: &ContestRef) -> Result<Contest>;
    fn languages(&self, problem: &ProblemRef) -> Result<Vec<SubmissionLanguage>>;
    fn submit(&self, request: &SubmissionRequest<'_>) -> Result<Submission>;
    fn submissions(&self, scope: &SubmissionScope, limit: usize) -> Result<Vec<Submission>>;
}

pub struct Services {
    atcoder: AtCoderBackend,
    problems: AtCoderProblemsBackend,
    yukicoder: YukicoderBackend,
    #[cfg(feature = "mock")]
    mock: mock::MockBackend,
}

impl Services {
    pub fn new(paths: &Paths) -> Result<Self> {
        let atcoder = AtCoderBackend {
            http: Http::new(&paths.cookies.join("atcoder.txt"), ServiceId::Atcoder)?,
        };
        let yukicoder = YukicoderBackend {
            http: Http::new(&paths.cookies.join("yukicoder.txt"), ServiceId::Yukicoder)?,
        };
        Ok(Self {
            problems: AtCoderProblemsBackend {
                atcoder: atcoder.clone(),
            },
            atcoder,
            yukicoder,
            #[cfg(feature = "mock")]
            mock: mock::MockBackend {
                cookies: cookie_jar(
                    &read_cookies(&paths.cookies.join("mock.txt"))?,
                    "mock.local",
                )?,
            },
        })
    }

    pub fn backend(&self, service: ServiceId) -> &dyn ServiceBackend {
        match service {
            ServiceId::Atcoder => &self.atcoder,
            ServiceId::AtcoderProblems => &self.problems,
            ServiceId::Yukicoder => &self.yukicoder,
            #[cfg(feature = "mock")]
            ServiceId::Mock => &self.mock,
        }
    }

    pub fn resolve(&self, url: &Url) -> Result<ResourceRef> {
        let backend = self.backend(ServiceId::from_url(url)?);
        tracing::info!("Resolving {url}...");
        backend.resolve_url(url)
    }

    pub fn login(paths: &Paths, service: ServiceId, source: &Path) -> Result<()> {
        let source = expand_path(source)?;
        let raw = fs::read(&source).with_context(|| format!("Cannot read {}", source.display()))?;
        let auth_service = match service {
            ServiceId::AtcoderProblems => ServiceId::Atcoder,
            s => s,
        };
        tracing::info!(
            "Checking authentication for {} using {}...",
            auth_service.as_str(),
            source.display()
        );
        match auth_service {
            ServiceId::Atcoder => AtCoderBackend {
                http: Http::from_cookies(&raw, auth_service)?,
            }
            .check_auth()?,
            ServiceId::Yukicoder => YukicoderBackend {
                http: Http::from_cookies(&raw, auth_service)?,
            }
            .check_auth()?,
            ServiceId::AtcoderProblems => unreachable!(),
            #[cfg(feature = "mock")]
            ServiceId::Mock => mock::MockBackend {
                cookies: cookie_jar(&raw, "mock.local")?,
            }
            .check_auth()?,
        }
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&paths.cookies)?;
        fs::set_permissions(&paths.cookies, fs::Permissions::from_mode(0o700))?;
        let mut staging = tempfile::NamedTempFile::new_in(&paths.cookies)?;
        staging
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
        staging.write_all(&raw)?;
        staging.as_file().sync_all()?;
        staging.persist(paths.cookies.join(format!("{}.txt", auth_service.as_str())))?;
        tracing::info!("Logged in to {}", auth_service.as_str());
        Ok(())
    }
}

fn read_cookies(path: &Path) -> Result<Vec<u8>> {
    match fs::read(path) {
        Ok(raw) => Ok(raw),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

fn cookie_jar(raw: &[u8], host: &str) -> Result<Arc<Jar>> {
    let jar = Arc::new(Jar::default());
    for record in netscape_cookie_file_parser::parse(Cursor::new(raw))
        .context("Invalid Netscape cookie file")?
    {
        let domain = String::from_utf8(record.domain).context("Cookie domain must be UTF-8")?;
        let origin_host = domain.trim_start_matches('.');
        if origin_host != host && !origin_host.ends_with(&format!(".{host}")) {
            continue;
        }
        let mut cookie = cookie::Cookie::build((
            String::from_utf8(record.name)?,
            String::from_utf8(record.value)?,
        ))
        .path(String::from_utf8(record.path)?)
        .secure(record.secure)
        .http_only(record.http_only);
        if record.tail_match {
            cookie = cookie.domain(domain.clone());
        }
        if record.expires != 0 {
            cookie = cookie.expires(cookie::time::OffsetDateTime::from_unix_timestamp(
                record.expires.try_into()?,
            )?);
        }
        jar.add_cookie_str(
            &cookie.build().to_string(),
            &Url::parse(&format!("https://{origin_host}/"))?,
        );
    }
    Ok(jar)
}

#[derive(Clone)]
pub(super) struct Http {
    client: Client,
    accessed: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Http {
    fn new(path: &Path, service: ServiceId) -> Result<Self> {
        let raw = read_cookies(path)?;
        Self::from_cookies(&raw, service)
    }

    fn from_cookies(raw: &[u8], service: ServiceId) -> Result<Self> {
        let host = match service {
            ServiceId::Atcoder | ServiceId::AtcoderProblems => "atcoder.jp",
            ServiceId::Yukicoder => "yukicoder.me",
            #[cfg(feature = "mock")]
            ServiceId::Mock => anyhow::bail!("Mock services do not use HTTP"),
        };
        let jar = cookie_jar(raw, host)?;
        let client = Client::builder()
            .cookie_provider(jar)
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("cpcli/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    return attempt.error("Too many redirects");
                }
                let Some(previous) = attempt.previous().first() else {
                    return attempt.error("Missing redirect origin");
                };
                if attempt.url().scheme() != previous.scheme()
                    || attempt.url().host_str() != previous.host_str()
                    || attempt.url().port_or_known_default() != previous.port_or_known_default()
                {
                    return attempt.error("Judge redirected to a different origin");
                }
                attempt.follow()
            }))
            .build()?;
        Ok(Self {
            client,
            accessed: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn request_url(&self, url: &Url) -> Result<Url> {
        ServiceId::from_url(url)?;
        let host = url.host_str().context("Judge URL has no host")?;
        let mut accessed = self.accessed.lock().expect("HTTP throttle lock");
        if let Some(last) = accessed.get(host) {
            let interval = Duration::from_millis(1100);
            if last.elapsed() < interval {
                thread::sleep(interval - last.elapsed());
            }
        }
        accessed.insert(host.into(), Instant::now());
        Ok(url.clone())
    }

    fn response(
        &self,
        original: &Url,
        response: reqwest::blocking::Response,
    ) -> Result<(Url, String)> {
        let response = response
            .error_for_status()
            .with_context(|| format!("Request failed: {original}"))?;
        let url = response.url().clone();
        ensure!(
            !url.path().starts_with("/login") && !url.path().starts_with("/auth/"),
            "Session expired; import fresh cookies with cpcli login"
        );
        Ok((url, response.text()?))
    }

    pub fn get(&self, url: &Url) -> Result<(Url, Html)> {
        let response = self.client.get(self.request_url(url)?).send()?;
        let (url, text) = self.response(url, response)?;
        Ok((url, Html::parse_document(&text)))
    }

    pub fn json<T: DeserializeOwned>(&self, url: &Url) -> Result<T> {
        let response = self.client.get(self.request_url(url)?).send()?;
        let (_, text) = self.response(url, response)?;
        serde_json::from_str(&text).with_context(|| format!("Invalid response from {url}"))
    }

    pub fn post(
        &self,
        url: &Url,
        fields: Vec<(String, String)>,
        source_file: Option<&str>,
        csrf: Option<&str>,
    ) -> Result<(Url, Html)> {
        let mut request = self
            .client
            .post(self.request_url(url)?)
            .header(reqwest::header::REFERER, url.as_str());
        if let Some(csrf) = csrf {
            request = request.header("X-CSRFToken", csrf);
        }
        request = if let Some(source) = source_file {
            let mut form = reqwest::blocking::multipart::Form::new();
            for (name, value) in fields {
                form = form.text(name, value);
            }
            let file =
                reqwest::blocking::multipart::Part::text(source.to_owned()).file_name("solution");
            request.multipart(form.part("file", file))
        } else {
            request.form(&fields)
        };
        let response = request
            .send()
            .context("Submission outcome is unknown; check results before submitting again")?;
        let (url, body) = self.response(url, response)?;
        Ok((url, Html::parse_document(&body)))
    }
}

pub(super) fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("valid built-in CSS selector")
}
pub(super) fn text(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}
pub(super) fn required<'a>(document: &'a Html, css: &str) -> Result<ElementRef<'a>> {
    document
        .select(&selector(css))
        .next()
        .with_context(|| format!("Page format changed: missing {css}"))
}

pub(super) fn pre_text(element: ElementRef<'_>) -> String {
    fn append(element: ElementRef<'_>, output: &mut String) {
        for node in element.children() {
            match node.value() {
                scraper::Node::Text(t) => output.push_str(t),
                scraper::Node::Element(e) if e.name() == "br" => output.push('\n'),
                scraper::Node::Element(_) => {
                    append(ElementRef::wrap(node).expect("element node"), output)
                }
                _ => (),
            }
        }
    }
    let mut output = String::new();
    append(element, &mut output);
    output
}

pub(super) fn form_fields(form: ElementRef<'_>) -> Result<Vec<(String, String)>> {
    form.select(&selector("input[type=hidden][name]"))
        .map(|input| {
            Ok((
                input
                    .value()
                    .attr("name")
                    .expect("selected named input")
                    .to_owned(),
                input
                    .value()
                    .attr("value")
                    .context("Hidden form input has no value")?
                    .to_owned(),
            ))
        })
        .collect()
}

pub(super) fn form_action(form: ElementRef<'_>, page: &Url) -> Result<Url> {
    let action = match form.value().attr("action") {
        Some(action) => page.join(action)?,
        None => page.clone(),
    };
    ensure!(
        action.origin() == page.origin(),
        "Submission form points to a different origin"
    );
    Ok(action)
}

pub(super) fn next_page(document: &Html, current: &Url, page: usize) -> Result<bool> {
    for link in document.select(&selector("a[href]")) {
        let target = current.join(link.value().attr("href").expect("selected href"))?;
        if target.path() == current.path()
            && target
                .query_pairs()
                .any(|(key, value)| key == "page" && value == (page + 1).to_string())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn sort_submissions(submissions: &mut Vec<Submission>, limit: usize) {
    submissions.sort_by(|a, b| {
        b.submitted_at
            .cmp(&a.submitted_at)
            .then_with(|| b.id.len().cmp(&a.id.len()))
            .then_with(|| b.id.cmp(&a.id))
    });
    submissions.dedup_by(|a, b| a.url == b.url);
    submissions.truncate(limit);
}
