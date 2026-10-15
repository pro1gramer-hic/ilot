//! `ilot-hook <source> <event>`: called by an agent's hook with the event JSON on stdin
//! (or, for Codex `notify`, as the last argument). Forwards it to îlot over the pipe.
//!
//! It never decides anything itself. Whatever happens (îlot closed, timeout, error, even
//! a crash), it exits with code 0 and prints nothing, so the agent goes on as if there
//! were no hook and asks in its own terminal (docs/02-ARCHITECTURE.md, point 3 of the
//! bridge). Another code would not be neutral: Copilot CLI refuses the tool, Kiro starts
//! another turn after `Stop`, Cursor blocks on 2 (docs/AGENTS.md). It prints only the
//! exact answer îlot sends for a request the user answered.
//!
//! Cursor also runs the hooks of `~/.claude/settings.json` (enabled by default,
//! https://cursor.com/docs/reference/third-party-hooks): an event that comes with
//! `CURSOR_VERSION` in the environment is Cursor's, never Claude's, and never waits.

use ilot_bridge::client;
use ilot_bridge::protocol::{MAX_MESSAGE_BYTES, MAX_STRING_CHARS, RelayEnv, Reply, trim_payload, wait_for};
use serde_json::Value;
use std::io::{self, Read, Write};
use std::time::Duration;

/// Hard cap on what we read from the agent before trimming long strings.
const MAX_INPUT_BYTES: u64 = 8 * 1024 * 1024;
/// Fire-and-forget events only wait for îlot's receipt.
const EVENT_TIMEOUT: Duration = Duration::from_secs(2);

fn main() {
    // A crash prints nothing either: the agent must not read a panic as an answer.
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let output = std::panic::catch_unwind(|| run(args, &RelayEnv::read(|name| std::env::var(name).ok(), working_folder())))
        .ok()
        .flatten();
    if let Some(output) = output {
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(output.as_bytes());
        let _ = stdout.flush();
    }
    std::process::exit(0);
}

fn working_folder() -> Option<String> {
    std::env::current_dir().ok().and_then(|path| path.to_str().map(str::to_owned))
}

fn run(args: Vec<String>, env: &RelayEnv) -> Option<String> {
    // Without arguments only Cursor is expected (its hooks may drop Claude's `args`).
    if args.len() < 2 && !env.cursor {
        return None;
    }
    let payload = match args.get(2) {
        Some(json) => parse_payload(json.as_bytes())?,
        None => {
            let mut input = Vec::new();
            io::stdin().take(MAX_INPUT_BYTES).read_to_end(&mut input).ok()?;
            parse_payload(&input)?
        }
    };
    let (source, event) = route(&args, &payload, env.cursor)?;
    let limit = wait_for(&source, &event);
    let config = client::local_config(limit.unwrap_or(EVENT_TIMEOUT)).ok()?;
    let mut request = client::request(&config.token, &source, &event, payload, limit.is_some());
    request.env = Some(env.clone()).filter(|env| !env.is_empty());
    printable(limit.is_some(), client::send(&config, &request).ok()??)
}

/// The source and the event of a hook call: its two arguments, or, under Cursor without
/// them, the event named in the payload. Under Cursor, Claude Code's hooks speak for
/// Cursor.
fn route(args: &[String], payload: &Value, cursor: bool) -> Option<(String, String)> {
    let (source, event) = match (args.first(), args.get(1)) {
        (Some(source), Some(event)) => (source.clone(), event.clone()),
        _ if !cursor => return None,
        _ => {
            let named = payload.get("hook_event_name").and_then(Value::as_str)?;
            let plain = !named.is_empty() && named.len() <= 64 && named.chars().all(|c| c.is_ascii_alphanumeric());
            plain.then(|| ("cursor".to_owned(), named.to_owned()))?
        }
    };
    if cursor && source == "claude-code" {
        return Some(("cursor".to_owned(), event));
    }
    Some((source, event))
}

/// Only the answer to a request the relay waited for is printed, whatever îlot sends:
/// Claude Code adds the plain output of some hooks (`SessionStart`, `UserPromptSubmit`)
/// to Claude's context (https://code.claude.com/docs/en/hooks, "Exit code 0").
fn printable(wait: bool, reply: Reply) -> Option<String> {
    reply.output.filter(|_| wait)
}

/// Parses the agent's JSON and trims it so the message fits on the pipe.
fn parse_payload(bytes: &[u8]) -> Option<Value> {
    let mut payload: Value = serde_json::from_slice(bytes).ok()?;
    trim_payload(&mut payload, MAX_STRING_CHARS);
    (serde_json::to_vec(&payload).ok()?.len() < MAX_MESSAGE_BYTES - 1024).then_some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    fn routed(list: &[&str], payload: Value, cursor: bool) -> Option<(String, String)> {
        route(&args(list), &payload, cursor)
    }

    #[test]
    fn missing_arguments_give_nothing() {
        assert_eq!(run(vec![], &RelayEnv::default()), None);
        assert_eq!(run(args(&["claude-code"]), &RelayEnv::default()), None);
    }

    #[test]
    fn invalid_json_gives_nothing() {
        assert_eq!(run(args(&["codex", "notify", "{not json"]), &RelayEnv::default()), None);
    }

    #[test]
    fn claude_hooks_run_by_cursor_speak_for_cursor() {
        // A real Claude Code payload (tests/fixtures/claude-code/PostToolUse.json), as
        // Cursor runs the hooks of ~/.claude/settings.json with CURSOR_VERSION set.
        let payload = json!({ "session_id": "s1", "hook_event_name": "PostToolUse", "tool_name": "Bash" });
        assert_eq!(routed(&["claude-code", "PostToolUse"], payload.clone(), true), Some(("cursor".into(), "PostToolUse".into())));
        assert_eq!(routed(&["claude-code", "PostToolUse"], payload.clone(), false), Some(("claude-code".into(), "PostToolUse".into())));
        // Cursor may drop `args`: the event comes from the payload.
        assert_eq!(routed(&[], payload.clone(), true), Some(("cursor".into(), "PostToolUse".into())));
        assert_eq!(routed(&[], payload, false), None, "not Cursor: no guess");
        assert_eq!(routed(&[], json!({ "hook_event_name": "Stop; rm -rf ~" }), true), None);
        // Cursor's own hooks keep their source.
        assert_eq!(routed(&["cursor", "stop"], json!({}), true), Some(("cursor".into(), "stop".into())));
    }

    #[test]
    fn under_cursor_nothing_ever_waits() {
        for event in ["PermissionRequest", "AskUserQuestion"] {
            let (source, event) = routed(&["claude-code", event], json!({}), true).unwrap();
            assert_eq!(wait_for(&source, &event), None, "{event}");
        }
        assert!(wait_for("claude-code", "AskUserQuestion").is_some());
    }

    #[test]
    fn only_an_awaited_answer_is_printed() {
        let reply = || Reply { output: Some("{\"decision\":\"allow\"}".into()) };
        assert_eq!(printable(true, reply()).as_deref(), Some("{\"decision\":\"allow\"}"));
        // A SessionStart or UserPromptSubmit event stays silent even if îlot answers it.
        assert_eq!(printable(false, reply()), None);
        assert_eq!(printable(true, Reply::default()), None);
    }

    #[test]
    fn huge_payloads_are_trimmed_to_fit() {
        let big = format!(r#"{{"tool_input":{{"content":"{}"}}}}"#, "a".repeat(1_000_000));
        let payload = parse_payload(big.as_bytes()).unwrap();
        assert!(serde_json::to_vec(&payload).unwrap().len() < MAX_MESSAGE_BYTES);
        assert_eq!(payload["_ilot_truncated"], true);
    }
}
