//! Messages exchanged on the pipe: one JSON object per line, at most 256 KiB
//! (docs/02-ARCHITECTURE.md, "Le pont WSL ↔ Windows").

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead};
use std::time::Duration;

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
/// Longest string kept in a payload; longer ones are cut and the payload is flagged.
pub const MAX_STRING_CHARS: usize = 8 * 1024;
pub const TRUNCATED_FLAG: &str = "_ilot_truncated";

/// Sent by a relay. `wait` asks îlot to answer (permission requests).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    pub token: String,
    pub source: String,
    pub event: String,
    pub payload: Value,
    #[serde(default)]
    pub wait: bool,
    /// What the relay saw around the agent (terminal, editor, working folder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<RelayEnv>,
}

/// What the relay knows about where the agent runs, read from its own environment:
/// short strings, checked on both sides of the pipe, shown or compared, never run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayEnv {
    /// `WT_SESSION`: Windows Terminal's id of the tab or pane (a GUID). It says that the
    /// agent runs in Windows Terminal, not which tab: nothing maps it to one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wt_session: Option<String>,
    /// `TERM_PROGRAM`: `vscode` in the terminal of VS Code (and of editors built on it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_program: Option<String>,
    /// `CURSOR_VERSION` is set: the hook runs under Cursor, which also runs Claude Code's
    /// hooks (https://cursor.com/docs/hooks, "Environment Variables").
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cursor: bool,
    /// The relay's working folder: IBM Bob sends no `cwd` and runs its hooks in the
    /// task's folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Longest working folder kept.
const MAX_CWD_CHARS: usize = 1024;

impl RelayEnv {
    /// Reads the relay's environment through `var`; anything that does not look right is
    /// left out.
    pub fn read(var: impl Fn(&str) -> Option<String>, cwd: Option<String>) -> Self {
        Self {
            wt_session: var("WT_SESSION"),
            term_program: var("TERM_PROGRAM"),
            cursor: var("CURSOR_VERSION").is_some_and(|v| !v.trim().is_empty()),
            cwd,
        }
        .checked()
    }

    /// Keeps only values of the expected shape and size.
    pub fn checked(self) -> Self {
        let id = |c: char| c.is_ascii_hexdigit() || matches!(c, '-' | '{' | '}');
        let name = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
        Self {
            wt_session: self.wt_session.filter(|v| (1..=40).contains(&v.len()) && v.chars().all(id)),
            term_program: self.term_program.filter(|v| (1..=32).contains(&v.len()) && v.chars().all(name)),
            cursor: self.cursor,
            cwd: self.cwd.filter(|v| !v.is_empty() && v.chars().count() <= MAX_CWD_CHARS && !v.chars().any(char::is_control)),
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// îlot's answer to a waiting relay: the exact text the relay prints for the agent, or
/// nothing, in which case the agent asks in its own terminal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub output: Option<String>,
}

/// How long the relay waits for a permission: a little less than the hook's own timeout
/// (120 s), so that it always exits on its own terms.
pub const PERMISSION_WAIT: Duration = Duration::from_secs(110);
/// How long a question of Claude Code (`AskUserQuestion`) waits for an answer from îlot
/// (the hook's timeout is 60 s). Claude shows its own question only once the hook
/// returned: the wait stays short.
pub const QUESTION_WAIT: Duration = Duration::from_secs(55);

/// The relay argument of Claude Code's synchronous `PreToolUse` hook for
/// `AskUserQuestion` (the hook itself is `PreToolUse` with `matcher: "AskUserQuestion"`).
pub const QUESTION_EVENT: &str = "AskUserQuestion";

/// Events for which the relay waits for îlot's answer, and how long. Everything else is
/// fire and forget. Gemini CLI, Cursor, Copilot, OpenCode, Amp, IBM Bob and Kiro never
/// wait: îlot does not answer for them (docs/04-CONNECTEURS-IA.md, docs/AGENTS.md).
pub fn wait_for(source: &str, event: &str) -> Option<Duration> {
    match (source, event) {
        ("claude-code" | "codex", "PermissionRequest") => Some(PERMISSION_WAIT),
        ("claude-code", QUESTION_EVENT) => Some(QUESTION_WAIT),
        _ => None,
    }
}

pub fn waits_for_answer(source: &str, event: &str) -> bool {
    wait_for(source, event).is_some()
}

/// Serializes a message as one line, refusing anything larger than the limit.
pub fn encode_line<T: Serialize>(message: &T) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(message).map_err(io::Error::other)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    Ok(bytes)
}

/// Reads one line of at most `max` bytes (newline excluded). `None` at end of stream.
pub fn read_line_limited(reader: &mut impl BufRead, max: usize) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            return Ok((!line.is_empty()).then_some(line));
        }
        let (taken, done) = match chunk.iter().position(|&b| b == b'\n') {
            Some(i) => (i, true),
            None => (chunk.len(), false),
        };
        if line.len() + taken > max {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "line too long"));
        }
        line.extend_from_slice(&chunk[..taken]);
        reader.consume(taken + usize::from(done));
        if done {
            return Ok(Some(line));
        }
    }
}

/// Cuts every string longer than `max_chars` and marks the payload as truncated, so
/// that a huge file write still fits in a message and the interface can say so.
pub fn trim_payload(payload: &mut Value, max_chars: usize) {
    if trim_strings(payload, max_chars)
        && let Value::Object(map) = payload
    {
        map.insert(TRUNCATED_FLAG.to_owned(), Value::Bool(true));
    }
}

fn trim_strings(value: &mut Value, max_chars: usize) -> bool {
    match value {
        Value::String(text) => match text.char_indices().nth(max_chars) {
            Some((cut, _)) => {
                text.truncate(cut);
                text.push('…');
                true
            }
            None => false,
        },
        Value::Array(items) => items.iter_mut().fold(false, |acc, v| trim_strings(v, max_chars) | acc),
        Value::Object(map) => map.values_mut().fold(false, |acc, v| trim_strings(v, max_chars) | acc),
        _ => false,
    }
}

/// Compares two secrets without stopping at the first difference.
pub fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    fn request() -> Request {
        Request {
            version: PROTOCOL_VERSION,
            token: "abc".into(),
            source: "claude-code".into(),
            event: "PreToolUse".into(),
            payload: json!({ "tool_name": "Bash" }),
            wait: false,
            env: None,
        }
    }

    #[test]
    fn a_message_is_one_line_that_reads_back() {
        let bytes = encode_line(&request()).unwrap();
        assert_eq!(bytes.iter().filter(|&&b| b == b'\n').count(), 1);
        let line = read_line_limited(&mut Cursor::new(bytes), MAX_MESSAGE_BYTES).unwrap().unwrap();
        assert_eq!(serde_json::from_slice::<Request>(&line).unwrap(), request());
    }

    #[test]
    fn unknown_fields_are_ignored_and_wait_defaults_to_false() {
        let parsed: Request = serde_json::from_str(
            r#"{"version":1,"token":"t","source":"s","event":"e","payload":{},"extra":42}"#,
        )
        .unwrap();
        assert!(!parsed.wait);
    }

    #[test]
    fn oversized_messages_are_refused_both_ways() {
        let mut big = request();
        big.payload = json!({ "x": "a".repeat(MAX_MESSAGE_BYTES) });
        assert!(encode_line(&big).is_err());
        let line = vec![b'a'; MAX_MESSAGE_BYTES + 1];
        assert!(read_line_limited(&mut Cursor::new(line), MAX_MESSAGE_BYTES).is_err());
    }

    #[test]
    fn reads_successive_lines_and_stops_at_the_end() {
        let mut input = Cursor::new(b"one\ntwo\nthree".to_vec());
        assert_eq!(read_line_limited(&mut input, 10).unwrap().unwrap(), b"one");
        assert_eq!(read_line_limited(&mut input, 10).unwrap().unwrap(), b"two");
        assert_eq!(read_line_limited(&mut input, 10).unwrap().unwrap(), b"three");
        assert_eq!(read_line_limited(&mut input, 10).unwrap(), None);
    }

    #[test]
    fn long_strings_are_cut_and_flagged() {
        let mut payload = json!({ "tool_input": { "content": "é".repeat(20), "file_path": "a.txt" } });
        trim_payload(&mut payload, 5);
        assert_eq!(payload["tool_input"]["content"], "ééééé…");
        assert_eq!(payload["tool_input"]["file_path"], "a.txt");
        assert_eq!(payload[TRUNCATED_FLAG], true);
    }

    #[test]
    fn short_payloads_are_left_alone() {
        let mut payload = json!({ "tool_input": { "command": "ls" } });
        trim_payload(&mut payload, 5);
        assert_eq!(payload, json!({ "tool_input": { "command": "ls" } }));
    }

    #[test]
    fn only_permission_requests_and_claude_questions_wait() {
        assert_eq!(wait_for("claude-code", "PermissionRequest"), Some(PERMISSION_WAIT));
        assert_eq!(wait_for("codex", "PermissionRequest"), Some(PERMISSION_WAIT));
        assert_eq!(wait_for("claude-code", "AskUserQuestion"), Some(QUESTION_WAIT));
        assert!(QUESTION_WAIT < PERMISSION_WAIT);
        assert!(!waits_for_answer("claude-code", "PreToolUse"));
        assert!(!waits_for_answer("codex", "notify"));
        assert!(!waits_for_answer("codex", "AskUserQuestion"), "Codex questions are only shown");
        assert!(!waits_for_answer("gemini-cli", "PermissionRequest"));
        assert!(!waits_for_answer("gemini-cli", "Notification"));
        assert!(!waits_for_answer("generic", "PermissionRequest"));
        // The agents îlot only follows never wait, whatever they send.
        for source in ["cursor", "copilot", "opencode", "amp", "bob", "kiro"] {
            for event in ["PermissionRequest", "AskUserQuestion", "PreToolUse", "permissionRequest", "Stop"] {
                assert!(!waits_for_answer(source, event), "{source} {event}");
            }
        }
    }

    fn vars(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
    }

    #[test]
    fn the_relay_environment_is_read_and_bounded() {
        let env = RelayEnv::read(
            vars(&[("WT_SESSION", "8c2a1f0e-6b1d-4c1e-9b0a-1f2e3d4c5b6a"), ("TERM_PROGRAM", "vscode"), ("CURSOR_VERSION", "1.7.2")]),
            Some("/home/user/demo".into()),
        );
        assert_eq!(env.wt_session.as_deref(), Some("8c2a1f0e-6b1d-4c1e-9b0a-1f2e3d4c5b6a"));
        assert_eq!(env.term_program.as_deref(), Some("vscode"));
        assert!(env.cursor);
        assert_eq!(env.cwd.as_deref(), Some("/home/user/demo"));
        let odd = RelayEnv::read(
            vars(&[("WT_SESSION", "$(rm -rf ~)"), ("TERM_PROGRAM", "vs code; echo"), ("CURSOR_VERSION", " ")]),
            Some("/a\nb".into()),
        );
        assert!(odd.is_empty(), "{odd:?}");
        assert!(RelayEnv::read(vars(&[]), Some("x".repeat(2000))).cwd.is_none());
        let long_id = RelayEnv::read(vars(&[("WT_SESSION", "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa-aaaaa")]), None);
        assert!(long_id.wt_session.is_none(), "41 characters");
    }

    #[test]
    fn an_older_relay_without_environment_still_reads() {
        let parsed: Request = serde_json::from_str(r#"{"version":1,"token":"t","source":"s","event":"e","payload":{}}"#).unwrap();
        assert_eq!(parsed.env, None);
        let line = String::from_utf8(encode_line(&request()).unwrap()).unwrap();
        assert!(!line.contains("\"env\""), "nothing sent when there is nothing to say");
    }

    #[test]
    fn secrets_compare_exactly() {
        assert!(same_secret("0123abcd", "0123abcd"));
        assert!(!same_secret("0123abcd", "0123abce"));
        assert!(!same_secret("0123", "0123abcd"));
    }
}
