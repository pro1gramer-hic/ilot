//! DeepL (prompt 12): a text the user chose (a message received, an answer of the chat,
//! the clipboard) leaves for DeepL with the user's own key, only when they click
//! « Traduire ». The key lives in the vault (`service:deepl`) and never reaches a window;
//! the request leaves from here, never from the webview, whose CSP keeps `connect-src`
//! local. Nothing is written to disk or logged: the text and its translation stay in the
//! island's memory.
//!
//! - `POST /v2/translate`, JSON body `{ "text": [...], "target_lang": ... }`, the source
//!   language detected by DeepL (`detected_source_language`); errors 403, 413, 429, 456,
//!   5xx: https://developers.deepl.com/api-reference/translate
//! - `Authorization: DeepL-Auth-Key <key>`; a key of DeepL API Free ends with `:fx` and goes
//!   to `api-free.deepl.com`, any other to `api.deepl.com`:
//!   https://developers.deepl.com/docs/getting-started/auth
//! - 128 KiB per request: https://developers.deepl.com/docs/resources/usage-limits
//! - Target languages: https://developers.deepl.com/docs/getting-started/supported-languages
//!
//! The core has no catalog of texts: a failure is a code (`Failure`) that the interface says
//! in its language (src/lib/i18n/<lang>/translate.ts).

use crate::vault;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// The vault entry of the user's DeepL key.
pub const SECRET: &str = "service:deepl";
/// Hosts written here, HTTPS only: the interface never chooses an address.
pub const FREE_ENDPOINT: &str = "https://api-free.deepl.com/v2/translate";
pub const PRO_ENDPOINT: &str = "https://api.deepl.com/v2/translate";
/// What ends a DeepL API Free key.
const FREE_SUFFIX: &str = ":fx";
/// The longest text sent at once: 30 000 characters stay under DeepL's 128 KiB per request
/// even when each one takes 4 bytes.
pub const MAX_TEXT_CHARS: usize = 30_000;
/// A translation larger than this is refused (a text of 30 000 characters comes back in
/// far less).
const MAX_ANSWER_BYTES: usize = 512 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// The detected language as DeepL writes it (`EN`, `PT`, `ZH`): short letters only.
const MAX_LANGUAGE_CHARS: usize = 8;

/// Every target language DeepL lists for text (supported-languages, read on 2026-10-12).
/// The interface offers the same list (src/lib/translate.ts, checked by a test).
pub const TARGETS: [&str; 126] = [
    "ACE", "AF", "SQ", "AR", "AN", "HY", "AS", "AY", "AZ", "BA", "EU", "BE", "BN", "BHO", "BS", "BR", "BG", "MY", "YUE",
    "CA", "CEB", "ZH-HANS", "ZH-HANT", "ZH", "HR", "CS", "DA", "PRS", "NL", "EN", "EN-US", "EN-GB", "EO", "ET", "FI",
    "FR", "FR-CA", "FR-FR", "GL", "KA", "DE", "DE-DE", "DE-CH", "EL", "GN", "GU", "HT", "HA", "HE", "HI", "HU", "IS",
    "IG", "ID", "GA", "IT", "JA", "JV", "PAM", "KK", "GOM", "KO", "KMR", "CKB", "KY", "LA", "LV", "LN", "LT", "LMO",
    "LB", "MK", "MAI", "MG", "MS", "ML", "MT", "MI", "MR", "MN", "NE", "NB", "OC", "OM", "PAG", "PS", "FA", "PL",
    "PT-BR", "PT-PT", "PT", "PA", "QU", "RO", "RU", "SA", "SR", "ST", "SCN", "SK", "SL", "ES", "ES-ES", "ES-419", "SU",
    "SW", "SV", "TL", "TG", "TA", "TT", "TE", "TH", "TS", "TN", "TR", "TK", "UK", "UR", "UZ", "VI", "CY", "WO", "XH",
    "YI", "ZU",
];

/// Why there is no translation. The interface says each one in its language; none of them
/// carries the key or the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Failure {
    /// No key in the vault: nothing was sent.
    NoKey,
    /// 403 (or 401): DeepL refused the key.
    KeyRefused,
    /// 456: the month's characters are used up.
    Quota,
    /// 429 or 529: too many requests for now.
    Busy,
    /// 5xx: DeepL has a problem of its own.
    Unavailable,
    /// No answer at all (network, timeout).
    Network,
    /// Longer than `MAX_TEXT_CHARS` (or 413).
    TooLong,
    /// Nothing to translate.
    Empty,
    /// A target language DeepL does not offer.
    BadLanguage,
    /// Any other answer.
    Unexpected,
}

/// A translation and the language DeepL recognized (`EN`, `FR`…; empty when it did not say).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    pub text: String,
    pub detected: String,
}

/// The language the user picked in Réglages › Chat; empty follows the interface's language.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranslateSettings {
    pub target: String,
}

impl TranslateSettings {
    /// A language DeepL does not offer goes back to the interface's.
    pub fn sanitized(self) -> Self {
        Self { target: if valid_target(&self.target) { self.target } else { String::new() } }
    }
}

pub fn valid_target(target: &str) -> bool {
    TARGETS.contains(&target)
}

/// DeepL API Free keys end with `:fx` and have their own host.
pub fn endpoint(key: &str) -> &'static str {
    if key.trim().ends_with(FREE_SUFFIX) { FREE_ENDPOINT } else { PRO_ENDPOINT }
}

/// The text as it goes: trimmed, not empty, not too long.
pub fn checked_text(text: &str) -> Result<&str, Failure> {
    let text = text.trim();
    if text.is_empty() {
        Err(Failure::Empty)
    } else if text.chars().count() > MAX_TEXT_CHARS {
        Err(Failure::TooLong)
    } else {
        Ok(text)
    }
}

/// The JSON body: one text, the target language; DeepL detects the source.
pub fn body(text: &str, target: &str) -> Value {
    json!({ "text": [text], "target_lang": target })
}

/// One request, as it would leave: the host chosen by the key, the key in a header only.
pub struct Request {
    pub url: &'static str,
    /// `DeepL-Auth-Key <key>`, zeroed when the request is dropped.
    pub authorization: String,
    pub body: String,
}

impl Drop for Request {
    fn drop(&mut self) {
        vault::wipe_string(std::mem::take(&mut self.authorization));
    }
}

/// A status DeepL documents as a failure; `None` for a success.
pub fn failure_of(status: u16) -> Option<Failure> {
    match status {
        200..=299 => None,
        401 | 403 => Some(Failure::KeyRefused),
        413 => Some(Failure::TooLong),
        429 | 529 => Some(Failure::Busy),
        456 => Some(Failure::Quota),
        500..=599 => Some(Failure::Unavailable),
        _ => Some(Failure::Unexpected),
    }
}

/// The first translation of a successful answer.
pub fn translation_of(answer: &[u8]) -> Result<Translation, Failure> {
    let value: Value = serde_json::from_slice(answer).map_err(|_| Failure::Unexpected)?;
    let first = &value["translations"][0];
    let text = first["text"].as_str().ok_or(Failure::Unexpected)?.to_owned();
    let detected = first["detected_source_language"].as_str().unwrap_or_default();
    let detected = if !detected.is_empty()
        && detected.len() <= MAX_LANGUAGE_CHARS
        && detected.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
    {
        detected.to_ascii_uppercase()
    } else {
        String::new()
    };
    Ok(Translation { text, detected })
}

pub type Pending<'a> = Pin<Box<dyn Future<Output = Result<(u16, Vec<u8>), Failure>> + Send + 'a>>;

/// Sends a request: reqwest in the app, a recorder in the tests.
pub trait Transport: Send + Sync {
    fn post<'a>(&'a self, request: &'a Request) -> Pending<'a>;
}

/// Translates `text` into `target` with `key`. Without a key, or with a text or a language
/// that cannot go, nothing is sent.
pub async fn translate(transport: &dyn Transport, key: Option<&str>, text: &str, target: &str) -> Result<Translation, Failure> {
    if !valid_target(target) {
        return Err(Failure::BadLanguage);
    }
    let text = checked_text(text)?;
    let key = key.map(str::trim).filter(|k| !k.is_empty()).ok_or(Failure::NoKey)?;
    let request = Request { url: endpoint(key), authorization: format!("DeepL-Auth-Key {key}"), body: body(text, target).to_string() };
    let (status, answer) = transport.post(&request).await?;
    match failure_of(status) {
        Some(failure) => Err(failure),
        None => translation_of(&answer),
    }
}

/// The real transport: HTTPS only, no redirect followed (the key never leaves for another
/// host), short timeouts, a bounded answer.
struct Http;

impl Http {
    async fn send(request: &Request) -> Result<(u16, Vec<u8>), Failure> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(concat!("ilot/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Failure::Network)?;
        let mut authorization = reqwest::header::HeaderValue::from_str(&request.authorization).map_err(|_| Failure::KeyRefused)?;
        // The key stays out of any debug output of the client.
        authorization.set_sensitive(true);
        let mut response = client
            .post(request.url)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request.body.clone())
            .send()
            .await
            .map_err(|_| Failure::Network)?;
        let status = response.status().as_u16();
        let mut answer = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Network)? {
            if answer.len() + chunk.len() > MAX_ANSWER_BYTES {
                return Err(Failure::Unexpected);
            }
            answer.extend_from_slice(&chunk);
        }
        Ok((status, answer))
    }
}

impl Transport for Http {
    fn post<'a>(&'a self, request: &'a Request) -> Pending<'a> {
        Box::pin(Self::send(request))
    }
}

/// « Traduire »: the text the user clicked on, into the language the interface resolved.
/// The key is read from the vault here and zeroed once the request is gone.
#[tauri::command]
pub async fn translate_text(text: String, target: String) -> Result<Translation, Failure> {
    let key = vault::read(SECRET).ok().flatten();
    let result = translate(&Http, key.as_deref(), &text, &target).await;
    if let Some(key) = key {
        vault::wipe_string(key);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records what would have gone (address, header, body) and answers as DeepL would.
    struct Recorder {
        sent: Mutex<Vec<(String, String, String)>>,
        reply: Result<(u16, Vec<u8>), Failure>,
    }

    impl Recorder {
        fn replying(status: u16, body: &str) -> Self {
            Self { sent: Mutex::new(Vec::new()), reply: Ok((status, body.as_bytes().to_vec())) }
        }
    }

    impl Transport for Recorder {
        fn post<'a>(&'a self, request: &'a Request) -> Pending<'a> {
            self.sent.lock().unwrap().push((request.url.to_owned(), request.authorization.clone(), request.body.clone()));
            let reply = self.reply.clone();
            Box::pin(async move { reply })
        }
    }

    fn run<T>(future: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(future)
    }

    /// The example answer of the documentation (api-reference/translate).
    const HALLO: &str = r#"{"translations":[{"detected_source_language":"EN","text":"Hallo, Welt!"}]}"#;

    #[test]
    fn free_keys_go_to_the_free_host_and_others_to_the_pro_one() {
        assert_eq!(endpoint("279a2e9d-83b3-c416-7e2d-f721593e42a0:fx"), FREE_ENDPOINT);
        assert_eq!(endpoint(" 279a2e9d-83b3-c416-7e2d-f721593e42a0:fx\n"), FREE_ENDPOINT);
        assert_eq!(endpoint("279a2e9d-83b3-c416-7e2d-f721593e42a0"), PRO_ENDPOINT);
        assert_eq!(endpoint("fx:279a2e9d"), PRO_ENDPOINT, "the suffix, not anywhere");
        assert!(FREE_ENDPOINT.starts_with("https://api-free.deepl.com/"));
        assert!(PRO_ENDPOINT.starts_with("https://api.deepl.com/"));
    }

    #[test]
    fn without_a_key_nothing_is_sent() {
        let recorder = Recorder::replying(200, HALLO);
        assert_eq!(run(translate(&recorder, None, "Hello, World!", "DE")), Err(Failure::NoKey));
        assert_eq!(run(translate(&recorder, Some("  "), "Hello, World!", "DE")), Err(Failure::NoKey));
        assert!(recorder.sent.lock().unwrap().is_empty());
    }

    #[test]
    fn empty_long_texts_and_unknown_languages_never_leave() {
        let recorder = Recorder::replying(200, HALLO);
        assert_eq!(run(translate(&recorder, Some("k:fx"), "  \n ", "DE")), Err(Failure::Empty));
        let long = "a".repeat(MAX_TEXT_CHARS + 1);
        assert_eq!(run(translate(&recorder, Some("k:fx"), &long, "DE")), Err(Failure::TooLong));
        assert_eq!(run(translate(&recorder, Some("k:fx"), "Hello", "de")), Err(Failure::BadLanguage));
        assert_eq!(run(translate(&recorder, Some("k:fx"), "Hello", "KLINGON")), Err(Failure::BadLanguage));
        assert_eq!(run(translate(&recorder, Some("k:fx"), "Hello", "\"DE\",\"x\":1")), Err(Failure::BadLanguage));
        assert!(recorder.sent.lock().unwrap().is_empty());
        assert!(checked_text(&"é".repeat(MAX_TEXT_CHARS)).is_ok(), "characters, not bytes");
    }

    #[test]
    fn the_text_goes_in_the_body_and_the_key_in_the_header_only() {
        let recorder = Recorder::replying(200, HALLO);
        let key = "279a2e9d-83b3-c416-7e2d-f721593e42a0:fx";
        let translation = run(translate(&recorder, Some(key), "  Hello, World! ", "DE")).unwrap();
        assert_eq!(translation, Translation { text: "Hallo, Welt!".into(), detected: "EN".into() });
        let sent = recorder.sent.lock().unwrap();
        let (url, authorization, body) = &sent[0];
        assert_eq!(url, FREE_ENDPOINT);
        assert_eq!(authorization, &format!("DeepL-Auth-Key {key}"));
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body, json!({ "text": ["Hello, World!"], "target_lang": "DE" }));
        assert!(!body.to_string().contains(key));
    }

    #[test]
    fn documented_statuses_become_failures() {
        assert_eq!(failure_of(200), None);
        assert_eq!(failure_of(403), Some(Failure::KeyRefused));
        assert_eq!(failure_of(401), Some(Failure::KeyRefused));
        assert_eq!(failure_of(413), Some(Failure::TooLong));
        assert_eq!(failure_of(429), Some(Failure::Busy));
        assert_eq!(failure_of(529), Some(Failure::Busy));
        assert_eq!(failure_of(456), Some(Failure::Quota));
        assert_eq!(failure_of(500), Some(Failure::Unavailable));
        assert_eq!(failure_of(504), Some(Failure::Unavailable));
        assert_eq!(failure_of(400), Some(Failure::Unexpected));
        assert_eq!(failure_of(404), Some(Failure::Unexpected));
        // DeepL's error bodies (`{"message": …}`) are never shown: a refusal stays a code.
        let recorder = Recorder::replying(456, r#"{"message":"Quota exceeded"}"#);
        assert_eq!(run(translate(&recorder, Some("k"), "Hello", "FR")), Err(Failure::Quota));
        let recorder = Recorder::replying(403, r#"{"message":"Wrong endpoint. Use https://api.deepl.com"}"#);
        assert_eq!(run(translate(&recorder, Some("k:fx"), "Hello", "FR")), Err(Failure::KeyRefused));
        let unreachable = Recorder { sent: Mutex::new(Vec::new()), reply: Err(Failure::Network) };
        assert_eq!(run(translate(&unreachable, Some("k"), "Hello", "FR")), Err(Failure::Network));
    }

    #[test]
    fn answers_are_read_defensively() {
        assert_eq!(translation_of(HALLO.as_bytes()).unwrap().text, "Hallo, Welt!");
        let without = translation_of(br#"{"translations":[{"text":"Bonjour"}]}"#).unwrap();
        assert_eq!(without, Translation { text: "Bonjour".into(), detected: String::new() });
        let odd = translation_of(br#"{"translations":[{"text":"x","detected_source_language":"<b>EN</b>"}]}"#).unwrap();
        assert_eq!(odd.detected, "");
        let lower = translation_of(br#"{"translations":[{"text":"x","detected_source_language":"en"}]}"#).unwrap();
        assert_eq!(lower.detected, "EN");
        assert_eq!(translation_of(b"<html>"), Err(Failure::Unexpected));
        assert_eq!(translation_of(br#"{"translations":[]}"#), Err(Failure::Unexpected));
        assert_eq!(translation_of(br#"{"message":"Bad request"}"#), Err(Failure::Unexpected));
    }

    #[test]
    fn failures_reach_the_interface_as_codes() {
        assert_eq!(serde_json::to_value(Failure::NoKey).unwrap(), json!("noKey"));
        assert_eq!(serde_json::to_value(Failure::KeyRefused).unwrap(), json!("keyRefused"));
        assert_eq!(serde_json::to_value(Failure::BadLanguage).unwrap(), json!("badLanguage"));
        let shown = serde_json::to_value(Translation { text: "Hallo".into(), detected: "EN".into() }).unwrap();
        assert_eq!(shown, json!({ "text": "Hallo", "detected": "EN" }));
    }

    #[test]
    fn the_chosen_language_is_one_deepl_offers() {
        let chosen: TranslateSettings = serde_json::from_str(r#"{"target":"PT-BR"}"#).unwrap();
        assert_eq!(chosen.sanitized().target, "PT-BR");
        let odd: TranslateSettings = serde_json::from_str(r#"{"target":"pt-br"}"#).unwrap();
        assert_eq!(odd.sanitized().target, "", "back to the interface's language");
        assert_eq!(TranslateSettings::default().target, "");
        let older: crate::settings::Settings = serde_json::from_str(r#"{"theme":"fusion"}"#).unwrap();
        assert_eq!(older.sanitized().translate.target, "", "a settings file from before DeepL still loads");
        assert!(TARGETS.iter().all(|t| t.len() <= MAX_LANGUAGE_CHARS && t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')));
        let mut sorted = TARGETS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), TARGETS.len(), "no language twice");
    }

    /// The real capabilities, compiled by build.rs: only the island translates.
    mod access {
        use tauri::ipc::{CallbackFn, InvokeBody};
        use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
        use tauri::webview::InvokeRequest;
        use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

        fn app() -> App<MockRuntime> {
            mock_builder().build(tauri::generate_context!()).unwrap()
        }

        fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
            WebviewWindowBuilder::new(app, label, WebviewUrl::default()).build().unwrap()
        }

        fn allowed(window: &WebviewWindow<MockRuntime>, cmd: &str) -> bool {
            let request = InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "http://tauri.localhost".parse().unwrap(),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            get_ipc_response(window, request).is_err_and(|e| !e.to_string().contains("not allowed"))
        }

        #[test]
        fn only_the_island_translates_and_reads_the_clipboard() {
            let app = app();
            let main = window(&app, crate::window::WINDOW_LABEL);
            let buddy = window(&app, crate::buddy::BUDDY_LABEL);
            let mini = window(&app, crate::players::MINI_LABEL);
            for cmd in ["translate_text", "read_clipboard_text"] {
                assert!(allowed(&main, cmd), "island: {cmd}");
                assert!(!allowed(&buddy, cmd), "buddy: {cmd}");
                assert!(!allowed(&mini, cmd), "mini-player: {cmd}");
            }
        }
    }
}
