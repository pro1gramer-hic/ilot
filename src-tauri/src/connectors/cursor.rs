//! Cursor (the editor and its CLI, `agent`): hooks in `~/.cursor/hooks.json`, activity only.
//! Formats from https://cursor.com/docs/hooks (read on 11 October 2026).
//!
//! - The handlers sit right under the event (`{"version": 1, "hooks": {"stop": [{"command":
//!   …}]}}`); `command` is a command line.
//! - Only the events that report what happened are plugged. Never `preToolUse`,
//!   `subagentStart`, `beforeShellExecution`, `beforeMCPExecution` nor `beforeReadFile`:
//!   on those « permission hooks », an answer Cursor cannot read blocks the action. Without
//!   `subagentStart`, Cursor's subagents are not counted.
//! - Cursor also runs the hooks of `~/.claude/settings.json` (« Include Third-Party
//!   Plugins », on by default, https://cursor.com/docs/reference/third-party-hooks): the
//!   relay sees `CURSOR_VERSION` and hands those events here, under their Claude names. They
//!   only count until Cursor's own hooks speak for the conversation (`Core::handle`).
//! - No permission from îlot: there is no event where Cursor waits for the user.

use super::activity::{changed_lines, edits_counts, first_line, tool_activity};
use super::diff;
use super::hook_events::cut;
use super::hooks_json::{self, Call, HandlerFormat, HookSpec, ShellKind};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{Action, Activity, AgentEvent, EventKind, Source};
use crate::locale::{Text, tr};
use serde_json::Value;
use std::io;
use std::path::Path;

const HOOKS: &str = ".cursor/hooks.json";
/// Cursor's own events; each one waits for the relay (about 60 ms through WSL, 2 s at most).
const SPECS: [HookSpec<'static>; 8] = [
    HookSpec::new("sessionStart", 10, false),
    HookSpec::new("sessionEnd", 10, false),
    HookSpec::new("beforeSubmitPrompt", 10, false),
    HookSpec::new("postToolUse", 10, false),
    HookSpec::new("postToolUseFailure", 10, false),
    HookSpec::new("afterShellExecution", 10, false),
    HookSpec::new("afterFileEdit", 10, false),
    HookSpec::new("stop", 10, false),
];

/// One command line: for `sh` inside WSL (the CLI), for `cmd.exe` under Windows (the
/// editor, an Electron app: to check on a real PC, the page does not say).
fn format(target: &Target) -> HandlerFormat {
    let shell = if target.is_windows() { ShellKind::Cmd } else { ShellKind::Posix };
    HandlerFormat { call: Call::Shell(shell), timeout_in_ms: false, background_key: None, name: None, flat: true, version: Some(1) }
}

/// True for an event Cursor sent through Claude Code's hooks (Claude's names start with a
/// capital letter, Cursor's own with a small one).
pub fn imported(event: &str) -> bool {
    event.starts_with(|c: char| c.is_ascii_uppercase())
}

/// The conversation an event belongs to: `conversation_id` (every event), or
/// `session_id` (« same as conversation_id », sessionStart and sessionEnd).
pub fn conversation(payload: &Value) -> Option<&str> {
    text(payload, "conversation_id").or_else(|| text(payload, "session_id"))
}

pub struct Cursor;

impl Connector for Cursor {
    fn id(&self) -> &'static str {
        "cursor"
    }

    fn display_name(&self) -> String {
        "Cursor".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn status(&self, target: &Target) -> Status {
        hooks_json::status(&target.home_file(HOOKS), &SPECS)
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        hooks_json::plug(&target.home_file(HOOKS), &relay, self.id(), &SPECS, &format(target))
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        hooks_json::unplug_with_root(&target.home_file(HOOKS), &["version"])?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        let session = conversation(payload)?;
        let root = payload.get("workspace_roots").and_then(Value::as_array).and_then(|roots| roots.first()).and_then(Value::as_str);
        let cwd = text(payload, "cwd").or(root).unwrap_or_default();
        let new = |kind, title: Text| AgentEvent::new(Source::Cursor, session, cwd, kind, tr(title), at);
        let tool = |activity: Activity| {
            let title = activity.title();
            AgentEvent { activity: Some(activity), ..AgentEvent::new(Source::Cursor, session, cwd, EventKind::Tool, title, at) }
        };
        Some(match event {
            "sessionStart" | "SessionStart" => new(EventKind::SessionStart, Text::AgentStarts),
            "sessionEnd" | "SessionEnd" => new(EventKind::SessionEnd, Text::AgentClosedSession),
            "beforeSubmitPrompt" | "UserPromptSubmit" => AgentEvent {
                detail: text(payload, "prompt").map(first_line),
                activity: Some(Activity::new(Action::Think, None)),
                ..new(EventKind::Working, Text::VerbThink)
            },
            "postToolUse" | "postToolUseFailure" | "PostToolUse" => {
                let unnamed = tr(Text::ToolUnnamed);
                let name = text(payload, "tool_name").unwrap_or(unnamed.as_str());
                let input = payload.get("tool_input").unwrap_or(&Value::Null);
                let mut activity = tool_activity(name, input);
                // Cursor's own `afterFileEdit` counts the lines; Claude's hooks have only this.
                let changed = if event == "PostToolUse" {
                    (activity.added, activity.removed) = changed_lines(name, input, None);
                    diff::from_tool(name, input, cut(payload))
                } else {
                    None
                };
                AgentEvent {
                    detail: (event == "postToolUseFailure").then(|| text(payload, "error_message").map(first_line)).flatten(),
                    diff: changed,
                    ..tool(activity)
                }
            }
            "afterShellExecution" => tool(tool_activity("Shell", payload)),
            "afterFileEdit" => {
                let path = text(payload, "file_path")?;
                let edits = payload.get("edits").unwrap_or(&Value::Null);
                let (added, removed) = edits_counts(edits);
                let activity = Activity { added, removed, ..tool_activity("Edit", payload) };
                AgentEvent { diff: diff::from_edits(path, edits, cut(payload)), ..tool(activity) }
            }
            "stop" | "Stop" => match text(payload, "status") {
                Some("aborted") => new(EventKind::Interrupted, Text::AgentInterrupted),
                Some("error") => new(EventKind::Error, Text::AgentFailed),
                _ => new(EventKind::TurnDone, Text::AgentFinished),
            },
            _ => return None,
        })
    }

    fn permission(&self, _event: &str, _payload: &Value) -> Option<PermissionView> {
        None
    }

    fn answer(&self, _event: &str, _payload: &Value, _answer: Answer) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::diff::LineKind;

    /// Payloads built from the examples of https://cursor.com/docs/hooks (« Common
    /// schema » and each hook's input), in `tests/fixtures/cursor/`.
    fn fixture(event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/cursor/{event}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn parse(event: &str) -> AgentEvent {
        Cursor.parse(event, &fixture(event), 1).unwrap()
    }

    const EVENTS: [&str; 8] =
        ["sessionStart", "beforeSubmitPrompt", "postToolUse", "postToolUseFailure", "afterShellExecution", "afterFileEdit", "stop", "sessionEnd"];

    #[test]
    fn every_documented_event_parses_into_one_conversation() {
        for event in EVENTS {
            let parsed = parse(event);
            assert_eq!(parsed.source, Source::Cursor, "{event}");
            assert_eq!(parsed.session_id, "668320d2-2fd8-4888-b33c-2a466fec86e7", "{event}");
            assert_eq!(parsed.project, "demo", "{event}");
        }
        assert!(SPECS.iter().all(|spec| EVENTS.contains(&spec.event)), "every plugged event is understood");
    }

    #[test]
    fn events_map_to_the_common_kinds() {
        assert_eq!(parse("sessionStart").kind, EventKind::SessionStart);
        assert_eq!(parse("beforeSubmitPrompt").detail.as_deref(), Some("Fix the failing tests in api.py"));
        assert_eq!(parse("postToolUse").title, "lance npm test");
        assert_eq!(parse("postToolUseFailure").detail.as_deref(), Some("Command timed out after 30s"));
        assert_eq!(parse("afterShellExecution").title, "lance npm test");
        assert_eq!(parse("stop").kind, EventKind::TurnDone);
        assert_eq!(parse("sessionEnd").kind, EventKind::SessionEnd);
        let mut aborted = fixture("stop");
        aborted["status"] = "aborted".into();
        assert_eq!(Cursor.parse("stop", &aborted, 1).unwrap().kind, EventKind::Interrupted);
        aborted["status"] = "error".into();
        assert_eq!(Cursor.parse("stop", &aborted, 1).unwrap().kind, EventKind::Error);
        assert!(Cursor.parse("preToolUse", &fixture("postToolUse"), 1).is_none());
    }

    #[test]
    fn a_file_edit_counts_its_lines_and_keeps_its_diff() {
        let edit = parse("afterFileEdit");
        assert_eq!(edit.title, "édite api.py");
        let activity = edit.activity.unwrap();
        assert_eq!((activity.added, activity.removed), (2, 1));
        let diff = edit.diff.unwrap();
        assert_eq!(diff.file, "api.py");
        assert_eq!(diff.lines.iter().map(|l| l.kind).collect::<Vec<_>>(), [LineKind::Del, LineKind::Add, LineKind::Add]);
    }

    #[test]
    fn claude_hooks_run_by_cursor_count_as_cursor() {
        // A real Claude Code payload (tests/fixtures/claude-code/), as the relay hands it
        // over when CURSOR_VERSION is set.
        let path = format!("{}/../tests/fixtures/claude-code/UserPromptSubmit.json", env!("CARGO_MANIFEST_DIR"));
        let claude: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let event = Cursor.parse("UserPromptSubmit", &claude, 1).unwrap();
        assert_eq!((event.source, event.kind), (Source::Cursor, EventKind::Working));
        assert!(imported("UserPromptSubmit") && !imported("beforeSubmitPrompt"));
        let edit = serde_json::json!({ "session_id": "s", "cwd": "/p/demo", "tool_name": "Write", "tool_input": { "file_path": "/p/demo/a.md", "content": "x\ny" } });
        let event = Cursor.parse("PostToolUse", &edit, 1).unwrap();
        assert_eq!(event.activity.map(|a| a.added), Some(2));
        assert!(event.diff.is_some());
    }

    #[test]
    fn cursor_is_followed_only() {
        assert!(!Cursor.capabilities().permissions);
        assert!(Cursor.permission("stop", &fixture("stop")).is_none());
        assert!(Cursor.answer("stop", &fixture("stop"), Answer::Allow).is_none());
        for unsafe_event in ["preToolUse", "subagentStart", "beforeShellExecution", "beforeMCPExecution", "beforeReadFile"] {
            assert!(SPECS.iter().all(|s| s.event != unsafe_event), "{unsafe_event} is a permission hook");
        }
    }

    #[test]
    fn connect_then_disconnect_restores_the_user_s_hooks() {
        let dir = std::env::temp_dir().join(format!("ilot-cursor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".cursor")).unwrap();
        let original = "{\n  \"version\": 1,\n  \"hooks\": {\n    \"afterFileEdit\": [\n      {\n        \"command\": \"./format.sh\"\n      }\n    ]\n  }\n}\n";
        let hooks = dir.join(".cursor").join("hooks.json");
        std::fs::write(&hooks, original).unwrap();
        let target = Target::windows(dir.clone());
        assert_eq!(Cursor.status(&target), Status::Installed);
        Cursor.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(Cursor.status(&target), Status::Connected);
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&hooks).unwrap()).unwrap();
        let relay = dir.join("ilot-hook.exe").display().to_string();
        assert_eq!(written["hooks"]["afterFileEdit"][1], serde_json::json!({ "command": format!("\"{relay}\" cursor afterFileEdit"), "timeout": 10 }));
        assert_eq!(std::fs::read_to_string(hooks_json::backup_path(&hooks)).unwrap(), original);
        Cursor.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&hooks).unwrap(), original);
        // A file created for îlot leaves with it, `version` included.
        std::fs::remove_file(&hooks).unwrap();
        std::fs::remove_file(hooks_json::backup_path(&hooks)).unwrap();
        Cursor.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        Cursor.disconnect(&target).unwrap();
        assert!(!hooks.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cursor_s_titles_follow_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(parse("sessionStart").title, "starts");
        assert_eq!(parse("afterShellExecution").title, "runs npm test");
    }
}
