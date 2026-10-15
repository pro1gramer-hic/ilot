//! Gemini CLI: hooks in `~/.gemini/settings.json` (user settings: no warning per project),
//! activity only. Formats from https://geminicli.com/docs/hooks/reference/ and the code of
//! Gemini CLI 0.62.0, checked against real captures in `tests/fixtures/gemini-cli/`.
//!
//! No permissions from îlot: `BeforeTool` can refuse a tool, but its "allow" grants
//! nothing (Gemini still asks in its terminal). `Notification` (`ToolPermission`) only
//! says that Gemini asks: the session then shows "attend ton OK" until Gemini goes on.

use super::activity::{changed_lines, first_line, tool_activity};
use super::diff;
use super::hook_events::cut;
use super::hooks_json::{self, Call, HandlerFormat, HookSpec, ShellKind};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{Action, Activity, AgentEvent, EventKind, Source, SubagentStep};
use crate::locale::{Text, tr};
use serde_json::Value;
use std::io;
use std::path::Path;

/// Gemini's built-in subagents, which older versions expose as tools of their own name
/// (https://geminicli.com/docs/core/subagents).
const BUILT_IN_AGENTS: [&str; 4] = ["codebase_investigator", "cli_help", "generalist", "browser_agent"];

/// A subagent runs from `BeforeTool` to `AfterTool` of the `invoke_agent` tool, named by
/// `agent_name` (https://geminicli.com/docs/reference/policy-engine; payload read in the
/// code of Gemini CLI, `packages/core/src/agents/agent-tool.ts`). Neither event carries an
/// id: the start opens a subagent without one, the end closes the oldest such one. The
/// subagent's own tools look like the main agent's (no field tells them apart).
fn subagent_step(event: &str, tool: &str, input: &Value) -> Option<SubagentStep> {
    let kind = match tool {
        "invoke_agent" => text(input, "agent_name").map(str::to_owned),
        _ if BUILT_IN_AGENTS.contains(&tool) => Some(tool.to_owned()),
        _ => return None,
    };
    Some(if event == "BeforeTool" { SubagentStep::Start { id: None, kind } } else { SubagentStep::Stop { id: None } })
}

const SETTINGS: &str = ".gemini/settings.json";
/// Never `BeforeModel`, `AfterModel` or `BeforeToolSelection`: they run once per streamed
/// chunk and would slow Gemini down. Gemini waits for every hook (it has no background
/// mode); the relay gives up after about 2 s, so 5 s is a ceiling never reached.
const SPECS: [HookSpec<'static>; 7] = [
    HookSpec::new("SessionStart", 5, false),
    HookSpec::new("SessionEnd", 5, false),
    HookSpec::new("BeforeAgent", 5, false),
    HookSpec::new("AfterAgent", 5, false),
    HookSpec::new("BeforeTool", 5, false),
    HookSpec::new("AfterTool", 5, false),
    HookSpec::new("Notification", 5, false),
];

/// `command` is a command line for `bash -c` (WSL) or `powershell -Command` (Windows),
/// `timeout` is in milliseconds, and `name` lets the user spot îlot in `/hooks`.
fn format(target: &Target) -> HandlerFormat {
    let shell = if target.is_windows() { ShellKind::PowerShell } else { ShellKind::Posix };
    HandlerFormat { call: Call::Shell(shell), timeout_in_ms: true, background_key: None, name: Some("ilot-hook"), flat: false, version: None }
}

pub struct GeminiCli;

impl Connector for GeminiCli {
    fn id(&self) -> &'static str {
        "gemini-cli"
    }

    fn display_name(&self) -> String {
        "Gemini".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn status(&self, target: &Target) -> Status {
        hooks_json::status(&target.home_file(SETTINGS), &SPECS)
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        hooks_json::plug(&target.home_file(SETTINGS), &relay, self.id(), &SPECS, &format(target))
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        hooks_json::unplug(&target.home_file(SETTINGS))?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        let session_id = text(payload, "session_id")?;
        let cwd = text(payload, "cwd").unwrap_or_default();
        let new = |kind, title: Text| AgentEvent::new(Source::GeminiCli, session_id, cwd, kind, tr(title), at);
        let event = match event {
            "SessionStart" => new(EventKind::SessionStart, Text::AgentStarts),
            "SessionEnd" => new(EventKind::SessionEnd, Text::AgentClosedSession),
            "BeforeAgent" => AgentEvent {
                detail: text(payload, "prompt").map(first_line),
                activity: Some(Activity::new(Action::Think, None)),
                ..new(EventKind::Working, Text::VerbThink)
            },
            "AfterAgent" => new(EventKind::TurnDone, Text::AgentFinished),
            "BeforeTool" | "AfterTool" => {
                let unnamed = tr(Text::ToolUnnamed);
                let tool = text(payload, "tool_name").unwrap_or(unnamed.as_str());
                let input = payload.get("tool_input").unwrap_or(&Value::Null);
                let mut activity = tool_activity(tool, input);
                let mut changed = None;
                if event == "AfterTool" {
                    // Gemini reports no failure for a shell command: the lines count as written.
                    (activity.added, activity.removed) = changed_lines(tool, input, None);
                    changed = diff::from_tool(tool, input, cut(payload));
                }
                let title = activity.title();
                AgentEvent {
                    activity: Some(activity),
                    diff: changed,
                    subagent: subagent_step(event, tool, input),
                    ..AgentEvent::new(Source::GeminiCli, session_id, cwd, EventKind::Tool, title, at)
                }
            }
            "Notification" if text(payload, "notification_type") == Some("ToolPermission") => {
                let details = payload.get("details").unwrap_or(&Value::Null);
                let what = ["command", "filePath", "toolDisplayName"].iter().find_map(|key| text(details, key));
                AgentEvent {
                    detail: what.or_else(|| text(payload, "message")).map(first_line),
                    ..new(EventKind::Permission, Text::AgentWaitsForOkInTerminal)
                }
            }
            "Notification" => {
                AgentEvent::new(Source::GeminiCli, session_id, cwd, EventKind::Notification, text(payload, "message").map(first_line)?, at)
            }
            _ => return None,
        };
        Some(event)
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
    use crate::sessions::{SessionStatus, Sessions};
    use serde_json::json;

    fn fixture(event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/gemini-cli/{event}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn parse(event: &str) -> AgentEvent {
        GeminiCli.parse(event, &fixture(event), 1).unwrap()
    }

    const CAPTURES: [&str; 7] = ["SessionStart", "SessionEnd", "BeforeAgent", "AfterAgent", "BeforeTool", "AfterTool", "Notification"];

    #[test]
    fn every_real_capture_parses() {
        for event in CAPTURES {
            let parsed = parse(event);
            assert_eq!(parsed.source, Source::GeminiCli, "{event}");
            assert_eq!(parsed.project, "demo", "{event}");
            assert!(!parsed.session_id.is_empty(), "{event}");
        }
    }

    #[test]
    fn events_map_to_the_common_kinds() {
        assert_eq!(parse("SessionStart").kind, EventKind::SessionStart);
        assert_eq!(parse("BeforeAgent").kind, EventKind::Working);
        assert_eq!(parse("BeforeTool").kind, EventKind::Tool);
        assert_eq!(parse("AfterTool").kind, EventKind::Tool);
        assert_eq!(parse("AfterAgent").kind, EventKind::TurnDone);
        assert_eq!(parse("Notification").kind, EventKind::Permission);
        assert_eq!(parse("SessionEnd").kind, EventKind::SessionEnd);
    }

    #[test]
    fn tools_read_in_french_and_the_prompt_becomes_the_task() {
        assert_eq!(parse("BeforeTool").title, "lance ls");
        assert!(parse("BeforeAgent").detail.unwrap().starts_with("Do these steps in order"));
        let write = json!({ "session_id": "s", "cwd": "/p/demo", "tool_name": "write_file", "tool_input": { "file_path": "/p/demo/created.txt", "content": "hi\nthere" } });
        let event = GeminiCli.parse("AfterTool", &write, 1).unwrap();
        assert_eq!(event.title, "écrit created.txt");
        assert_eq!(event.activity.map(|a| (a.added, a.removed)), Some((2, 0)));
    }

    #[test]
    fn a_tool_permission_makes_the_session_wait_until_gemini_goes_on() {
        assert_eq!(parse("Notification").detail.as_deref(), Some("echo hello-ilot"));
        let in_session = |event: &str| AgentEvent { session_id: "s1".into(), ..parse(event) };
        let mut sessions = Sessions::default();
        sessions.apply(&in_session("BeforeAgent"));
        sessions.apply(&in_session("Notification"));
        assert_eq!(sessions.list()[0].status, SessionStatus::Waiting);
        sessions.apply(&in_session("AfterTool"));
        assert_eq!(sessions.list()[0].status, SessionStatus::Active);
    }

    #[test]
    fn a_subagent_works_from_before_to_after_invoke_agent() {
        // The real BeforeTool capture, with the tool Gemini runs a subagent through
        // (`invoke_agent`, `tool_input = {agent_name, prompt}`, read in Gemini CLI's code).
        let mut before = fixture("BeforeTool");
        before["tool_name"] = json!("invoke_agent");
        before["tool_input"] = json!({ "agent_name": "codebase_investigator", "prompt": "Map the auth flow" });
        let mut after = before.clone();
        after["hook_event_name"] = json!("AfterTool");
        let start = GeminiCli.parse("BeforeTool", &before, 1).unwrap();
        assert_eq!(start.subagent, Some(SubagentStep::Start { id: None, kind: Some("codebase_investigator".into()) }));
        assert_eq!(start.title, "délègue codebase_investigator");
        let mut sessions = Sessions::default();
        sessions.apply(&start);
        sessions.apply(&AgentEvent { at: 2, ..start.clone() });
        assert_eq!(sessions.list()[0].subagents.len(), 2, "two subagents at once");
        let stop = GeminiCli.parse("AfterTool", &after, 3).unwrap();
        assert_eq!(stop.subagent, Some(SubagentStep::Stop { id: None }));
        sessions.apply(&stop);
        assert_eq!(sessions.list()[0].subagents.len(), 1);
        // Older Gemini: the built-in subagent is a tool of its own name.
        let mut named = fixture("BeforeTool");
        named["tool_name"] = json!("generalist");
        assert_eq!(GeminiCli.parse("BeforeTool", &named, 1).unwrap().subagent, Some(SubagentStep::Start { id: None, kind: Some("generalist".into()) }));
        assert_eq!(GeminiCli.parse("BeforeTool", &fixture("BeforeTool"), 1).unwrap().subagent, None);
    }

    #[test]
    fn an_edit_keeps_its_diff() {
        let replace = json!({ "session_id": "s", "cwd": "/p/demo", "tool_name": "replace", "tool_input": { "file_path": "/p/demo/a.py", "old_string": "x = 1", "new_string": "x = 2" } });
        let diff = GeminiCli.parse("AfterTool", &replace, 1).unwrap().diff.unwrap();
        assert_eq!((diff.file.as_str(), diff.added, diff.removed), ("a.py", 1, 1));
        assert!(GeminiCli.parse("BeforeTool", &replace, 1).unwrap().diff.is_none(), "only once done");
    }

    #[test]
    fn other_notifications_and_unknown_events_are_quiet() {
        let mut payload = fixture("Notification");
        payload["notification_type"] = json!("Other");
        assert_eq!(GeminiCli.parse("Notification", &payload, 1).unwrap().kind, EventKind::Notification);
        assert!(GeminiCli.parse("AfterModel", &fixture("AfterAgent"), 1).is_none());
        assert!(GeminiCli.parse("AfterAgent", &json!({ "cwd": "/x" }), 1).is_none());
    }

    #[test]
    fn gemini_never_waits_for_an_answer() {
        assert!(GeminiCli.permission("Notification", &fixture("Notification")).is_none());
        assert!(GeminiCli.answer("BeforeTool", &fixture("BeforeTool"), Answer::Allow).is_none());
        assert!(!GeminiCli.capabilities().permissions);
    }

    fn temp_home(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ilot-gemini-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".gemini")).unwrap();
        dir
    }

    #[test]
    fn connect_then_disconnect_restores_a_copy_of_a_real_config() {
        let dir = temp_home("restore");
        let original = "{\n  \"security\": {\n    \"auth\": {\n      \"selectedType\": \"oauth-personal\"\n    }\n  },\n  \"hooks\": {\n    \"AfterAgent\": [\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"name\": \"notify\",\n            \"command\": \"notify-send done\",\n            \"timeout\": 3000\n          }\n        ]\n      }\n    ]\n  }\n}\n";
        let settings = dir.join(".gemini").join("settings.json");
        std::fs::write(&settings, original).unwrap();
        let target = Target::windows(dir.clone());
        assert_eq!(GeminiCli.status(&target), Status::Installed);
        GeminiCli.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(GeminiCli.status(&target), Status::Connected);
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let ours = &written["hooks"]["BeforeTool"][0]["hooks"][0];
        assert_eq!(ours["name"], "ilot-hook");
        assert_eq!(ours["timeout"], 5000);
        assert_eq!(ours["command"], format!("& '{}' gemini-cli BeforeTool", dir.join("ilot-hook.exe").display()));
        assert!(ours.get("args").is_none());
        assert_eq!(written["hooks"]["AfterAgent"].as_array().unwrap().len(), 2, "the user's own hook stays");
        for unwanted in ["BeforeModel", "AfterModel", "BeforeToolSelection"] {
            assert!(written["hooks"].get(unwanted).is_none(), "{unwanted}");
        }
        GeminiCli.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), original);
        assert_eq!(GeminiCli.status(&target), Status::Installed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_settings_file_created_for_ilot_leaves_with_it() {
        let dir = temp_home("created");
        let target = Target::windows(dir.clone());
        GeminiCli.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        let settings = dir.join(".gemini").join("settings.json");
        assert!(settings.exists());
        GeminiCli.disconnect(&target).unwrap();
        assert!(!settings.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn gemini_s_titles_follow_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(parse("SessionStart").title, "starts");
        assert_eq!(parse("AfterAgent").title, "finished");
        let prompt = parse("BeforeAgent");
        assert_eq!(prompt.activity.unwrap().verb, "thinks");
    }
}
