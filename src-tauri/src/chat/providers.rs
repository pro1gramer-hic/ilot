//! The five providers of the chat, as pure functions: where a request goes, its headers
//! and body, what each streamed piece means, the list of models, and an error the user
//! can act on. Every address is fixed here: nothing that comes from the interface ever
//! picks a host. The two local ones answer on the loopback address only.
//!
//! Sources (October 2026):
//! - Anthropic: https://platform.claude.com/docs/en/build-with-claude/streaming,
//!   https://platform.claude.com/docs/en/api/models/list,
//!   https://platform.claude.com/docs/en/build-with-claude/vision
//! - Gemini: https://ai.google.dev/gemini-api/docs/generate-content/text-generation,
//!   https://ai.google.dev/api/generate-content, https://ai.google.dev/api/models,
//!   https://ai.google.dev/gemini-api/docs/image-understanding
//! - OpenAI: https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events,
//!   https://developers.openai.com/api/reference/resources/models/methods/list
//! - Ollama: https://docs.ollama.com/api/chat, https://docs.ollama.com/api/tags,
//!   https://docs.ollama.com/api/errors, https://docs.ollama.com/faq (127.0.0.1:11434)
//! - LM Studio: https://lmstudio.ai/docs/developer/openai-compat (localhost:1234, 127.0.0.1)

use super::stream::Event;
use crate::locale::{Text, tr};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    Gemini,
    Openai,
    Ollama,
    Lmstudio,
}

const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Anthropic asks for a ceiling on every answer.
const MAX_ANSWER_TOKENS: u32 = 4096;

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Claude",
            Self::Gemini => "Gemini",
            Self::Openai => "OpenAI",
            Self::Ollama => "Ollama",
            Self::Lmstudio => "LM Studio",
        }
    }

    /// The vault entry holding its key (none for the local ones).
    pub fn secret(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("chat:anthropic"),
            Self::Gemini => Some("chat:gemini"),
            Self::Openai => Some("chat:openai"),
            Self::Ollama | Self::Lmstudio => None,
        }
    }

    /// Runs on this PC: reached on 127.0.0.1 only, never through a proxy.
    pub fn local(self) -> bool {
        matches!(self, Self::Ollama | Self::Lmstudio)
    }

    /// Where the answer is asked for. `model` was checked by `valid_model_for`.
    pub fn chat_url(self, model: &str) -> String {
        match self {
            Self::Anthropic => "https://api.anthropic.com/v1/messages".into(),
            Self::Gemini => format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse"),
            Self::Openai => "https://api.openai.com/v1/chat/completions".into(),
            Self::Ollama => "http://127.0.0.1:11434/api/chat".into(),
            Self::Lmstudio => "http://127.0.0.1:1234/v1/chat/completions".into(),
        }
    }

    pub fn models_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com/v1/models?limit=100",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000",
            Self::Openai => "https://api.openai.com/v1/models",
            Self::Ollama => "http://127.0.0.1:11434/api/tags",
            Self::Lmstudio => "http://127.0.0.1:1234/v1/models",
        }
    }

    /// Ollama answers one JSON object per line; the others send server-sent events.
    pub fn ndjson(self) -> bool {
        self == Self::Ollama
    }
}

/// A model id the provider can take. Gemini's goes into the address: no `/` nor `@`.
pub fn valid_model_for(provider: Provider, model: &str) -> bool {
    crate::settings::valid_model(model) && (provider != Provider::Gemini || !model.contains(['/', '@']))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A piece of a message: text, or an image read from a dropped file.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    Image { mime: &'static str, base64: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

fn text_of(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|p| if let Part::Text(t) = p { Some(t.as_str()) } else { None })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Headers of a request (the key, when the provider takes one).
pub fn headers(provider: Provider, key: Option<&str>) -> Vec<(&'static str, String)> {
    let mut list = vec![("content-type", "application/json".to_owned())];
    match (provider, key) {
        (Provider::Anthropic, Some(key)) => {
            list.push(("x-api-key", key.to_owned()));
            list.push(("anthropic-version", ANTHROPIC_VERSION.to_owned()));
        }
        (Provider::Anthropic, None) => list.push(("anthropic-version", ANTHROPIC_VERSION.to_owned())),
        (Provider::Gemini, Some(key)) => list.push(("x-goog-api-key", key.to_owned())),
        (Provider::Openai, Some(key)) => list.push(("authorization", format!("Bearer {key}"))),
        _ => {}
    }
    list
}

/// The body that asks `model` to answer `messages`, streamed.
pub fn body(provider: Provider, model: &str, messages: &[Message]) -> Value {
    match provider {
        Provider::Anthropic => {
            let messages: Vec<Value> = messages
                .iter()
                .map(|m| {
                    let content: Vec<Value> = m
                        .parts
                        .iter()
                        .map(|part| match part {
                            Part::Text(text) => json!({ "type": "text", "text": text }),
                            Part::Image { mime, base64 } => {
                                json!({ "type": "image", "source": { "type": "base64", "media_type": mime, "data": base64 } })
                            }
                        })
                        .collect();
                    json!({ "role": role_name(m.role), "content": content })
                })
                .collect();
            json!({ "model": model, "max_tokens": MAX_ANSWER_TOKENS, "stream": true, "messages": messages })
        }
        Provider::Openai | Provider::Lmstudio => {
            let messages: Vec<Value> = messages
                .iter()
                .map(|m| {
                    if m.parts.iter().all(|p| matches!(p, Part::Text(_))) {
                        return json!({ "role": role_name(m.role), "content": text_of(&m.parts) });
                    }
                    let content: Vec<Value> = m
                        .parts
                        .iter()
                        .map(|part| match part {
                            Part::Text(text) => json!({ "type": "text", "text": text }),
                            Part::Image { mime, base64 } => {
                                json!({ "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{base64}") } })
                            }
                        })
                        .collect();
                    json!({ "role": role_name(m.role), "content": content })
                })
                .collect();
            json!({ "model": model, "stream": true, "messages": messages })
        }
        Provider::Gemini => {
            let contents: Vec<Value> = messages
                .iter()
                .map(|m| {
                    let parts: Vec<Value> = m
                        .parts
                        .iter()
                        .map(|part| match part {
                            Part::Text(text) => json!({ "text": text }),
                            Part::Image { mime, base64 } => json!({ "inline_data": { "mime_type": mime, "data": base64 } }),
                        })
                        .collect();
                    json!({ "role": if m.role == Role::User { "user" } else { "model" }, "parts": parts })
                })
                .collect();
            json!({ "contents": contents })
        }
        Provider::Ollama => {
            let messages: Vec<Value> = messages
                .iter()
                .map(|m| {
                    let images: Vec<&str> = m
                        .parts
                        .iter()
                        .filter_map(|p| if let Part::Image { base64, .. } = p { Some(base64.as_str()) } else { None })
                        .collect();
                    let mut message = json!({ "role": role_name(m.role), "content": text_of(&m.parts) });
                    if !images.is_empty() {
                        message["images"] = json!(images);
                    }
                    message
                })
                .collect();
            json!({ "model": model, "stream": true, "messages": messages })
        }
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

/// What a streamed piece brings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Done,
    Error(String),
    Nothing,
}

fn message_of(error: &Value) -> Option<String> {
    error
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| error.as_str())
        .map(str::to_owned)
}

/// One server-sent event of Anthropic, OpenAI, LM Studio or Gemini.
pub fn parse_event(provider: Provider, event: &Event) -> Piece {
    if event.data == "[DONE]" {
        return Piece::Done;
    }
    let Ok(value) = serde_json::from_str::<Value>(&event.data) else { return Piece::Nothing };
    if let Some(error) = value.get("error") {
        return Piece::Error(message_of(error).unwrap_or_else(|| tr(Text::ChatUnknownError)));
    }
    match provider {
        Provider::Anthropic => match value.get("type").and_then(Value::as_str) {
            Some("content_block_delta") => match value.pointer("/delta/type").and_then(Value::as_str) {
                Some("text_delta") => text_piece(value.pointer("/delta/text")),
                _ => Piece::Nothing,
            },
            Some("message_stop") => Piece::Done,
            _ => Piece::Nothing,
        },
        Provider::Openai | Provider::Lmstudio => text_piece(value.pointer("/choices/0/delta/content")),
        Provider::Gemini => {
            let text: String = value
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .map(|parts| parts.iter().filter_map(|p| p.get("text").and_then(Value::as_str)).collect())
                .unwrap_or_default();
            if !text.is_empty() {
                return Piece::Text(text);
            }
            match value.pointer("/candidates/0/finishReason").and_then(Value::as_str) {
                Some("SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII") => {
                    Piece::Error(tr(Text::GeminiBlockedAnswer))
                }
                _ => match value.pointer("/promptFeedback/blockReason").and_then(Value::as_str) {
                    Some(_) => Piece::Error(tr(Text::GeminiBlockedQuestion)),
                    None => Piece::Nothing,
                },
            }
        }
        Provider::Ollama => Piece::Nothing,
    }
}

/// One line of Ollama's answer.
pub fn parse_line(line: &str) -> Piece {
    let Ok(value) = serde_json::from_str::<Value>(line) else { return Piece::Nothing };
    if let Some(error) = value.get("error") {
        return Piece::Error(message_of(error).unwrap_or_else(|| tr(Text::ChatUnknownError)));
    }
    match text_piece(value.pointer("/message/content")) {
        Piece::Text(text) => Piece::Text(text),
        _ if value.get("done").and_then(Value::as_bool) == Some(true) => Piece::Done,
        _ => Piece::Nothing,
    }
}

fn text_piece(text: Option<&Value>) -> Piece {
    match text.and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Piece::Text(text.to_owned()),
        _ => Piece::Nothing,
    }
}

/// OpenAI lists every model of the account: only those that chat are offered.
fn openai_chats(id: &str) -> bool {
    let chat_family = id.starts_with("gpt-") || id.starts_with("chatgpt-") || (id.starts_with('o') && id[1..].starts_with(|c: char| c.is_ascii_digit()));
    let other_use = ["audio", "realtime", "transcribe", "tts", "image", "search", "embedding", "instruct"];
    chat_family && !other_use.iter().any(|word| id.contains(word))
}

/// The models offered by a provider's list, in the provider's order (OpenAI: newest first).
pub fn parse_models(provider: Provider, value: &Value) -> Vec<String> {
    let ids = |list: &str, key: &str| -> Vec<String> {
        value
            .get(list)
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(|m| m.get(key).and_then(Value::as_str).map(str::to_owned)).collect())
            .unwrap_or_default()
    };
    let mut models = match provider {
        Provider::Anthropic => ids("data", "id"),
        Provider::Ollama => ids("models", "name"),
        Provider::Lmstudio => ids("data", "id").into_iter().filter(|id| !id.contains("embed")).collect(),
        Provider::Openai => {
            let mut items: Vec<(i64, String)> = value
                .get("data")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|m| Some((m.get("created").and_then(Value::as_i64).unwrap_or(0), m.get("id")?.as_str()?.to_owned())))
                        .filter(|(_, id)| openai_chats(id))
                        .collect()
                })
                .unwrap_or_default();
            items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            items.into_iter().map(|(_, id)| id).collect()
        }
        Provider::Gemini => value
            .get("models")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|m| {
                        m.get("supportedGenerationMethods")
                            .and_then(Value::as_array)
                            .is_some_and(|methods| methods.iter().any(|x| x == "generateContent"))
                    })
                    .filter_map(|m| m.get("name")?.as_str()?.strip_prefix("models/").map(str::to_owned))
                    .filter(|id| id.starts_with("gemini") && !["tts", "image", "embedding"].iter().any(|w| id.contains(w)))
                    .collect()
            })
            .unwrap_or_default(),
    };
    models.retain(|id| valid_model_for(provider, id));
    models.dedup();
    models
}

/// Longest provider message shown to the user.
const MAX_MESSAGE_CHARS: usize = 220;

/// A refused request, in words the user can act on. 401 and 403 get a fixed sentence:
/// some providers quote part of the key in theirs.
pub fn error_message(provider: Provider, status: u16, body: &str) -> String {
    let name = provider.label();
    match status {
        401 | 403 if !provider.local() => tr(Text::ProviderKeyRefused { name }),
        404 => tr(Text::ProviderModelMissing { name }),
        413 => tr(Text::ProviderTooLong { name }),
        429 => tr(Text::ProviderLimited { name }),
        500..=599 => tr(Text::ProviderDown { name }),
        _ => {
            let detail = serde_json::from_str::<Value>(body)
                .ok()
                .and_then(|v| v.get("error").and_then(message_of))
                .map(|m| m.chars().take(MAX_MESSAGE_CHARS).collect::<String>());
            match detail {
                Some(detail) if !detail.trim().is_empty() => tr(Text::ProviderRefusedBecause { name, status, detail: &detail }),
                _ => tr(Text::ProviderRefused { name, status }),
            }
        }
    }
}

/// When the provider cannot be reached at all.
pub fn unreachable(provider: Provider) -> String {
    match provider {
        Provider::Ollama => tr(Text::OllamaDown),
        Provider::Lmstudio => tr(Text::LmStudioDown),
        _ => tr(Text::ProviderUnreachable { name: provider.label() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(data: &str) -> Event {
        Event { name: None, data: data.into() }
    }

    fn user(text: &str) -> Message {
        Message { role: Role::User, parts: vec![Part::Text(text.into())] }
    }

    #[test]
    fn remote_providers_use_https_and_local_ones_the_loopback_only() {
        for provider in [Provider::Anthropic, Provider::Gemini, Provider::Openai] {
            assert!(!provider.local());
            assert!(provider.chat_url("gemini-2.5-flash").starts_with("https://"));
            assert!(provider.models_url().starts_with("https://"));
        }
        for provider in [Provider::Ollama, Provider::Lmstudio] {
            assert!(provider.local());
            for url in [provider.chat_url("x"), provider.models_url().to_owned()] {
                let parsed = reqwest::Url::parse(&url).unwrap();
                assert_eq!(parsed.host_str(), Some("127.0.0.1"), "{url}");
            }
            assert_eq!(provider.secret(), None);
        }
        assert_eq!(Provider::Ollama.chat_url("x"), "http://127.0.0.1:11434/api/chat");
        assert_eq!(Provider::Lmstudio.models_url(), "http://127.0.0.1:1234/v1/models");
    }

    #[test]
    fn gemini_models_never_change_the_address() {
        assert!(valid_model_for(Provider::Gemini, "gemini-2.5-flash"));
        assert!(!valid_model_for(Provider::Gemini, "../../v1/files"));
        assert!(!valid_model_for(Provider::Gemini, "x@y"));
        assert!(valid_model_for(Provider::Lmstudio, "google/gemma-3-12b"));
        assert!(!valid_model_for(Provider::Openai, "gpt 4"));
        assert_eq!(
            Provider::Gemini.chat_url("gemini-2.5-flash"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn each_provider_gets_its_key_in_its_own_header() {
        let find = |provider, key: Option<&str>| headers(provider, key);
        assert!(find(Provider::Anthropic, Some("k")).contains(&("x-api-key", "k".into())));
        assert!(find(Provider::Anthropic, Some("k")).contains(&("anthropic-version", "2023-06-01".into())));
        assert!(find(Provider::Gemini, Some("k")).contains(&("x-goog-api-key", "k".into())));
        assert!(find(Provider::Openai, Some("k")).contains(&("authorization", "Bearer k".into())));
        for provider in [Provider::Ollama, Provider::Lmstudio] {
            assert_eq!(find(provider, None), [("content-type", "application/json".to_owned())]);
        }
    }

    #[test]
    fn bodies_follow_each_api() {
        let image = Message {
            role: Role::User,
            parts: vec![Part::Text("Que vois-tu ?".into()), Part::Image { mime: "image/png", base64: "iVBO".into() }],
        };
        let history = [user("Bonjour"), Message { role: Role::Assistant, parts: vec![Part::Text("Salut".into())] }, image];
        let anthropic = body(Provider::Anthropic, "claude-x", &history);
        assert_eq!(anthropic["max_tokens"], 4096);
        assert_eq!(anthropic["stream"], true);
        assert_eq!(anthropic["messages"][1]["role"], "assistant");
        assert_eq!(anthropic["messages"][2]["content"][1]["source"]["media_type"], "image/png");
        let openai = body(Provider::Openai, "gpt-x", &history);
        assert_eq!(openai["messages"][0]["content"], "Bonjour");
        assert_eq!(openai["messages"][2]["content"][1]["image_url"]["url"], "data:image/png;base64,iVBO");
        let gemini = body(Provider::Gemini, "gemini-x", &history);
        assert_eq!(gemini["contents"][1]["role"], "model");
        assert_eq!(gemini["contents"][2]["parts"][1]["inline_data"]["mime_type"], "image/png");
        assert!(gemini.get("model").is_none(), "the model is in the address");
        let ollama = body(Provider::Ollama, "llava", &history);
        assert_eq!(ollama["messages"][2]["content"], "Que vois-tu ?");
        assert_eq!(ollama["messages"][2]["images"][0], "iVBO");
        assert!(ollama["messages"][0].get("images").is_none());
    }

    /// Events of https://platform.claude.com/docs/en/build-with-claude/streaming
    #[test]
    fn anthropic_events() {
        let p = Provider::Anthropic;
        assert_eq!(parse_event(p, &event(r#"{"type": "message_start", "message": {"id": "msg_1", "content": []}}"#)), Piece::Nothing);
        assert_eq!(
            parse_event(p, &event(r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}"#)),
            Piece::Text("Hello".into())
        );
        assert_eq!(
            parse_event(p, &event(r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "hmm"}}"#)),
            Piece::Nothing
        );
        assert_eq!(parse_event(p, &event(r#"{"type": "ping"}"#)), Piece::Nothing);
        assert_eq!(
            parse_event(p, &event(r#"{"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence":null}, "usage": {"output_tokens": 15}}"#)),
            Piece::Nothing
        );
        assert_eq!(parse_event(p, &event(r#"{"type": "message_stop"}"#)), Piece::Done);
        assert_eq!(
            parse_event(p, &event(r#"{"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}"#)),
            Piece::Error("Overloaded".into())
        );
    }

    /// Chunks of https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events
    #[test]
    fn openai_and_lm_studio_chunks() {
        for p in [Provider::Openai, Provider::Lmstudio] {
            let first = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#;
            assert_eq!(parse_event(p, &event(first)), Piece::Nothing);
            let next = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"content":"Bonjour"},"finish_reason":null}]}"#;
            assert_eq!(parse_event(p, &event(next)), Piece::Text("Bonjour".into()));
            let usage = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":12}}"#;
            assert_eq!(parse_event(p, &event(usage)), Piece::Nothing);
            assert_eq!(parse_event(p, &event("[DONE]")), Piece::Done);
        }
    }

    /// GenerateContentResponse pieces of https://ai.google.dev/api/generate-content
    #[test]
    fn gemini_pieces() {
        let p = Provider::Gemini;
        let piece = r#"{"candidates": [{"content": {"parts": [{"text": "Le ciel "}, {"text": "est bleu"}],"role": "model"},"index": 0}],"usageMetadata": {"promptTokenCount": 8},"modelVersion": "gemini-2.5-flash","responseId": "abc"}"#;
        assert_eq!(parse_event(p, &event(piece)), Piece::Text("Le ciel est bleu".into()));
        let last = r#"{"candidates": [{"content": {"parts": [{"text": ""}],"role": "model"},"finishReason": "STOP","index": 0}]}"#;
        assert_eq!(parse_event(p, &event(last)), Piece::Nothing);
        let blocked = r#"{"candidates": [{"finishReason": "SAFETY","index": 0}]}"#;
        assert!(matches!(parse_event(p, &event(blocked)), Piece::Error(_)));
        let error = r#"{"error": {"code": 400, "message": "API key not valid. Please pass a valid API key.", "status": "INVALID_ARGUMENT"}}"#;
        assert_eq!(parse_event(p, &event(error)), Piece::Error("API key not valid. Please pass a valid API key.".into()));
    }

    /// Lines of https://docs.ollama.com/api/chat and https://docs.ollama.com/api/errors
    #[test]
    fn ollama_lines() {
        assert_eq!(
            parse_line(r#"{"model":"gemma4","created_at":"2025-10-17T23:14:07.414671Z","message":{"role":"assistant","content":"Bon"},"done":false}"#),
            Piece::Text("Bon".into())
        );
        assert_eq!(
            parse_line(r#"{"model":"gemma4","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop","total_duration":1}"#),
            Piece::Done
        );
        assert_eq!(parse_line(r#"{"error":"model 'nope' not found"}"#), Piece::Error("model 'nope' not found".into()));
        assert_eq!(parse_line("not json"), Piece::Nothing);
    }

    #[test]
    fn model_lists() {
        let anthropic = json!({"data":[{"id":"claude-opus-5","display_name":"Claude Opus 5","type":"model"},{"id":"claude-sonnet-5","type":"model"}],"has_more":false});
        assert_eq!(parse_models(Provider::Anthropic, &anthropic), ["claude-opus-5", "claude-sonnet-5"]);
        let openai = json!({"object":"list","data":[
            {"id":"gpt-4o-mini","created":10,"owned_by":"system"},
            {"id":"text-embedding-3-small","created":30},
            {"id":"gpt-5","created":20},
            {"id":"whisper-1","created":5},
            {"id":"gpt-4o-realtime-preview","created":25},
            {"id":"o3","created":15},
            {"id":"dall-e-3","created":1}]});
        assert_eq!(parse_models(Provider::Openai, &openai), ["gpt-5", "o3", "gpt-4o-mini"]);
        let gemini = json!({"models":[
            {"name":"models/gemini-2.5-flash","supportedGenerationMethods":["generateContent","countTokens"]},
            {"name":"models/text-embedding-004","supportedGenerationMethods":["embedContent"]},
            {"name":"models/gemini-2.5-flash-preview-tts","supportedGenerationMethods":["generateContent"]},
            {"name":"models/gemini-2.5-pro","supportedGenerationMethods":["generateContent"]}]});
        assert_eq!(parse_models(Provider::Gemini, &gemini), ["gemini-2.5-flash", "gemini-2.5-pro"]);
        let ollama = json!({"models":[{"name":"gemma4","modified_at":"2025-10-03T23:34:03","size":9608350245u64},{"name":"llama3.2:3b"}]});
        assert_eq!(parse_models(Provider::Ollama, &ollama), ["gemma4", "llama3.2:3b"]);
        let lmstudio = json!({"object":"list","data":[{"id":"google/gemma-3-12b"},{"id":"text-embedding-nomic-embed-text-v1.5"}]});
        assert_eq!(parse_models(Provider::Lmstudio, &lmstudio), ["google/gemma-3-12b"]);
        assert!(parse_models(Provider::Ollama, &json!({})).is_empty());
    }

    #[test]
    fn errors_name_the_provider_and_never_quote_a_key() {
        let quoted = r#"{"error":{"message":"Incorrect API key provided: sk-proj-****wxyz. You can find your API key at https://platform.openai.com/account/api-keys.","type":"invalid_request_error","code":"invalid_api_key"}}"#;
        let message = error_message(Provider::Openai, 401, quoted);
        assert_eq!(message, "Clé refusée par OpenAI. Vérifie-la dans Réglages › Chat.");
        assert!(!message.contains("sk-"));
        assert!(error_message(Provider::Anthropic, 404, "").starts_with("Modèle introuvable chez Claude"));
        assert!(error_message(Provider::Gemini, 429, "").contains("limite"));
        assert!(error_message(Provider::Anthropic, 529, "").contains("surchargé"));
        let bad = r#"{"type":"error","error":{"type":"invalid_request_error","message":"messages: text content blocks must be non-empty"}}"#;
        assert_eq!(
            error_message(Provider::Anthropic, 400, bad),
            "Claude a refusé la demande (400) : messages: text content blocks must be non-empty"
        );
        assert_eq!(error_message(Provider::Ollama, 400, "{}"), "Ollama a refusé la demande (400).");
        assert!(unreachable(Provider::Ollama).contains("Lance Ollama"));
    }

    #[test]
    fn errors_are_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(error_message(Provider::Anthropic, 404, ""), "Claude doesn't have this model. Pick another one.");
        assert_eq!(error_message(Provider::Ollama, 400, "{}"), "Ollama refused the request (400).");
        assert_eq!(unreachable(Provider::Openai), "Couldn't reach OpenAI. Check your Internet connection.");
    }
}
