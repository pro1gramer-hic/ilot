//! The chat in the island (prompt 12, lot 3): a question goes to the provider the user
//! picked (Claude, Gemini, OpenAI with the user's key, or Ollama and LM Studio on this
//! PC), and the answer streams back to the island piece by piece.
//!
//! Every request leaves from here, never from the webview: its CSP keeps `connect-src`
//! local, and the key never reaches it. Hosts are fixed in `providers.rs`; remote ones are
//! reached over HTTPS only, local ones on 127.0.0.1 only and never through a proxy; no
//! redirect is followed. The conversation lives in the interface's memory and is sent with
//! each question; nothing is written to disk or logged. A dropped file or a window's text
//! joins a question only when the user sends it (`attachments.rs`, `window_context.rs`).

pub mod providers;
mod stream;

use crate::attachments::{Attachments, Loaded};
use crate::locale::{Text, tr};
use crate::vault;
use crate::window::WINDOW_LABEL;
use crate::window_context::Contexts;
use providers::{Message, Part, Piece, Provider, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};

pub const EVENT_DELTA: &str = "chat:delta";
pub const EVENT_DONE: &str = "chat:done";
pub const EVENT_ERROR: &str = "chat:error";

/// The conversation sent with a question: its latest turns only.
const MAX_TURNS: usize = 40;
const MAX_TURN_CHARS: usize = 32_000;
/// An answer stops there, whatever the model meant to add.
const MAX_ANSWER_CHARS: usize = 200_000;
/// What is read of a refusal to explain it.
const MAX_ERROR_BYTES: usize = 16 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Silence allowed between two pieces of an answer (a local model may load first).
const READ_TIMEOUT: Duration = Duration::from_secs(120);

pub const EMPTY_QUESTION: Text<'static> = Text::ChatEmptyQuestion;
pub const BAD_REQUEST: Text<'static> = Text::ChatBadRequest;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub role: Role,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    /// Chosen by the interface, it tags the streamed pieces of this answer.
    pub request_id: String,
    pub provider: Provider,
    pub model: String,
    pub turns: Vec<Turn>,
    /// An attachment made from a file dropped on the island (`file_attach`).
    #[serde(default)]
    pub attachment: Option<String>,
    /// A window's text the user previewed and joined (`context_confirm`).
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Delta<'a> {
    request_id: &'a str,
    text: &'a str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ended<'a> {
    request_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
}

/// What joins the question being sent.
#[derive(Default)]
pub struct Extra {
    /// A dropped file: its name and content.
    pub file: Option<(String, Loaded)>,
    /// A window: its title, its program, its text.
    pub context: Option<(String, String, String)>,
}

/// Answers on their way, by request id.
#[derive(Default)]
pub struct Chat {
    running: Mutex<HashMap<String, JoinHandle<()>>>,
}

fn valid_request_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The messages sent to the provider: the latest turns, with the same role never twice in
/// a row (Claude refuses it), starting with the user and ending with the question, which
/// carries the window's text, the file and the image, if any.
pub fn compose(turns: &[Turn], extra: Extra) -> Result<Vec<Message>, String> {
    let mut merged: Vec<(Role, String)> = Vec::new();
    for turn in turns {
        let text = turn.text.trim();
        if text.chars().count() > MAX_TURN_CHARS {
            return Err(tr(Text::ChatMessageTooLong { max: MAX_TURN_CHARS }));
        }
        if text.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some((role, previous)) if *role == turn.role => {
                previous.push_str("\n\n");
                previous.push_str(text);
            }
            _ => merged.push((turn.role, text.to_owned())),
        }
    }
    let skip = merged.len().saturating_sub(MAX_TURNS);
    let mut kept: Vec<(Role, String)> = merged.into_iter().skip(skip).collect();
    while kept.first().is_some_and(|(role, _)| *role == Role::Assistant) {
        kept.remove(0);
    }
    let has_extra = extra.file.is_some() || extra.context.is_some();
    match kept.last() {
        Some((Role::User, _)) => {}
        _ if has_extra => kept.push((Role::User, String::new())),
        _ => return Err(EMPTY_QUESTION.into()),
    }
    let (_, question) = kept.pop().unwrap_or((Role::User, String::new()));
    let mut messages: Vec<Message> =
        kept.into_iter().map(|(role, text)| Message { role, parts: vec![Part::Text(text)] }).collect();
    let mut parts = Vec::new();
    let mut texts = Vec::new();
    if let Some((title, app, text)) = extra.context {
        texts.push(tr(Text::ChatContextAttached { title: &title, app: &app, text: &text }));
    }
    let mut image = None;
    match extra.file {
        Some((name, Loaded::Text(text))) => texts.push(tr(Text::ChatFileAttached { name: &name, text: &text })),
        Some((name, Loaded::Image { mime, base64 })) => {
            texts.push(tr(Text::ChatImageAttached { name: &name }));
            image = Some(Part::Image { mime, base64 });
        }
        None => {}
    }
    texts.push(if question.is_empty() { tr(Text::ChatWhatDoYouThink) } else { question });
    parts.push(Part::Text(texts.join("\n\n")));
    parts.extend(image);
    messages.push(Message { role: Role::User, parts });
    Ok(messages)
}

/// A client for one provider: HTTPS only for the remote ones; for the local ones, no
/// proxy (a system proxy could carry a local request elsewhere). Redirects are refused.
fn client(local: bool) -> Result<reqwest::Client, String> {
    let builder = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("ilot/", env!("CARGO_PKG_VERSION")));
    let builder = if local { builder.no_proxy() } else { builder.https_only(true) };
    builder.build().map_err(|_| tr(Text::ChatNoClient))
}

fn header_map(list: Vec<(&'static str, String)>) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in list {
        let mut value = reqwest::header::HeaderValue::from_str(&value).map_err(|_| tr(Text::ChatKeyInvalid))?;
        // Keys are kept out of any debug output of the client.
        value.set_sensitive(name != "content-type" && name != "anthropic-version");
        headers.insert(name, value);
    }
    Ok(headers)
}

/// Removes the key from a text that is about to be shown (an error).
pub fn redact(text: String, key: Option<&str>) -> String {
    match key {
        Some(key) if key.len() >= 6 => text.replace(key, "•••"),
        _ => text,
    }
}

async fn refusal(provider: Provider, mut response: reqwest::Response) -> String {
    let status = response.status().as_u16();
    let mut body = Vec::new();
    while body.len() < MAX_ERROR_BYTES {
        match response.chunk().await {
            Ok(Some(piece)) => body.extend_from_slice(&piece),
            _ => break,
        }
    }
    providers::error_message(provider, status, &String::from_utf8_lossy(&body))
}

/// Asks for an answer and hands each streamed piece of text to `on_text`.
pub async fn stream_answer(
    client: &reqwest::Client,
    url: &str,
    provider: Provider,
    headers: reqwest::header::HeaderMap,
    body: &serde_json::Value,
    mut on_text: impl FnMut(&str),
) -> Result<(), String> {
    let mut response = client
        .post(url)
        .headers(headers)
        .json(body)
        .send()
        .await
        .map_err(|_| providers::unreachable(provider))?;
    if !response.status().is_success() {
        return Err(refusal(provider, response).await);
    }
    let mut sse = stream::Sse::default();
    let mut lines = stream::Lines::default();
    let mut written = 0usize;
    loop {
        let piece = response.chunk().await.map_err(|e| {
            if e.is_timeout() { tr(Text::ChatStoppedAnswering { provider: provider.label() }) } else { providers::unreachable(provider) }
        })?;
        let pieces: Vec<Piece> = match &piece {
            Some(bytes) if provider.ndjson() => lines.push(bytes)?.iter().map(|l| providers::parse_line(l)).collect(),
            Some(bytes) => sse.push(bytes)?.iter().map(|e| providers::parse_event(provider, e)).collect(),
            None if provider.ndjson() => lines.finish().iter().map(|l| providers::parse_line(l)).collect(),
            None => sse.finish().iter().map(|e| providers::parse_event(provider, e)).collect(),
        };
        let mut text = String::new();
        let mut done = piece.is_none();
        for piece in pieces {
            match piece {
                Piece::Text(t) => text.push_str(&t),
                Piece::Done => done = true,
                Piece::Error(message) => {
                    if !text.is_empty() {
                        on_text(&text);
                    }
                    let message: String = message.chars().take(220).collect();
                    return Err(tr(Text::ChatProviderError { provider: provider.label(), message: &message }));
                }
                Piece::Nothing => {}
            }
        }
        if !text.is_empty() {
            written += text.chars().count();
            on_text(&text);
        }
        if done || written >= MAX_ANSWER_CHARS {
            return Ok(());
        }
    }
}

fn missing_key(provider: Provider) -> String {
    tr(Text::ChatMissingKey { provider: provider.label() })
}

fn key_for(provider: Provider) -> Result<Option<String>, String> {
    match provider.secret() {
        Some(name) => vault::read(name)?.map(Some).ok_or_else(|| missing_key(provider)),
        None => Ok(None),
    }
}

/// Sends a question; the answer comes back as `chat:delta` events, then `chat:done` or
/// `chat:error`. The attachment and the window's text are read now, on this click.
#[tauri::command]
pub async fn chat_send(request: ChatRequest, app: AppHandle) -> Result<(), String> {
    if !valid_request_id(&request.request_id) || !providers::valid_model_for(request.provider, &request.model) {
        return Err(BAD_REQUEST.into());
    }
    let provider = request.provider;
    let key = key_for(provider)?;
    let file = match &request.attachment {
        Some(id) => Some(app.state::<Attachments>().load(id)?),
        None => None,
    };
    let context = match &request.context {
        Some(id) => Some(app.state::<Contexts>().confirmed(id)?),
        None => None,
    };
    let messages = compose(&request.turns, Extra { file, context })?;
    let body = providers::body(provider, &request.model, &messages);
    let headers = header_map(providers::headers(provider, key.as_deref()))?;
    let url = provider.chat_url(&request.model);
    let client = client(provider.local())?;
    let id = request.request_id;
    let task_app = app.clone();
    let task_id = id.clone();
    let task = tauri::async_runtime::spawn(async move {
        let emit = |name: &str, message: Option<&str>| {
            let _ = task_app.emit_to(WINDOW_LABEL, name, Ended { request_id: &task_id, message });
        };
        let result = stream_answer(&client, &url, provider, headers, &body, |text| {
            let _ = task_app.emit_to(WINDOW_LABEL, EVENT_DELTA, Delta { request_id: &task_id, text });
        })
        .await;
        match result {
            Ok(()) => emit(EVENT_DONE, None),
            Err(message) => emit(EVENT_ERROR, Some(&redact(message, key.as_deref()))),
        }
        if let Some(key) = key {
            vault::wipe_string(key);
        }
        task_app.state::<Chat>().running.lock().unwrap().remove(&task_id);
    });
    if let Some(previous) = app.state::<Chat>().running.lock().unwrap().insert(id, task) {
        previous.abort();
    }
    Ok(())
}

/// Stops an answer on its way (« Arrêter »).
#[tauri::command]
pub fn chat_cancel(request_id: String, chat: State<'_, Chat>) {
    if let Some(task) = chat.running.lock().unwrap().remove(&request_id) {
        task.abort();
    }
}

/// The models a provider offers to this key (or that this PC holds, for the local ones).
#[tauri::command]
pub async fn chat_models(provider: Provider) -> Result<Vec<String>, String> {
    let key = key_for(provider)?;
    let headers = header_map(providers::headers(provider, key.as_deref()))?;
    let client = client(provider.local())?;
    let result = async {
        let response = client
            .get(provider.models_url())
            .headers(headers)
            .send()
            .await
            .map_err(|_| providers::unreachable(provider))?;
        if !response.status().is_success() {
            return Err(refusal(provider, response).await);
        }
        let value: serde_json::Value = response.json().await.map_err(|_| stream::MALFORMED)?;
        Ok(providers::parse_models(provider, &value))
    }
    .await
    .map_err(|message| redact(message, key.as_deref()));
    if let Some(key) = key {
        vault::wipe_string(key);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn turn(role: Role, text: &str) -> Turn {
        Turn { role, text: text.into() }
    }

    fn text(message: &Message) -> &str {
        match &message.parts[0] {
            Part::Text(t) => t,
            Part::Image { .. } => "",
        }
    }

    #[test]
    fn the_conversation_alternates_and_ends_with_the_question() {
        let turns = [
            turn(Role::Assistant, "Bonjour, je suis Mochi"),
            turn(Role::User, "Salut"),
            turn(Role::User, "Tu es là ?"),
            turn(Role::Assistant, ""),
            turn(Role::Assistant, "Oui"),
            turn(Role::User, "  Résume ce texte  "),
        ];
        let messages = compose(&turns, Extra::default()).unwrap();
        let roles: Vec<Role> = messages.iter().map(|m| m.role).collect();
        assert_eq!(roles, [Role::User, Role::Assistant, Role::User]);
        assert_eq!(text(&messages[0]), "Salut\n\nTu es là ?");
        assert_eq!(text(&messages[2]), "Résume ce texte");
        assert_eq!(compose(&[turn(Role::User, "  ")], Extra::default()).unwrap_err(), EMPTY_QUESTION);
        assert_eq!(compose(&[turn(Role::Assistant, "Hello")], Extra::default()).unwrap_err(), EMPTY_QUESTION);
        assert!(compose(&[turn(Role::User, &"x".repeat(MAX_TURN_CHARS + 1))], Extra::default()).is_err());
    }

    #[test]
    fn only_the_latest_turns_are_sent() {
        let turns: Vec<Turn> =
            (0..100).map(|i| turn(if i % 2 == 0 { Role::User } else { Role::Assistant }, &format!("m{i}"))).collect();
        let mut turns = turns;
        turns.push(turn(Role::User, "dernière"));
        let messages = compose(&turns, Extra::default()).unwrap();
        assert!(messages.len() <= MAX_TURNS);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(text(messages.last().unwrap()), "dernière");
        assert_eq!(text(&messages[messages.len() - 2]), "m99");
    }

    #[test]
    fn the_window_and_the_file_join_the_question_only() {
        let extra = Extra {
            file: Some(("notes.md".into(), Loaded::Text("Liste des courses".into()))),
            context: Some(("Facture 42 — Mozilla Firefox".into(), "firefox".into(), "Total : 120 €".into())),
        };
        let messages = compose(&[turn(Role::User, "Avant"), turn(Role::Assistant, "Ok"), turn(Role::User, "Et ça ?")], extra).unwrap();
        assert_eq!(text(&messages[0]), "Avant");
        let question = text(&messages[2]);
        assert!(question.starts_with("Contexte joint par l'utilisateur : la fenêtre « Facture 42 — Mozilla Firefox » (firefox)."));
        assert!(question.contains("Total : 120 €"));
        assert!(question.contains("Fichier joint : notes.md\n\"\"\"\nListe des courses\n\"\"\""));
        assert!(question.ends_with("Et ça ?"));
        let image = Extra { file: Some(("chat.png".into(), Loaded::Image { mime: "image/png", base64: "iVBO".into() })), context: None };
        let messages = compose(&[], image).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(text(&messages[0]), "Image jointe : chat.png\n\nQu'en dis-tu ?");
        assert_eq!(messages[0].parts[1], Part::Image { mime: "image/png", base64: "iVBO".into() });
    }

    #[test]
    fn request_ids_are_plain() {
        assert!(valid_request_id("r-1728-ab"));
        assert!(!valid_request_id(""));
        assert!(!valid_request_id("r 1"));
        assert!(!valid_request_id(&"r".repeat(65)));
    }

    #[test]
    fn a_key_never_shows_in_an_error() {
        let key = "sk-ant-api03-SECRET-abcdef";
        assert_eq!(redact(format!("bad key {key} here"), Some(key)), "bad key ••• here");
        assert_eq!(redact("rien".into(), None), "rien");
        let headers = header_map(providers::headers(Provider::Anthropic, Some(key))).unwrap();
        assert!(headers["x-api-key"].is_sensitive());
        assert!(!format!("{headers:?}").contains("SECRET"));
        assert_eq!(missing_key(Provider::Gemini), "Ajoute ta clé Gemini dans Réglages › Chat.");
    }

    /// A tiny HTTP server on 127.0.0.1 that answers one request with `response`, in small
    /// pieces, and hands back the request it got.
    async fn serve_once(response: &'static str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let n = socket.read(&mut buffer).await.unwrap();
                request.extend_from_slice(&buffer[..n]);
                let text = String::from_utf8_lossy(&request);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length: usize = text
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.trim().parse().unwrap()))
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            for piece in response.as_bytes().chunks(9) {
                socket.write_all(piece).await.unwrap();
                socket.flush().await.unwrap();
            }
            let _ = socket.shutdown().await;
            String::from_utf8_lossy(&request).into_owned()
        });
        (url, task)
    }

    #[tokio::test]
    async fn an_answer_streams_piece_by_piece_from_a_local_server() {
        let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\nevent: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Bon\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"jour !\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let (url, server) = serve_once(response).await;
        let messages = compose(&[turn(Role::User, "Salut")], Extra::default()).unwrap();
        let body = providers::body(Provider::Anthropic, "claude-test", &messages);
        let headers = header_map(providers::headers(Provider::Anthropic, Some("sk-test-key"))).unwrap();
        let mut answer = String::new();
        stream_answer(&client(true).unwrap(), &url, Provider::Anthropic, headers, &body, |t| answer.push_str(t)).await.unwrap();
        assert_eq!(answer, "Bonjour !");
        let request = server.await.unwrap();
        assert!(request.starts_with("POST /chat HTTP/1.1"));
        assert!(request.to_ascii_lowercase().contains("x-api-key: sk-test-key"));
        assert!(request.contains("\"stream\":true"));
    }

    #[tokio::test]
    async fn ollama_lines_and_refusals_from_a_local_server() {
        let response = "HTTP/1.1 200 OK\r\ncontent-type: application/x-ndjson\r\nconnection: close\r\n\r\n{\"message\":{\"role\":\"assistant\",\"content\":\"Oui\"},\"done\":false}\n{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true}\n";
        let (url, server) = serve_once(response).await;
        let mut answer = String::new();
        let body = serde_json::json!({});
        stream_answer(&client(true).unwrap(), &url, Provider::Ollama, header_map(vec![]).unwrap(), &body, |t| answer.push_str(t))
            .await
            .unwrap();
        assert_eq!(answer, "Oui");
        server.await.unwrap();

        let refused = "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\ncontent-length: 34\r\nconnection: close\r\n\r\n{\"error\":\"model 'nope' not found\"}";
        let (url, server) = serve_once(refused).await;
        let error = stream_answer(&client(true).unwrap(), &url, Provider::Ollama, header_map(vec![]).unwrap(), &body, |_| {})
            .await
            .unwrap_err();
        assert_eq!(error, "Modèle introuvable chez Ollama. Choisis-en un autre.");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn nothing_listening_says_how_to_start_the_local_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/api/chat", listener.local_addr().unwrap());
        drop(listener);
        let error = stream_answer(&client(true).unwrap(), &url, Provider::Ollama, header_map(vec![]).unwrap(), &serde_json::json!({}), |_| {})
            .await
            .unwrap_err();
        assert!(error.starts_with("Ollama ne répond pas"));
    }

    #[test]
    fn a_remote_client_refuses_plain_http() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let error = runtime.block_on(async {
            client(false).unwrap().get("http://127.0.0.1:9/").send().await.unwrap_err()
        });
        assert!(error.is_builder(), "{error}");
    }

    #[test]
    fn what_joins_the_question_follows_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        let image = Extra { file: Some(("chat.png".into(), Loaded::Image { mime: "image/png", base64: "iVBO".into() })), context: None };
        let messages = compose(&[], image).unwrap();
        assert_eq!(text(&messages[0]), "Attached image: chat.png\n\nWhat do you think?");
        assert_eq!(compose(&[turn(Role::User, "  ")], Extra::default()).unwrap_err(), "Write a question");
    }
}
