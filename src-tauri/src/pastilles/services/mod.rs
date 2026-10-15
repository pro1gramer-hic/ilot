//! Service pastilles (GitHub, Stripe, n8n…), all optional and read-only. Each service is
//! one small file implementing `Service`: its card (`Spec`: name, Mochi colors, URL,
//! token, refresh), the requests of a reading, and how to sum their answers up. Adding a
//! service = one file here + one line in `REGISTRY` (docs/INTEGRATIONS.md).
//!
//! A reading never writes: services only plan GET requests, except the two documented
//! reads that need a body (Notion's search, Docker Hub's token exchange).

use super::http::{Failure, Reply, Request, Transport, failure_of};
use crate::locale::Text;
use serde::Serialize;
use std::time::Duration;

mod calcom;
mod cloudflare;
mod dockerhub;
mod dokploy;
mod github;
mod gitlab;
mod n8n;
mod notion;
mod resend;
mod stripe;
mod vercel;

/// Every service îlot knows, in the order of the settings' catalog.
pub static REGISTRY: [&dyn Service; 11] = [
    &github::GitHub,
    &gitlab::GitLab,
    &vercel::Vercel,
    &cloudflare::Cloudflare,
    &dokploy::Dokploy,
    &dockerhub::DockerHub,
    &stripe::Stripe,
    &n8n::N8n,
    &resend::Resend,
    &notion::Notion,
    &calcom::CalCom,
];

pub fn find(id: &str) -> Option<&'static dyn Service> {
    REGISTRY.iter().copied().find(|s| s.spec().id == id)
}

/// A reading never takes more rounds or requests than this.
const MAX_ROUNDS: usize = 3;
const MAX_REQUESTS: usize = 6;

/// Where a service lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Address {
    /// Always this one (api.stripe.com…).
    Fixed(&'static str),
    /// The user's instance; `required` when there is no sensible default.
    Custom { default: &'static str, required: bool },
}

/// An identifier a service needs besides its token (not secret).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountRule {
    /// Cloudflare's account id: 32 hexadecimal characters.
    Hex32,
    /// A user or namespace name: lowercase letters, digits, `-`, `_`, `.`.
    Name,
}

#[derive(Clone, Copy, Debug)]
pub struct Account {
    pub label: Text<'static>,
    pub hint: Text<'static>,
    pub rule: AccountRule,
}

impl AccountRule {
    /// The identifier as stored, or why it is refused.
    pub fn clean(self, text: &str) -> Result<String, &'static str> {
        let text = text.trim().to_lowercase();
        let ok = match self {
            AccountRule::Hex32 => text.len() == 32 && text.bytes().all(|b| b.is_ascii_hexdigit()),
            AccountRule::Name => {
                (2..=64).contains(&text.len())
                    && text.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
                    && !text.starts_with('.')
                    && !text.contains("..")
            }
        };
        if ok { Ok(text) } else { Err("Identifiant invalide") }
    }
}

/// A service's card: what the settings show and how the core paces it.
#[derive(Debug)]
pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    /// Mochi's palette and silhouette for this pastille (src/lib/mochi/palettes.ts, kinds.ts).
    pub palette: &'static str,
    pub kind: &'static str,
    pub address: Address,
    pub account: Option<Account>,
    /// Time between two readings (each API's limits, docs/INTEGRATIONS.md).
    pub refresh: Duration,
    pub token_label: Text<'static>,
    /// The least rights the token needs, in plain words.
    pub rights: Text<'static>,
    /// Said before adding (a key that cannot be read-only…).
    pub warning: Option<Text<'static>>,
    /// Where the user creates the token.
    pub help: &'static str,
}

/// How a pastille feels: its dot and its Mochi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    /// All is well.
    Ok,
    /// Something runs (a deployment, a pipeline, a model loaded).
    Busy,
    /// Something waits for the user (a review).
    Warn,
    /// Something failed.
    Err,
    /// Nothing to tell.
    Idle,
    /// Not running or unreachable.
    Off,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Metric {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
}

impl Metric {
    pub fn new(label: &str, value: impl ToString) -> Self {
        Self { label: label.to_owned(), value: value.to_string(), tone: None }
    }

    pub fn toned(label: &str, value: impl ToString, tone: Tone) -> Self {
        Self { label: label.to_owned(), value: value.to_string(), tone: Some(tone) }
    }
}

/// One line of a list: a title, a detail, an instant (epoch ms) the interface tells as
/// « il y a 5 min » or « dans 2 h ».
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item {
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
}

impl Item {
    pub fn new(title: impl Into<String>, detail: impl Into<String>, at: Option<i64>) -> Self {
        Self { title: clip(&title.into(), 80), detail: clip(&detail.into(), 60), at, tone: None }
    }

    pub fn toned(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }
}

/// What a reading gives: a tone, a line, a few words for the pill, numbers and a list.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Summary {
    pub tone: Tone,
    pub line: String,
    pub short: String,
    pub metrics: Vec<Metric>,
    pub items: Vec<Item>,
}

/// Everything a service needs for a reading. The token stays in the core.
pub struct Ctx<'a> {
    pub base: &'a str,
    pub token: &'a str,
    pub account: &'a str,
    pub now_ms: i64,
    /// Local time minus UTC, in minutes (for « today » and clock times).
    pub offset_min: i32,
}

const DAY_MS: i64 = 86_400_000;

impl Ctx<'_> {
    /// Last local midnight, in epoch milliseconds.
    pub fn midnight_ms(&self) -> i64 {
        let offset = self.offset_min as i64 * 60_000;
        (self.now_ms + offset).div_euclid(DAY_MS) * DAY_MS - offset
    }

    /// « 15:30 », local time of an instant.
    pub fn clock(&self, at_ms: i64) -> String {
        let local = (at_ms + self.offset_min as i64 * 60_000).rem_euclid(DAY_MS) / 60_000;
        format!("{:02}:{:02}", local / 60, local % 60)
    }
}

pub trait Service: Sync {
    fn spec(&self) -> &'static Spec;
    /// The requests of the next round, knowing the answers so far; none when done.
    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request>;
    /// Sums the answers up.
    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure>;
}

/// One reading: the rounds the service plans (bounded), then its summary. Refused tokens,
/// limits and server errors stop it at once.
pub async fn read(service: &dyn Service, transport: &dyn Transport, ctx: &Ctx<'_>) -> Result<Summary, Failure> {
    let mut answers: Vec<Reply> = Vec::new();
    for _ in 0..MAX_ROUNDS {
        let requests = service.plan(ctx, &answers);
        if requests.is_empty() {
            break;
        }
        for request in requests {
            if answers.len() >= MAX_REQUESTS {
                break;
            }
            let reply = transport.send(request).await?;
            if let Some(failure) = failure_of(&reply) {
                return Err(failure);
            }
            answers.push(reply);
        }
    }
    service.summarize(ctx, &answers)
}

// ---------------------------------------------------------------------------------------
// Small helpers shared by the services

/// At most `max` characters, with an ellipsis when cut.
pub fn clip(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// A count read from a list that may have more: « 100+ ».
pub fn count_label(n: usize, more: bool) -> String {
    if more { format!("{n}+") } else { n.to_string() }
}

/// Epoch milliseconds of an RFC 3339 date (« 2025-10-17T16:47:07.93355-07:00 », « …Z »).
pub fn parse_time(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't' | b' ') || b[13] != b':' {
        return None;
    }
    let num = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, s) = (num(0..4)?, num(5..7)?, num(8..10)?, num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let mut rest = &text[19..];
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        let ms: String = fraction[..digits].chars().chain("000".chars()).take(3).collect();
        millis = ms.parse::<i64>().ok()?;
        rest = &fraction[digits..];
    }
    // « Z », « +02:00 », and the short « +00 » / « +0200 » some APIs write (Resend).
    let offset_min = match rest {
        "Z" | "z" | "" => 0,
        zone if zone.starts_with('+') || zone.starts_with('-') => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let digits: String = zone[1..].chars().filter(|c| *c != ':').collect();
            if !matches!(digits.len(), 2 | 4) || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let minutes = if digits.len() == 4 { digits[2..].parse::<i64>().ok()? } else { 0 };
            sign * (digits[..2].parse::<i64>().ok()? * 60 + minutes)
        }
        _ => return None,
    };
    let days = days_from_civil(y, mo, d);
    Some(((days * 24 + h) * 60 + mi - offset_min) * 60_000 + s * 1000 + millis)
}

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// RFC 3339 (UTC) of an instant, for APIs that filter by date.
pub fn rfc3339(ms: i64) -> String {
    let days = ms.div_euclid(DAY_MS);
    let in_day = ms.rem_euclid(DAY_MS) / 1000;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        in_day / 3600,
        in_day % 3600 / 60,
        in_day % 60
    )
}

/// A string field, empty when missing.
pub fn text<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value.get(key).and_then(serde_json::Value::as_str).unwrap_or("")
}

#[cfg(test)]
pub mod testing {
    //! A transport that answers from a script and remembers what was asked: no network.
    use super::super::http::{Failure, Pending, Reply, Request, Transport};
    use std::sync::Mutex;

    pub struct Script {
        answers: Mutex<Vec<Result<Reply, Failure>>>,
        pub asked: Mutex<Vec<Request>>,
    }

    impl Script {
        pub fn new(answers: Vec<Result<Reply, Failure>>) -> Self {
            Self { answers: Mutex::new(answers), asked: Mutex::new(Vec::new()) }
        }

        pub fn asked(&self) -> Vec<Request> {
            self.asked.lock().unwrap().clone()
        }
    }

    impl Transport for Script {
        fn send(&self, request: Request) -> Pending<'_> {
            self.asked.lock().unwrap().push(request);
            let mut answers = self.answers.lock().unwrap();
            let answer = if answers.is_empty() { Err(Failure::Unreachable) } else { answers.remove(0) };
            Box::pin(async move { answer })
        }
    }

    pub fn ok(json: &str) -> Result<Reply, Failure> {
        Ok(Reply { status: 200, body: json.as_bytes().to_vec(), retry_after: None })
    }

    pub fn status(code: u16) -> Result<Reply, Failure> {
        Ok(Reply { status: code, body: Vec::new(), retry_after: if code == 429 { Some(60) } else { None } })
    }

    /// 2026-10-11 12:00 UTC, in Paris (UTC+2).
    pub const NOW: i64 = 1_791_720_000_000;

    pub fn ctx<'a>(base: &'a str, account: &'a str) -> super::Ctx<'a> {
        super::Ctx { base, token: "jeton-de-test", account, now_ms: NOW, offset_min: 120 }
    }

    /// Runs a reading to the end on a small runtime.
    pub fn run(service: &dyn super::Service, script: &Script, ctx: &super::Ctx) -> Result<super::Summary, Failure> {
        tauri::async_runtime::block_on(super::read(service, script, ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::Method;
    use super::testing::*;
    use super::*;

    #[test]
    fn every_service_has_a_unique_id_and_a_calm_refresh() {
        let mut ids: Vec<&str> = REGISTRY.iter().map(|s| s.spec().id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), REGISTRY.len());
        for service in REGISTRY {
            let spec = service.spec();
            assert!(spec.refresh >= Duration::from_secs(120), "{} refreshes too often", spec.id);
            assert!(spec.refresh <= Duration::from_secs(3600), "{} refreshes too rarely", spec.id);
            assert!(["lavande", "peche", "menthe", "ciel", "citron", "nuit"].contains(&spec.palette));
            assert!(["meche", "ecran", "oreilles"].contains(&spec.kind));
            assert!(spec.help.starts_with("https://"), "{}", spec.id);
            assert!(!crate::locale::tr(spec.rights).is_empty());
        }
        // Stripe's monthly read quota (docs.stripe.com/rate-limits): ten minutes at least.
        assert!(find("stripe").unwrap().spec().refresh >= Duration::from_secs(600));
    }

    #[test]
    fn readings_never_write() {
        // The only POSTs are documented reads: Notion's search and Docker Hub's token.
        let answers: Vec<Reply> = Vec::new();
        for service in REGISTRY {
            let ctx = ctx("https://instance.example", "0123456789abcdef0123456789abcdef");
            for request in service.plan(&ctx, &answers) {
                let read_with_body = request.url.ends_with("/v1/search") || request.url.ends_with("/v2/auth/token");
                assert!(
                    request.method == Method::Get || read_with_body,
                    "{} plans {:?} {}",
                    service.spec().id,
                    request.method,
                    request.url
                );
                assert!(request.url.starts_with("https://"), "{}", request.url);
            }
        }
    }

    #[test]
    fn the_token_only_goes_to_the_service() {
        for service in REGISTRY {
            let ctx = ctx("https://instance.example", "0123456789abcdef0123456789abcdef");
            for request in service.plan(&ctx, &[]) {
                assert!(!request.url.contains("jeton-de-test"), "token in the URL of {}", service.spec().id);
            }
        }
    }

    #[test]
    fn local_midnight_and_clock() {
        let ctx = ctx("", "");
        // 12:00 UTC = 14:00 in Paris; midnight in Paris = 22:00 UTC the day before.
        assert_eq!(ctx.clock(NOW), "14:00");
        assert_eq!(ctx.midnight_ms(), NOW - 14 * 3_600_000);
    }

    #[test]
    fn dates_parse_from_rfc3339() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_time("2026-10-11T12:00:00Z"), Some(NOW));
        assert_eq!(parse_time("2026-10-11T14:00:00+02:00"), Some(NOW));
        assert_eq!(parse_time("2025-10-17T16:47:07.93355-07:00"), parse_time("2025-10-17T23:47:07.933Z"));
        assert_eq!(parse_time("2026-10-11T12:00:00.5Z"), Some(NOW + 500));
        assert_eq!(parse_time("2026-10-11 12:00:00.674981+00"), Some(NOW + 674));
        assert_eq!(parse_time("2026-10-11T14:00:00+0200"), Some(NOW));
        assert_eq!(parse_time("2026-10-11T12:00:00+2"), None);
        assert_eq!(parse_time("hier"), None);
        assert_eq!(parse_time("2026-13-11T12:00:00Z"), None);
        assert_eq!(rfc3339(NOW), "2026-10-11T12:00:00Z");
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(parse_time(&rfc3339(1_234_567_890_000)), Some(1_234_567_890_000));
    }

    #[test]
    fn texts_are_clipped_and_counted() {
        assert_eq!(clip("  court  ", 10), "court");
        assert_eq!(clip("un titre beaucoup trop long", 10), "un titre…");
        assert_eq!("1 PR ouverte", Text::GithubOpenPrs { n: 1 });
        assert_eq!("3 PR ouvertes", Text::GithubOpenPrs { n: 3 });
        assert_eq!(count_label(100, true), "100+");
        assert_eq!(count_label(4, false), "4");
    }

    #[test]
    fn account_ids_are_checked() {
        assert!(AccountRule::Hex32.clean("0123456789ABCDEF0123456789abcdef").is_ok());
        assert!(AccountRule::Hex32.clean("0123").is_err());
        assert_eq!(AccountRule::Name.clean(" acme ").unwrap(), "acme");
        assert!(AccountRule::Name.clean("../etc").is_err());
        assert!(AccountRule::Name.clean("a/b").is_err());
        assert!(AccountRule::Name.clean("x").is_err());
    }

    struct Probe;
    static PROBE: Spec = Spec {
        id: "probe",
        name: "Probe",
        palette: "lavande",
        kind: "meche",
        address: Address::Fixed("https://probe.example"),
        account: None,
        refresh: Duration::from_secs(600),
        token_label: Text::GithubTokenLabel,
        rights: Text::GithubRights,
        warning: None,
        help: "https://probe.example",
    };

    impl Service for Probe {
        fn spec(&self) -> &'static Spec {
            &PROBE
        }

        fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
            // Asks forever: the engine must stop it.
            vec![Request::get(format!("{}/{}", ctx.base, answers.len()))]
        }

        fn summarize(&self, _: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
            Ok(Summary { tone: Tone::Ok, line: answers.len().to_string(), short: String::new(), metrics: vec![], items: vec![] })
        }
    }

    #[test]
    fn a_reading_is_bounded() {
        let script = Script::new((0..10).map(|_| ok("{}")).collect());
        let summary = run(&Probe, &script, &ctx("https://probe.example", "")).unwrap();
        assert_eq!(summary.line, MAX_ROUNDS.to_string());
        assert_eq!(script.asked().len(), MAX_ROUNDS);
    }

    #[test]
    fn refusals_limits_and_outages_stop_a_reading() {
        for (answer, failure) in [
            (status(401), Failure::Refused),
            (status(403), Failure::Refused),
            (status(429), Failure::Limited(Some(60))),
            (status(502), Failure::Server(502)),
            (Err(Failure::Unreachable), Failure::Unreachable),
        ] {
            let script = Script::new(vec![answer]);
            assert_eq!(run(&Probe, &script, &ctx("https://probe.example", "")), Err(failure));
            assert_eq!(script.asked().len(), 1);
        }
    }
}
