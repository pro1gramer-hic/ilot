//! The built relay itself: whatever it gets, it exits with 0 and prints nothing when îlot
//! gives no answer. Any other code is not neutral for the agents: Copilot CLI refuses the
//! tool on a non-zero `preToolUse`, Kiro starts another turn when `Stop` exits with 1,
//! Cursor blocks on 2 (https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-hooks-reference,
//! https://kiro.dev/docs/hooks/types, https://cursor.com/docs/hooks).
//!
//! `LOCALAPPDATA` points to an empty folder: the relay finds no token and never reaches an
//! îlot that would be running on this PC.

use std::io::Write;
use std::process::{Command, Stdio};

fn call(arguments: &[&str], stdin: &str, cursor: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ilot-hook"))
        .args(arguments)
        .env("LOCALAPPDATA", std::env::temp_dir().join("ilot-relay-test-no-token"))
        .env("CURSOR_VERSION", cursor)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A relay that stops reading early (no arguments) may close its input first.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

#[test]
fn the_relay_always_exits_with_zero_and_stays_silent() {
    let calls: [(&[&str], &str, &str); 8] = [
        (&[], "", ""),
        (&[], r#"{"hook_event_name":"stop","conversation_id":"c"}"#, "1.7.2"),
        (&["copilot", "PostToolUse"], "{not json", ""),
        (&["kiro", "Stop"], r#"{"hook_event_name":"Stop","session_id":"k1","cwd":"/p"}"#, ""),
        (&["claude-code", "AskUserQuestion"], r#"{"session_id":"s","tool_name":"AskUserQuestion","tool_input":{}}"#, ""),
        (&["claude-code", "PreToolUse"], r#"{"session_id":"s","tool_name":"Shell"}"#, "1.7.2"),
        (&["cursor", "stop"], r#"{"conversation_id":"c","status":"completed"}"#, ""),
        (&["codex", "notify", "{}"], "", ""),
    ];
    for (arguments, stdin, cursor) in calls {
        let out = call(arguments, stdin, cursor);
        assert_eq!(out.status.code(), Some(0), "{arguments:?}");
        assert!(out.stdout.is_empty(), "{arguments:?}: {}", String::from_utf8_lossy(&out.stdout));
        assert!(out.stderr.is_empty(), "{arguments:?}: {}", String::from_utf8_lossy(&out.stderr));
    }
}
