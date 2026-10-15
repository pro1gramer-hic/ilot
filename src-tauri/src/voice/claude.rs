//! The one question « OK Mochi » sends out: to Claude (Messages API), with the user's own
//! key, after the user clicked « Envoyer à Claude » on the island, and only with what the
//! island showed them (the words recognized and a few counters). Never audio. Without a
//! key, nothing is sent at all.
//! https://platform.claude.com/docs/en/api/messages

use crate::locale::{Text, tr};
use serde_json::{Value, json};

pub const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
pub const MODEL: &str = "claude-opus-5-5";
pub const API_VERSION: &str = "2023-06-01";
/// Server-side fallback when a safety classifier declines (`fallbacks: "default"`).
pub const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// What the island lets through, at most.
pub const MAX_QUESTION_CHARS: usize = 200;
pub const MAX_CONTEXT_CHARS: usize = 4_000;

/// Why the question got no answer, in words for the island and for Mochi's voice (the
/// language of the interface).
#[derive(Debug, PartialEq, Eq)]
pub enum AskError {
    NoKey,
    Refused,
    KeyRejected,
    Busy,
    Unreachable,
    Unexpected(u16),
}

impl AskError {
    pub fn message(&self) -> String {
        tr(match self {
            Self::NoKey => Text::AskNoKey,
            Self::Refused => Text::AskRefused,
            Self::KeyRejected => Text::AskKeyRejected,
            Self::Busy => Text::AskBusy,
            Self::Unreachable => Text::AskUnreachable,
            Self::Unexpected(status) => Text::AskUnexpected { status: *status },
        })
    }
}

/// Sends the request: Windows' HTTP client in the app, a recorder in the tests.
pub trait Transport {
    /// Posts `body` with `headers`; returns the HTTP status and the body.
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<(u16, String), AskError>;
}

fn cut(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control() || *c == '\n').take(max).collect()
}

/// The request body: the question the user said, then the data the island showed. Mochi
/// is asked to answer in the language of the interface, the one his voice speaks.
pub fn body(question: &str, context: &str) -> Value {
    let (question, context) = (cut(question, MAX_QUESTION_CHARS), cut(context, MAX_CONTEXT_CHARS));
    let content = tr(Text::AskContent { question: &question, context: &context });
    json!({
        "model": MODEL,
        "max_tokens": 2048,
        "fallbacks": "default",
        "output_config": { "effort": "low" },
        "system": tr(Text::AskSystem),
        "messages": [{ "role": "user", "content": content }],
    })
}

/// The answer's text, or why there is none.
pub fn answer_of(status: u16, response: &str) -> Result<String, AskError> {
    match status {
        200 => {}
        401 | 403 => return Err(AskError::KeyRejected),
        429 | 529 => return Err(AskError::Busy),
        other => return Err(AskError::Unexpected(other)),
    }
    let value: Value = serde_json::from_str(response).map_err(|_| AskError::Unexpected(status))?;
    if value["stop_reason"] == "refusal" {
        return Err(AskError::Refused);
    }
    let text: Vec<&str> = value["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect();
    let text = text.join(" ").trim().to_owned();
    if text.is_empty() { Err(AskError::Unexpected(status)) } else { Ok(text) }
}

/// Asks Claude. With no key, nothing goes out.
pub fn ask(transport: &impl Transport, key: Option<&str>, question: &str, context: &str) -> Result<String, AskError> {
    let key = key.map(str::trim).filter(|k| !k.is_empty()).ok_or(AskError::NoKey)?;
    let headers = [
        ("x-api-key", key),
        ("anthropic-version", API_VERSION),
        ("anthropic-beta", FALLBACK_BETA),
    ];
    let (status, response) = transport.post(ENDPOINT, &headers, &body(question, context).to_string())?;
    answer_of(status, &response)
}

/// Windows' own HTTP client (Windows.Web.Http, TLS by Windows): no network library added.
/// https://learn.microsoft.com/en-us/uwp/api/windows.web.http.httpclient
pub struct WindowsHttp;

impl Transport for WindowsHttp {
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<(u16, String), AskError> {
        use windows::Foundation::Uri;
        use windows::Storage::Streams::UnicodeEncoding;
        use windows::Web::Http::{HttpClient, HttpMethod, HttpRequestMessage, HttpStringContent};
        use windows::core::HSTRING;
        let send = || -> windows::core::Result<(u16, String)> {
            let request = HttpRequestMessage::Create(&HttpMethod::Post()?, &Uri::CreateUri(&HSTRING::from(url))?)?;
            let content = HttpStringContent::CreateFromStringWithEncodingAndMediaType(
                &HSTRING::from(body),
                UnicodeEncoding::Utf8,
                &HSTRING::from("application/json"),
            )?;
            request.SetContent(&content)?;
            let sent_headers = request.Headers()?;
            for (name, value) in headers {
                sent_headers.TryAppendWithoutValidation(&HSTRING::from(*name), &HSTRING::from(*value))?;
            }
            let response = HttpClient::new()?.SendRequestAsync(&request)?.join()?;
            let status = response.StatusCode()?.0 as u16;
            let text = response.Content()?.ReadAsStringAsync()?.join()?;
            Ok((status, text.to_string()))
        };
        send().map_err(|_| AskError::Unreachable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A request as it would have gone: address, headers, body.
    type Sent = (String, Vec<(String, String)>, String);

    /// Records what would have been sent.
    #[derive(Default)]
    struct Recorder {
        sent: RefCell<Vec<Sent>>,
        reply: (u16, String),
    }

    impl Transport for Recorder {
        fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<(u16, String), AskError> {
            let headers = headers.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
            self.sent.borrow_mut().push((url.to_owned(), headers, body.to_owned()));
            Ok(self.reply.clone())
        }
    }

    fn replying(status: u16, body: &str) -> Recorder {
        Recorder { reply: (status, body.to_owned()), ..Recorder::default() }
    }

    #[test]
    fn without_a_key_nothing_is_sent() {
        let recorder = replying(200, "{}");
        assert_eq!(ask(&recorder, None, "Résume mon activité", "3 sessions"), Err(AskError::NoKey));
        assert_eq!(ask(&recorder, Some("  "), "Résume mon activité", "3 sessions"), Err(AskError::NoKey));
        assert!(recorder.sent.borrow().is_empty());
        assert!(AskError::NoKey.message().contains("clé Claude"));
    }

    #[test]
    fn only_the_words_and_the_data_shown_go_out_with_the_key() {
        let recorder = replying(200, r#"{"stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":"Trois sessions tournent."}]}"#);
        let answer = ask(&recorder, Some("sk-ant-test"), "Résume mon activité", "Claude Code : au travail sur epilot").unwrap();
        assert_eq!(answer, "Trois sessions tournent.");
        let sent = recorder.sent.borrow();
        let (url, headers, body) = &sent[0];
        assert_eq!(url, ENDPOINT);
        assert!(headers.contains(&("x-api-key".into(), "sk-ant-test".into())));
        assert!(headers.contains(&("anthropic-beta".into(), FALLBACK_BETA.into())));
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["fallbacks"], "default");
        let content = body["messages"][0]["content"].as_str().unwrap();
        assert_eq!(content, "Résume mon activité\n\nDonnées d'îlot :\nClaude Code : au travail sur epilot");
        assert!(!body.to_string().contains("sk-ant-test"), "the key travels in a header only");
    }

    #[test]
    fn long_inputs_are_cut() {
        let body = body(&"q".repeat(1000), &"c\u{0}".repeat(9000));
        let content = body["messages"][0]["content"].as_str().unwrap();
        assert_eq!(content.matches('q').count(), MAX_QUESTION_CHARS);
        assert_eq!(content.matches('c').count(), MAX_CONTEXT_CHARS);
        assert!(!content.contains('\u{0}'));
    }

    #[test]
    fn errors_and_refusals_have_words() {
        assert_eq!(answer_of(401, ""), Err(AskError::KeyRejected));
        assert_eq!(answer_of(429, ""), Err(AskError::Busy));
        assert_eq!(answer_of(500, ""), Err(AskError::Unexpected(500)));
        assert_eq!(answer_of(200, r#"{"stop_reason":"refusal","content":[]}"#), Err(AskError::Refused));
        assert_eq!(answer_of(200, "not json"), Err(AskError::Unexpected(200)));
        assert_eq!(AskError::Unexpected(500).message(), "Claude a répondu une erreur (500).");
    }

    #[test]
    fn mochi_answers_in_the_language_of_the_interface() {
        let french = body("Résume", "3 sessions");
        assert!(french["system"].as_str().unwrap().contains("réponds en français, en deux ou trois phrases courtes"));
        crate::locale::speak(crate::locale::Lang::En);
        let english = body("Sum it up", "3 sessions");
        assert!(english["system"].as_str().unwrap().contains("answer in English"));
        assert_eq!(english["messages"][0]["content"], "Sum it up\n\nîlot data:\n3 sessions");
        assert_eq!(AskError::Unexpected(500).message(), "Claude answered with an error (500).");
        assert!(AskError::NoKey.message().contains("Claude key"));
    }
}
