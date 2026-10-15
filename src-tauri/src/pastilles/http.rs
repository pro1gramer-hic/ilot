//! The pastilles' only way out: plain requests described as data (method, URL, headers,
//! JSON body), sent by the core with reqwest, never by the island's webview. A trait
//! stands between the two, so that tests replay documented answers without any network.

use crate::locale::{Text, tr};
use reqwest::Url;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// A whole request never takes longer than this; a service that hangs does not block.
const TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Answers larger than this are refused: a pastille needs a few kilobytes.
pub const MAX_BODY: usize = 2 * 1024 * 1024;
/// Resend refuses requests without a User-Agent (https://resend.com/docs/api-reference/introduction).
pub const USER_AGENT: &str = "ilot/0.1 (pastilles, lecture seule)";
/// Longest custom URL the settings accept.
const MAX_URL: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    /// Only for the two documented reads that need a body: Notion's search and Docker
    /// Hub's token exchange (tests list them).
    Post,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<Value>,
}

impl Request {
    pub fn get(url: String) -> Self {
        Self { method: Method::Get, url, headers: Vec::new(), body: None }
    }

    pub fn post(url: String, body: Value) -> Self {
        Self { method: Method::Post, url, headers: Vec::new(), body: Some(body) }
    }

    pub fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("Authorization", format!("Bearer {token}"))
    }
}

/// What came back: the status, the body, and how long a limited service asks to wait.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
    pub retry_after: Option<u64>,
}

impl Reply {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The body as JSON, when the request succeeded.
    pub fn json(&self) -> Result<Value, Failure> {
        if !self.ok() {
            return Err(Failure::BadAnswer);
        }
        serde_json::from_slice(&self.body).map_err(|_| Failure::BadAnswer)
    }

    /// The body as JSON when it succeeded, nothing otherwise (an optional request).
    pub fn json_opt(&self) -> Option<Value> {
        self.json().ok()
    }
}

/// Why a pastille could not be read. Each one is told calmly on the pastille.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// Nothing answers (service down, no network, local server not started).
    Unreachable,
    /// 401 or 403: the token was refused or lacks a right.
    Refused,
    /// 429: the service asks to wait (seconds, when it says).
    Limited(Option<u64>),
    /// 5xx: the service has a problem of its own.
    Server(u16),
    /// The answer was not what the documentation shows.
    BadAnswer,
    /// No token in the vault for this service.
    NoToken,
    /// The settings lack something (account id, URL).
    Setup(Text<'static>),
}

impl Failure {
    /// The status line shown on the pastille.
    pub fn line(&self) -> String {
        tr(match self {
            Failure::Unreachable => Text::ServiceUnreachable,
            Failure::Refused => Text::ServiceTokenRefused,
            Failure::Limited(_) => Text::ServiceLimited,
            Failure::Server(code) => Text::ServiceError { code: *code },
            Failure::BadAnswer => Text::ServiceBadAnswer,
            Failure::NoToken => Text::ServiceNoToken,
            Failure::Setup(what) => *what,
        })
    }
}

/// Maps a status to a failure; other statuses (2xx, 404…) go to the service, which decides.
pub fn failure_of(reply: &Reply) -> Option<Failure> {
    match reply.status {
        401 | 403 => Some(Failure::Refused),
        429 => Some(Failure::Limited(reply.retry_after)),
        500..=599 => Some(Failure::Server(reply.status)),
        _ => None,
    }
}

pub type Pending<'a> = Pin<Box<dyn Future<Output = Result<Reply, Failure>> + Send + 'a>>;

/// Sends one request. The app uses reqwest; tests use canned answers.
pub trait Transport: Send + Sync {
    fn send(&self, request: Request) -> Pending<'_>;
}

/// The real transport: no redirects followed (a token never leaves for another host),
/// short timeouts, a bounded body.
pub struct Http {
    client: reqwest::Client,
}

impl Http {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    async fn run(&self, request: Request) -> Result<Reply, Failure> {
        let mut builder = match request.method {
            Method::Get => self.client.get(&request.url),
            Method::Post => self.client.post(&request.url),
        };
        for (name, value) in &request.headers {
            builder = builder.header(*name, value);
        }
        if let Some(body) = &request.body {
            builder = builder.json(body);
        }
        let mut response = builder.send().await.map_err(|_| Failure::Unreachable)?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Unreachable)? {
            if body.len() + chunk.len() > MAX_BODY {
                return Err(Failure::BadAnswer);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Reply { status, body, retry_after })
    }
}

impl Transport for Http {
    fn send(&self, request: Request) -> Pending<'_> {
        Box::pin(self.run(request))
    }
}

/// Is this host the user's own machine?
pub fn is_local_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

/// A base URL typed by the user (GitLab, Dokploy, n8n…), cleaned: https, or http only on
/// this machine; no user name, password, query or fragment; no trailing slash.
pub fn base_url(text: &str) -> Result<String, &'static str> {
    const BAD: &str = "Adresse invalide : https://… (ou http://localhost)";
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_URL || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(BAD);
    }
    let url = Url::parse(text).map_err(|_| BAD)?;
    let host = url.host_str().ok_or(BAD)?;
    let secure = match url.scheme() {
        "https" => true,
        "http" => false,
        _ => return Err(BAD),
    };
    if !secure && !is_local_host(host) {
        return Err("Adresse non chiffrée : seul http://localhost est accepté, sinon https://");
    }
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err(BAD);
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

/// `base` + `path` + an encoded query. `base` is a cleaned base URL (or a fixed one).
pub fn join(base: &str, path: &str, query: &[(&str, &str)]) -> String {
    let mut url = match Url::parse(&format!("{base}{path}")) {
        Ok(url) => url,
        Err(_) => return String::new(),
    };
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_urls_are_https_or_local_http() {
        assert_eq!(base_url(" https://gitlab.example.com/ ").unwrap(), "https://gitlab.example.com");
        assert_eq!(base_url("https://ghe.example.com/api/v3").unwrap(), "https://ghe.example.com/api/v3");
        assert_eq!(base_url("http://localhost:3000").unwrap(), "http://localhost:3000");
        assert_eq!(base_url("http://127.0.0.1:5678/").unwrap(), "http://127.0.0.1:5678");
        assert!(base_url("http://dokploy.example.com").unwrap_err().contains("non chiffrée"));
        assert!(base_url("ftp://example.com").is_err());
        assert!(base_url("https://user:pass@example.com").is_err());
        assert!(base_url("https://example.com/?token=1").is_err());
        assert!(base_url("https://example.com/#x").is_err());
        assert!(base_url("https://exa mple.com").is_err());
        assert!(base_url("javascript:alert(1)").is_err());
        assert!(base_url("").is_err());
        assert!(base_url(&format!("https://{}.com", "a".repeat(200))).is_err());
    }

    #[test]
    fn queries_are_encoded() {
        assert_eq!(
            join("https://api.github.com", "/search/issues", &[("q", "is:pr is:open author:@me"), ("per_page", "5")]),
            "https://api.github.com/search/issues?q=is%3Apr+is%3Aopen+author%3A%40me&per_page=5"
        );
        assert_eq!(join("https://x.example", "/a", &[]), "https://x.example/a");
    }

    #[test]
    fn statuses_become_calm_failures() {
        let reply = |status, retry_after| Reply { status, body: Vec::new(), retry_after };
        assert_eq!(failure_of(&reply(401, None)), Some(Failure::Refused));
        assert_eq!(failure_of(&reply(403, None)), Some(Failure::Refused));
        assert_eq!(failure_of(&reply(429, Some(30))), Some(Failure::Limited(Some(30))));
        assert_eq!(failure_of(&reply(503, None)), Some(Failure::Server(503)));
        assert_eq!(failure_of(&reply(404, None)), None);
        assert_eq!(failure_of(&reply(200, None)), None);
        assert_eq!(Failure::Refused.line(), "Jeton refusé : vérifie ses droits");
        assert_eq!(Failure::Unreachable.line(), "Injoignable pour l'instant");
    }

    #[test]
    fn bodies_are_read_only_when_the_request_succeeded() {
        let good = Reply { status: 200, body: br#"{"a":1}"#.to_vec(), retry_after: None };
        assert_eq!(good.json().unwrap()["a"], 1);
        let missing = Reply { status: 404, body: br#"{"a":1}"#.to_vec(), retry_after: None };
        assert_eq!(missing.json(), Err(Failure::BadAnswer));
        assert!(missing.json_opt().is_none());
        let garbage = Reply { status: 200, body: b"<html>".to_vec(), retry_after: None };
        assert_eq!(garbage.json(), Err(Failure::BadAnswer));
    }

    #[test]
    fn failures_are_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(Failure::Server(502).line(), "The service is answering badly (error 502)");
        assert_eq!(Failure::NoToken.line(), "No token: add it in Settings");
        assert_eq!(Failure::Setup(crate::locale::Text::AccountIdMissing).line(), "Account ID missing");
    }
}
