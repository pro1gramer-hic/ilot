//! GitHub Copilot CLI (`copilot`): a hooks file of îlot's own, `~/.copilot/hooks/ilot.json`
//! (`%USERPROFILE%\.copilot\hooks\` under Windows), activity only. Format from
//! https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-hooks-reference
//! (read on 11 October 2026).
//!
//! - `exec` + `args`: the relay starts without a shell, its arguments as they are.
//! - Events named in PascalCase get the « VS Code compatible » payload, shaped like
//!   Claude Code's (`session_id`, `cwd`, `tool_name`, `tool_input`): `hook_events` reads
//!   them. `subagentStart` and `subagentStop` exist in camel case only for a start:
//!   their payload says `sessionId` and `agentName`.
//! - No permission from îlot: `permissionRequest` runs before Copilot's own rules (îlot
//!   would see requests Copilot allows by itself). A question or an approval Copilot waits
//!   for in its terminal arrives as a `Notification` and is only shown.
//! - Any exit code but 0 from `preToolUse` refuses the tool: îlot plugs no `preToolUse`,
//!   and its relay always exits with 0.

use super::hook_events;
use super::hooks_json::{is_relay_program, to_pretty};
use super::owned_file::{OwnedFile, Owner};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, EventKind, Source, SubagentStep};
use crate::locale::{Text, tr};
use serde_json::{Map, Value, json};
use std::io;
use std::path::Path;

const EVENTS: [&str; 9] =
    ["SessionStart", "SessionEnd", "UserPromptSubmit", "PostToolUse", "PostToolUseFailure", "Stop", "Notification", "subagentStart", "subagentStop"];

const SOURCE: &str = "copilot";

fn owner(content: &str) -> Owner {
    let Ok(file) = serde_json::from_str::<Value>(content) else { return Owner::Other };
    let Some(hooks) = file.get("hooks").and_then(Value::as_object) else { return Owner::Other };
    let ours = |handler: &Value| {
        handler.get("exec").and_then(Value::as_str).is_some_and(is_relay_program)
            && handler.get("args").and_then(Value::as_array).and_then(|args| args.first()).and_then(Value::as_str) == Some(SOURCE)
    };
    let handlers: Vec<&Value> = hooks.values().filter_map(Value::as_array).flatten().collect();
    if handlers.is_empty() || !handlers.iter().all(|h| ours(h)) {
        return Owner::Other;
    }
    let same = hooks.len() == EVENTS.len() && EVENTS.iter().all(|event| hooks.contains_key(*event));
    if same { Owner::Current } else { Owner::Older }
}

const FILE: OwnedFile = OwnedFile { path: ".copilot/hooks/ilot.json", agent_dir: ".copilot", owner };

/// The hooks file, for the relay as Copilot sees it.
pub fn render(relay: &str) -> io::Result<String> {
    let hooks: Map<String, Value> = EVENTS
        .iter()
        .map(|event| ((*event).to_owned(), json!([{ "type": "command", "exec": relay, "args": [SOURCE, event], "timeoutSec": 10 }])))
        .collect();
    to_pretty(&json!({ "version": 1, "hooks": hooks }), "  ")
}

pub struct Copilot;

impl Connector for Copilot {
    fn id(&self) -> &'static str {
        SOURCE
    }

    fn display_name(&self) -> String {
        "Copilot".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn status(&self, target: &Target) -> Status {
        FILE.status(target)
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        FILE.write(target, &render(&relay)?)
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        FILE.remove(target)?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        let new = |session: &str, kind, title: Text| {
            AgentEvent::new(Source::Copilot, session, text(payload, "cwd").unwrap_or_default(), kind, tr(title), at)
        };
        match event {
            "Notification" => {
                let session = text(payload, "session_id")?;
                let title = match text(payload, "notification_type") {
                    Some("permission_prompt") => Text::AgentWaitsForOkInTerminal,
                    Some("elicitation_dialog") => Text::AgentAsksInTerminal,
                    _ => return Some(new(session, EventKind::Notification, Text::AgentNotification)),
                };
                Some(AgentEvent { detail: text(payload, "message").map(super::activity::first_line), ..new(session, EventKind::Asking, title) })
            }
            // Copilot names no subagent id at the start: each stop closes one started
            // without an id.
            "subagentStart" => {
                let kind = text(payload, "agentDisplayName").or_else(|| text(payload, "agentName")).map(str::to_owned);
                let base = new(text(payload, "sessionId")?, EventKind::Tool, Text::VerbDelegate);
                Some(AgentEvent { subagent: Some(SubagentStep::Start { id: None, kind }), ..base })
            }
            "subagentStop" => {
                let base = new(text(payload, "sessionId")?, EventKind::Tool, Text::AgentTakesOver);
                Some(AgentEvent { subagent: Some(SubagentStep::Stop { id: None }), ..base })
            }
            _ => hook_events::parse(Source::Copilot, event, payload, at),
        }
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

    /// Payloads built from the fields listed in the hooks reference (« VS Code compatible »
    /// payloads, `notification`, `subagentStart`, `subagentStop`), in tests/fixtures/copilot/.
    fn fixture(event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/copilot/{event}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn parse(event: &str) -> AgentEvent {
        Copilot.parse(event, &fixture(event), 1).unwrap()
    }

    #[test]
    fn every_documented_event_parses_into_one_session() {
        for event in ["SessionStart", "UserPromptSubmit", "PostToolUse", "Notification", "Stop", "subagentStart", "subagentStop"] {
            let parsed = parse(event);
            assert_eq!((parsed.source, parsed.project.as_str()), (Source::Copilot, "demo"), "{event}");
            assert_eq!(parsed.session_id, "2f6c8d0e-3b7a-4e55-9a1c-5f2d7b8e9c01", "{event}");
        }
        assert_eq!(parse("UserPromptSubmit").detail.as_deref(), Some("Add a test for the parser"));
        assert_eq!(parse("PostToolUse").title, "lance npm test");
        assert_eq!(parse("Stop").kind, EventKind::TurnDone);
    }

    #[test]
    fn an_approval_waits_in_the_terminal_and_is_only_shown() {
        let waiting = parse("Notification");
        assert_eq!(waiting.kind, EventKind::Asking);
        assert_eq!(waiting.detail.as_deref(), Some("Copilot needs your approval to run: rm -rf build"));
        let mut idle = fixture("Notification");
        idle["notification_type"] = json!("agent_idle");
        assert_eq!(Copilot.parse("Notification", &idle, 1).unwrap().kind, EventKind::Notification);
        assert!(!Copilot.capabilities().permissions);
        assert!(Copilot.permission("Notification", &fixture("Notification")).is_none());
        assert!(!EVENTS.contains(&"permissionRequest") && !EVENTS.contains(&"preToolUse") && !EVENTS.contains(&"PreToolUse"));
    }

    #[test]
    fn subagents_open_and_close() {
        assert_eq!(parse("subagentStart").subagent, Some(SubagentStep::Start { id: None, kind: Some("Test writer".into()) }));
        assert_eq!(parse("subagentStop").subagent, Some(SubagentStep::Stop { id: None }));
    }

    #[test]
    fn the_hooks_file_starts_the_relay_without_a_shell() {
        let file: Value = serde_json::from_str(&render(r"C:\Users\HP 240\AppData\Local\îlot\ilot-hook.exe").unwrap()).unwrap();
        assert_eq!(file["version"], 1);
        assert_eq!(
            file["hooks"]["Stop"][0],
            json!({ "type": "command", "exec": r"C:\Users\HP 240\AppData\Local\îlot\ilot-hook.exe", "args": ["copilot", "Stop"], "timeoutSec": 10 })
        );
        assert_eq!(owner(&render("/home/u/.local/bin/ilot-hook").unwrap()), Owner::Current);
        assert_eq!(owner("{\"version\":1,\"hooks\":{\"Stop\":[{\"type\":\"command\",\"bash\":\"./mine.sh\"}]}}"), Owner::Other);
        let older = "{\"version\":1,\"hooks\":{\"Stop\":[{\"type\":\"command\",\"exec\":\"/home/u/.local/bin/ilot-hook\",\"args\":[\"copilot\",\"Stop\"]}]}}";
        assert_eq!(owner(older), Owner::Older);
    }

    #[test]
    fn connect_then_disconnect_leaves_the_folder_as_it_was() {
        let dir = std::env::temp_dir().join(format!("ilot-copilot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".copilot").join("hooks")).unwrap();
        let theirs = dir.join(".copilot").join("hooks").join("lint.json");
        std::fs::write(&theirs, "{\"version\":1,\"hooks\":{}}").unwrap();
        let target = Target::windows(dir.clone());
        assert_eq!(Copilot.status(&target), Status::Installed);
        Copilot.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(Copilot.status(&target), Status::Connected);
        Copilot.disconnect(&target).unwrap();
        assert!(!dir.join(".copilot").join("hooks").join("ilot.json").exists());
        assert_eq!(std::fs::read_to_string(&theirs).unwrap(), "{\"version\":1,\"hooks\":{}}");
        assert_eq!(Copilot.status(&target), Status::Installed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_approval_is_announced_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let payload = serde_json::json!({ "session_id": "s1", "cwd": "/p", "notification_type": "permission_prompt", "message": "Allow?" });
        assert_eq!(Copilot.parse("Notification", &payload, 1).unwrap().title, "waits for your OK in its terminal");
    }
}
