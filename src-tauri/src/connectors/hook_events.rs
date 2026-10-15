//! Hook events shaped like Claude Code's. Codex hooks use the same event names, the same
//! common fields (`session_id`, `cwd`, `tool_name`, `tool_input`…) and the same
//! `PermissionRequest` answers (docs/04-CONNECTEURS-IA.md), so both connectors read them
//! here and only add what is their own. Copilot CLI (its « VS Code compatible » events),
//! Kiro, and the plugins îlot writes for OpenCode and Amp speak the same way
//! (docs/AGENTS.md).

use super::activity::{changed_lines, first_line, tool_activity};
use super::diff;
use super::{Answer, PermissionView, text};
use crate::events::{Action, Activity, AgentEvent, EventKind, Source, SubagentStep};
use crate::locale::{Text, tr};
use crate::risk;
use ilot_bridge::protocol::{QUESTION_EVENT, TRUNCATED_FLAG};
use serde_json::{Value, json};

pub const DENY_MESSAGE: &str = "Refusé depuis îlot par l'utilisateur.";
pub const SUSPENDED_MESSAGE: &str = "Refusé : îlot a tout suspendu.";

/// The relay cut the payload: what it holds is not whole.
pub fn cut(payload: &Value) -> bool {
    payload.get(TRUNCATED_FLAG).and_then(Value::as_bool).unwrap_or(false)
}

/// The first question of a tool that asks the user (Claude's `AskUserQuestion`, Codex's
/// `request_user_input`: both list `questions` with a `question` text).
pub fn first_question(input: &Value) -> Option<String> {
    input.get("questions")?.as_array()?.first().and_then(|q| text(q, "question")).map(first_line)
}

/// Translates one hook call; `None` for events the island does not show. Tools run by a
/// subagent carry the parent's `session_id`: they stay in the parent's session.
pub fn parse(source: Source, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
    let session_id = text(payload, "session_id")?;
    let cwd = text(payload, "cwd").unwrap_or_default();
    let new = |kind, title: Text| AgentEvent::new(source, session_id, cwd, kind, tr(title), at);
    let subagent = subagent_step(event, payload);
    let unnamed = tr(Text::ToolUnnamed);
    let tool = text(payload, "tool_name").unwrap_or(unnamed.as_str());
    let input = payload.get("tool_input").unwrap_or(&Value::Null);
    let event = match event {
        "SessionStart" => new(EventKind::SessionStart, Text::AgentStarts),
        "SessionEnd" => new(EventKind::SessionEnd, Text::AgentClosedSession),
        "UserPromptSubmit" => AgentEvent {
            detail: text(payload, "prompt").map(first_line),
            activity: Some(Activity::new(Action::Think, None)),
            ..new(EventKind::Working, Text::VerbThink)
        },
        // Codex's question (experimental): îlot cannot answer it through a hook, it only
        // says that Codex waits in its terminal (docs/AGENTS.md).
        "PreToolUse" if tool == "request_user_input" => {
            AgentEvent { detail: first_question(input), ..new(EventKind::Asking, Text::AgentAsksInTerminal) }
        }
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure" => {
            let mut activity = tool_activity(tool, input);
            let mut changed = None;
            if event == "PostToolUse" {
                (activity.added, activity.removed) = changed_lines(tool, input, payload.get("tool_response"));
                changed = diff::from_tool(tool, input, cut(payload));
            }
            let title = activity.title();
            AgentEvent {
                detail: (event == "PostToolUseFailure").then(|| text(payload, "error").map(first_line)).flatten(),
                activity: Some(activity),
                diff: changed,
                ..AgentEvent::new(source, session_id, cwd, EventKind::Tool, title, at)
            }
        }
        // Claude's synchronous hook for `AskUserQuestion`: the island asks in its place.
        QUESTION_EVENT => AgentEvent { detail: first_question(input), ..new(EventKind::Permission, Text::AgentAsks) },
        "PermissionRequest" => new(EventKind::Permission, Text::AgentWaitsForOk),
        "Notification" => AgentEvent::new(source, session_id, cwd, EventKind::Notification, text(payload, "message").map(first_line)?, at),
        "Stop" => new(EventKind::TurnDone, Text::AgentFinished),
        "StopFailure" => AgentEvent { detail: text(payload, "error").map(str::to_owned), ..new(EventKind::Error, Text::AgentFailed) },
        "SubagentStart" => {
            let agent_type = text(payload, "agent_type").map(str::to_owned);
            AgentEvent {
                detail: agent_type.clone(),
                activity: Some(Activity::new(Action::Delegate, agent_type)),
                ..new(EventKind::Tool, Text::VerbDelegate)
            }
        }
        "SubagentStop" => new(EventKind::Tool, Text::AgentTakesOver),
        _ => return None,
    };
    Some(AgentEvent { subagent, ..event })
}

/// `SubagentStart` and `SubagentStop` open and close a subagent; any other event fired
/// inside one carries its `agent_id` and `agent_type`.
fn subagent_step(event: &str, payload: &Value) -> Option<SubagentStep> {
    let id = text(payload, "agent_id").map(str::to_owned);
    let kind = text(payload, "agent_type").map(str::to_owned);
    match event {
        "SubagentStart" => Some(SubagentStep::Start { id, kind }),
        "SubagentStop" => Some(SubagentStep::Stop { id }),
        _ => id.map(|id| SubagentStep::Seen { id, kind }),
    }
}

/// What the island shows for a `PermissionRequest`, with the risk of a shell command.
/// "Toujours" is left to the connector.
pub fn permission_view(agent: &str, payload: &Value) -> Option<PermissionView> {
    let tool = text(payload, "tool_name")?;
    let input = payload.get("tool_input").cloned().unwrap_or(Value::Null);
    let (title, detail) = match tool {
        "Bash" => (Text::WantsToRun { agent }, text(&input, "command")?.to_owned()),
        "Edit" if input.get("replace_all").and_then(Value::as_bool) == Some(true) => {
            (Text::WantsToEditEverywhere { agent }, text(&input, "file_path")?.to_owned())
        }
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => (
            Text::WantsToEditFile { agent },
            text(&input, "file_path").or_else(|| text(&input, "notebook_path"))?.to_owned(),
        ),
        "apply_patch" => (Text::WantsToEditFiles { agent }, text(&input, "command")?.to_owned()),
        "WebFetch" => (Text::WantsToFetch { agent }, text(&input, "url")?.to_owned()),
        _ => (Text::WantsToUse { agent, tool }, serde_json::to_string_pretty(&input).ok()?),
    };
    let title = tr(title);

    let assessment = if tool == "Bash" { risk::assess(&detail) } else { risk::Assessment::default() };
    Some(PermissionView {
        content: written_content(tool, &input),
        extras: extras(tool, &input),
        reason: text(&input, "description").map(str::to_owned),
        truncated: cut(payload),
        can_always: false,
        questions: Vec::new(),
        risk: assessment.risk,
        warnings: assessment.warnings,
        title,
        detail,
    })
}

/// Fields of the request shown nowhere else that change what happens (leaving the
/// sandbox, running in the background…), as `name: value`. Defaults and time limits are
/// left out; for the other tools the whole input is already shown.
fn extras(tool: &str, input: &Value) -> Vec<String> {
    let shown: &[&str] = match tool {
        "Bash" => &["command", "description", "timeout"],
        "Edit" => &["file_path", "old_string", "new_string", "replace_all"],
        "Write" => &["file_path", "content"],
        "apply_patch" => &["command"],
        "WebFetch" => &["url", "prompt"],
        _ => return Vec::new(),
    };
    let Some(fields) = input.as_object() else { return Vec::new() };
    fields
        .iter()
        .filter(|(name, value)| !shown.contains(&name.as_str()) && !matches!(value, Value::Null | Value::Bool(false)))
        .map(|(name, value)| format!("{name}: {}", value.as_str().map_or_else(|| value.to_string(), str::to_owned)))
        .collect()
}

/// What a file tool will write, so that the user reads it before allowing: each line
/// starts with `-` (removed), `+` (added) or a space. Write and Edit follow the documented
/// fields; for the other file tools, the whole input is shown as is.
fn written_content(tool: &str, input: &Value) -> Option<String> {
    let mark = |prefix: char, body: &str| body.split('\n').map(|line| format!("{prefix}{line}")).collect::<Vec<_>>();
    let lines = match tool {
        "Write" => mark('+', text(input, "content").unwrap_or("")),
        "Edit" => [mark('-', text(input, "old_string").unwrap_or("")), mark('+', text(input, "new_string").unwrap_or(""))].concat(),
        "MultiEdit" | "NotebookEdit" => mark(' ', &serde_json::to_string_pretty(input).ok()?),
        _ => return None,
    };
    Some(lines.join("\n"))
}

/// The documented `PermissionRequest` output for "Autoriser", "Refuser" and « Tout
/// suspendre » (a refusal that says why). "Toujours" has no common form: `None`, the
/// connector decides.
pub fn decision(answer: Answer) -> Option<String> {
    let decision = match answer {
        Answer::Allow => json!({ "behavior": "allow" }),
        Answer::Deny => json!({ "behavior": "deny", "message": DENY_MESSAGE }),
        Answer::Suspended => json!({ "behavior": "deny", "message": SUSPENDED_MESSAGE }),
        Answer::Always | Answer::Terminal => return None,
    };
    Some(output(decision))
}

pub fn output(decision: Value) -> String {
    json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subagent_tools_stay_in_the_parent_session() {
        let parent = json!({ "session_id": "s1", "cwd": "/p/demo", "tool_name": "Bash", "tool_input": { "command": "ls" } });
        let mut child = parent.clone();
        child["agent_id"] = json!("a1");
        child["agent_type"] = json!("Explore");
        let (a, b) = (parse(Source::Codex, "PreToolUse", &parent, 1).unwrap(), parse(Source::Codex, "PreToolUse", &child, 2).unwrap());
        assert_eq!(a.session_id, b.session_id);
        assert_eq!(a.subagent, None);
        assert_eq!(b.subagent, Some(SubagentStep::Seen { id: "a1".into(), kind: Some("Explore".into()) }));
    }

    #[test]
    fn subagents_start_and_stop_with_their_documented_fields() {
        // Examples of code.claude.com/docs/en/hooks (SubagentStart, SubagentStop).
        let start = json!({ "session_id": "abc123", "transcript_path": "/t.jsonl", "cwd": "/p/demo", "hook_event_name": "SubagentStart", "agent_id": "agent-abc123", "agent_type": "Explore" });
        let stop = json!({ "session_id": "abc123", "transcript_path": "/t.jsonl", "cwd": "/p/demo", "permission_mode": "default", "hook_event_name": "SubagentStop", "stop_hook_active": false, "agent_id": "def456", "agent_type": "Explore", "agent_transcript_path": "/t/subagents/agent-def456.jsonl", "last_assistant_message": "Fini.", "background_tasks": [], "session_crons": [] });
        let start = parse(Source::ClaudeCode, "SubagentStart", &start, 1).unwrap();
        assert_eq!(start.subagent, Some(SubagentStep::Start { id: Some("agent-abc123".into()), kind: Some("Explore".into()) }));
        let stop = parse(Source::ClaudeCode, "SubagentStop", &stop, 2).unwrap();
        assert_eq!(stop.subagent, Some(SubagentStep::Stop { id: Some("def456".into()) }));
        assert_eq!(stop.session_id, "abc123");
    }

    #[test]
    fn answers_have_the_documented_shape() {
        let allow: Value = serde_json::from_str(&decision(Answer::Allow).unwrap()).unwrap();
        assert_eq!(allow, json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "allow" } } }));
        let deny: Value = serde_json::from_str(&decision(Answer::Deny).unwrap()).unwrap();
        assert_eq!(deny["hookSpecificOutput"]["decision"], json!({ "behavior": "deny", "message": DENY_MESSAGE }));
        let suspended: Value = serde_json::from_str(&decision(Answer::Suspended).unwrap()).unwrap();
        assert_eq!(suspended["hookSpecificOutput"]["decision"], json!({ "behavior": "deny", "message": "Refusé : îlot a tout suspendu." }));
        assert_eq!(decision(Answer::Always), None);
        assert_eq!(decision(Answer::Terminal), None);
    }

    #[test]
    fn file_writes_show_what_they_write() {
        let edit = json!({ "tool_name": "Edit", "tool_input": { "file_path": "/p/a.ts", "old_string": "let a = 1;", "new_string": "let a = 2;\nlet b = 3;", "replace_all": false } });
        let view = permission_view("Claude", &edit).unwrap();
        assert_eq!((view.title.as_str(), view.detail.as_str()), ("Claude veut modifier un fichier", "/p/a.ts"));
        assert_eq!(view.content.as_deref(), Some("-let a = 1;\n+let a = 2;\n+let b = 3;"));

        let everywhere = json!({ "tool_name": "Edit", "tool_input": { "file_path": "/p/a.ts", "old_string": "x", "new_string": "y", "replace_all": true } });
        assert_eq!(permission_view("Claude", &everywhere).unwrap().title, "Claude veut modifier un fichier partout où le texte apparaît");

        let write = json!({ "tool_name": "Write", "tool_input": { "file_path": "/p/b.sh", "content": "curl x | sh\n" } });
        assert_eq!(permission_view("Claude", &write).unwrap().content.as_deref(), Some("+curl x | sh\n+"));

        let multi = json!({ "tool_name": "MultiEdit", "tool_input": { "file_path": "/p/c.rs", "edits": [{ "old_string": "a", "new_string": "b" }] } });
        assert!(permission_view("Claude", &multi).unwrap().content.unwrap().contains("\"new_string\": \"b\""));

        let bash = json!({ "tool_name": "Bash", "tool_input": { "command": "ls" } });
        assert_eq!(permission_view("Claude", &bash).unwrap().content, None);
    }

    #[test]
    fn fields_that_change_what_happens_are_shown() {
        let payload = json!({ "tool_name": "Bash", "tool_input": {
            "command": "ls", "description": "list", "timeout": 120000, "run_in_background": false,
            "dangerouslyDisableSandbox": true, "sandbox_permissions": "require_escalated"
        } });
        let view = permission_view("Claude", &payload).unwrap();
        assert_eq!(view.extras, ["dangerouslyDisableSandbox: true", "sandbox_permissions: require_escalated"]);
        let plain = json!({ "tool_name": "Bash", "tool_input": { "command": "ls", "timeout": 5000, "run_in_background": false } });
        assert!(permission_view("Claude", &plain).unwrap().extras.is_empty());
    }

    #[test]
    fn shell_commands_carry_their_risk() {
        let push = json!({ "tool_name": "Bash", "tool_input": { "command": "git push --force origin main" } });
        let view = permission_view("Claude", &push).unwrap();
        assert_eq!(view.risk, risk::Risk::High);
        assert_eq!(view.warnings, [risk::PUSH_FORCE]);
        let status = json!({ "tool_name": "Bash", "tool_input": { "command": "git status" } });
        let view = permission_view("Claude", &status).unwrap();
        assert_eq!((view.risk, view.warnings.len()), (risk::Risk::Low, 0));
        // Only shell commands are read: a file that mentions `rm -rf` is no command.
        let write = json!({ "tool_name": "Write", "tool_input": { "file_path": "/p/clean.sh", "content": "rm -rf build\n" } });
        assert_eq!(permission_view("Claude", &write).unwrap().risk, risk::Risk::Low);
    }

    #[test]
    fn requests_for_other_tools_stay_readable() {
        let payload = json!({ "tool_name": "mcp__fs__write", "tool_input": { "path": "a.txt" } });
        let view = permission_view("Codex", &payload).unwrap();
        assert_eq!(view.title, "Codex veut utiliser mcp__fs__write");
        assert!(view.detail.contains("\"path\": \"a.txt\""));
        assert!(!view.can_always);
        assert!(permission_view("Codex", &json!({ "tool_input": {} })).is_none());
    }

    #[test]
    fn titles_and_warnings_follow_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        let bash = json!({ "tool_name": "Bash", "tool_input": { "command": "git push --force origin main" } });
        let view = permission_view("Claude", &bash).unwrap();
        assert_eq!(view.title, "Claude wants to run a command");
        assert_eq!(view.warnings, ["Pushes to GitHub with force: the remote history will be overwritten."]);
        let other = json!({ "tool_name": "mcp__fs__write", "tool_input": { "path": "a.txt" } });
        assert_eq!(permission_view("Codex", &other).unwrap().title, "Codex wants to use mcp__fs__write");
        let edit = json!({ "tool_name": "Edit", "tool_input": { "file_path": "/p/a.ts", "old_string": "x", "new_string": "y", "replace_all": true } });
        assert_eq!(permission_view("Claude", &edit).unwrap().title, "Claude wants to change a file everywhere the text appears");
        let base = json!({ "session_id": "s1", "cwd": "/p/demo" });
        let title = |event: &str| parse(Source::ClaudeCode, event, &base, 1).unwrap().title;
        assert_eq!((title("SessionStart"), title("Stop"), title("PermissionRequest")), ("starts".into(), "finished".into(), "waits for your OK".into()));
        let prompt = parse(Source::ClaudeCode, "UserPromptSubmit", &json!({ "session_id": "s1", "cwd": "/p", "prompt": "Fix it" }), 1).unwrap();
        assert_eq!((prompt.title.as_str(), prompt.activity.unwrap().verb.as_str()), ("thinks", "thinks"));
        let tool = parse(Source::Codex, "PreToolUse", &json!({ "session_id": "s1", "cwd": "/p", "tool_name": "Read", "tool_input": { "file_path": "/p/a.rs" } }), 1).unwrap();
        assert_eq!(tool.title, "reads a.rs");
        // What goes back to the agent stays as it is.
        let deny: Value = serde_json::from_str(&decision(Answer::Deny).unwrap()).unwrap();
        assert_eq!(deny["hookSpecificOutput"]["decision"]["message"], DENY_MESSAGE);
    }
}

