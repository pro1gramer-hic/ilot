//! Codex: hooks in `~/.codex/hooks.json`, activity and permissions. Formats from
//! https://developers.openai.com/codex/hooks (Codex 0.160), checked against real captures
//! in `tests/fixtures/codex/`. Same events, fields and answers as Claude Code
//! (`hook_events`), with these differences:
//! - `command` is one command line, run by `$SHELL -lc` (WSL) or `cmd.exe /C` (Windows);
//! - Codex runs a new or changed hook only once the user trusted it in `/hooks`;
//! - no "Toujours": Codex refuses `updatedPermissions` for now;
//! - `Interrupt` ends a turn the user stopped (Codex sends no `Stop` then).
//!
//! The older `notify` program (`ilot-hook codex notify '<json>'`) is still understood but
//! never installed: `Stop` says the same thing, and a second notifier would announce
//! every turn twice and take the single `notify` slot of `config.toml`.

use super::hook_events;
use super::hooks_json::{self, Call, HandlerFormat, HookSpec, ShellKind};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, EventKind, Source};
use crate::locale::{Text, tr};
use serde_json::Value;
use std::io;
use std::path::Path;

const HOOKS: &str = ".codex/hooks.json";
/// Observation runs in the background (Codex never waits for îlot). `SessionEnd` always
/// runs in the foreground, 3 s at most, like `Interrupt`. The permission waits 120 s at
/// most, after which Codex asks in its terminal.
const SPECS: [HookSpec<'static>; 10] = [
    HookSpec::new("SessionStart", 10, true),
    HookSpec::new("SessionEnd", 3, false),
    HookSpec::new("UserPromptSubmit", 10, true),
    HookSpec::new("PreToolUse", 10, true),
    HookSpec::new("PostToolUse", 10, true),
    HookSpec::new("SubagentStart", 10, true),
    HookSpec::new("SubagentStop", 10, true),
    HookSpec::new("Stop", 10, true),
    HookSpec::new("Interrupt", 3, true),
    HookSpec::new("PermissionRequest", 120, false),
];

fn format(target: &Target) -> HandlerFormat {
    let shell = if target.is_windows() { ShellKind::Cmd } else { ShellKind::Posix };
    HandlerFormat { call: Call::Shell(shell), timeout_in_ms: false, background_key: Some("async"), name: None, flat: false, version: None }
}

pub struct Codex;

impl Connector for Codex {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> String {
        "Codex".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: true }
    }

    fn status(&self, target: &Target) -> Status {
        hooks_json::status(&target.home_file(HOOKS), &SPECS)
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        hooks_json::plug(&target.home_file(HOOKS), &relay, self.id(), &SPECS, &format(target))
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        hooks_json::unplug(&target.home_file(HOOKS))?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        match event {
            // `notify` names the session `thread-id`: the same id as the hooks' `session_id`.
            "notify" => {
                if text(payload, "type") != Some("agent-turn-complete") {
                    return None;
                }
                let cwd = text(payload, "cwd").unwrap_or_default();
                Some(AgentEvent::new(Source::Codex, text(payload, "thread-id")?, cwd, EventKind::TurnDone, tr(Text::AgentFinished), at))
            }
            "Interrupt" => {
                let cwd = text(payload, "cwd").unwrap_or_default();
                Some(AgentEvent::new(Source::Codex, text(payload, "session_id")?, cwd, EventKind::Interrupted, tr(Text::AgentInterrupted), at))
            }
            _ => hook_events::parse(Source::Codex, event, payload, at),
        }
    }

    fn permission(&self, event: &str, payload: &Value) -> Option<PermissionView> {
        if event != "PermissionRequest" {
            return None;
        }
        hook_events::permission_view(&self.display_name(), payload)
    }

    fn answer(&self, event: &str, _payload: &Value, answer: Answer) -> Option<String> {
        if event != "PermissionRequest" {
            return None;
        }
        hook_events::decision(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::hook_events::DENY_MESSAGE;
    use serde_json::json;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/../tests/fixtures/codex/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn parse(event: &str) -> AgentEvent {
        Codex.parse(event, &fixture(event), 1).unwrap()
    }

    /// The real `notify` call: the JSON is the relay's last argument.
    fn notify_payload() -> Value {
        let capture = fixture("agent-turn-complete");
        let argv = capture["argv"].as_array().unwrap();
        assert_eq!(argv[..2], [json!("codex"), json!("notify")]);
        serde_json::from_str(argv[2].as_str().unwrap()).unwrap()
    }

    #[test]
    fn every_real_capture_parses_into_one_session() {
        let events = ["SessionStart", "UserPromptSubmit", "PreToolUse", "PermissionRequest", "PostToolUse", "Stop", "SessionEnd"];
        for event in events {
            let parsed = parse(event);
            assert_eq!(parsed.source, Source::Codex, "{event}");
            assert_eq!(parsed.project, "demo", "{event}");
            assert_eq!(parsed.session_id, "01a128b7-b945-7ab0-9d7e-722cfaee9a0c", "{event}");
        }
        let notify = Codex.parse("notify", &notify_payload(), 1).unwrap();
        assert_eq!(notify.session_id, "01a128b7-b945-7ab0-9d7e-722cfaee9a0c");
        assert_eq!(notify.kind, EventKind::TurnDone);
        assert_eq!(notify.project, "demo");
    }

    #[test]
    fn events_map_to_the_common_kinds() {
        assert_eq!(parse("SessionStart").kind, EventKind::SessionStart);
        assert_eq!(parse("UserPromptSubmit").kind, EventKind::Working);
        assert_eq!(parse("UserPromptSubmit").detail.as_deref(), Some("Run the test steps."));
        assert_eq!(parse("PreToolUse").title, "lance cat notes.txt");
        assert_eq!(parse("PermissionRequest").kind, EventKind::Permission);
        assert_eq!(parse("Stop").kind, EventKind::TurnDone);
        assert_eq!(parse("SessionEnd").kind, EventKind::SessionEnd);
        let interrupt = json!({ "session_id": "s", "cwd": "/p/demo", "hook_event_name": "Interrupt", "turn_id": "t" });
        assert_eq!(Codex.parse("Interrupt", &interrupt, 1).unwrap().kind, EventKind::Interrupted);
    }

    #[test]
    fn other_notifications_and_unknown_events_are_ignored() {
        let mut payload = notify_payload();
        payload["type"] = json!("approval-requested");
        assert!(Codex.parse("notify", &payload, 1).is_none());
        assert!(Codex.parse("PreCompact", &fixture("Stop"), 1).is_none());
    }

    #[test]
    fn real_permission_request_shows_the_command_and_its_reason() {
        let view = Codex.permission("PermissionRequest", &fixture("PermissionRequest")).unwrap();
        assert_eq!(view.title, "Codex veut exécuter une commande");
        assert_eq!(view.detail, "touch /home/user/projects/allowed-by-ilot.txt");
        assert_eq!(view.reason.as_deref(), Some("Créer un fichier de test hors du dossier de travail ?"));
        assert!(!view.can_always, "Codex refuses updatedPermissions");
        assert!(Codex.permission("PreToolUse", &fixture("PreToolUse")).is_none());
    }

    #[test]
    fn answers_use_the_documented_format_without_toujours() {
        let payload = fixture("PermissionRequest");
        let allow: Value = serde_json::from_str(&Codex.answer("PermissionRequest", &payload, Answer::Allow).unwrap()).unwrap();
        assert_eq!(allow, json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "allow" } } }));
        let deny: Value = serde_json::from_str(&Codex.answer("PermissionRequest", &payload, Answer::Deny).unwrap()).unwrap();
        assert_eq!(
            deny,
            json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "deny", "message": DENY_MESSAGE } } })
        );
        assert_eq!(Codex.answer("PermissionRequest", &payload, Answer::Always), None);
        assert_eq!(Codex.answer("Stop", &payload, Answer::Allow), None);
        // « Tout suspendre »: a plain refusal, Codex refuses `interrupt` for now.
        let suspended: Value = serde_json::from_str(&Codex.answer("PermissionRequest", &payload, Answer::Suspended).unwrap()).unwrap();
        assert_eq!(suspended["hookSpecificOutput"]["decision"], json!({ "behavior": "deny", "message": "Refusé : îlot a tout suspendu." }));
    }

    #[test]
    fn a_question_of_codex_is_only_shown() {
        // The real PreToolUse capture, with Codex's question tool (`request_user_input`,
        // payload read in codex-rs/protocol/src/request_user_input.rs): no hook answer is
        // documented, so the session only waits in its terminal.
        let mut payload = fixture("PreToolUse");
        payload["tool_name"] = json!("request_user_input");
        payload["tool_input"] = json!({ "questions": [{ "id": "q1", "header": "Base", "question": "Quelle base garder ?", "options": [{ "label": "SQLite" }, { "label": "Postgres" }] }] });
        let event = Codex.parse("PreToolUse", &payload, 1).unwrap();
        assert_eq!((event.kind, event.detail.as_deref()), (EventKind::Asking, Some("Quelle base garder ?")));
        assert!(Codex.permission("PreToolUse", &payload).is_none());
        assert!(!ilot_bridge::protocol::waits_for_answer("codex", "PreToolUse"));
    }

    fn temp_home(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ilot-codex-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".codex")).unwrap();
        dir
    }

    #[test]
    fn a_hooks_file_created_for_ilot_leaves_with_it() {
        let dir = temp_home("created");
        let target = Target::windows(dir.clone());
        let hooks = dir.join(".codex").join("hooks.json");
        assert_eq!(Codex.status(&target), Status::Installed);
        Codex.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(Codex.status(&target), Status::Connected);
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&hooks).unwrap()).unwrap();
        let relay = dir.join("ilot-hook.exe").display().to_string();
        assert_eq!(
            written["hooks"]["PermissionRequest"][0]["hooks"][0],
            json!({ "type": "command", "command": format!("\"{relay}\" codex PermissionRequest"), "timeout": 120 })
        );
        assert_eq!(
            written["hooks"]["Stop"][0]["hooks"][0],
            json!({ "type": "command", "command": format!("\"{relay}\" codex Stop"), "timeout": 10, "async": true })
        );
        assert_eq!(written["hooks"]["SessionEnd"][0]["hooks"][0]["timeout"], 3);
        assert!(written["hooks"]["SessionEnd"][0]["hooks"][0].get("async").is_none());
        assert!(!hooks_json::backup_path(&hooks).exists(), "nothing to back up");
        Codex.disconnect(&target).unwrap();
        assert!(!hooks.exists(), "a file îlot created leaves with it");
        assert_eq!(Codex.status(&target), Status::Installed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn connect_then_disconnect_restores_a_copy_of_a_real_hooks_file() {
        let dir = temp_home("restore");
        let original = "{\n  \"description\": \"Mes hooks\",\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Bash\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"python3 ~/.codex/hooks/policy.py\",\n            \"statusMessage\": \"Checking Bash command\"\n          }\n        ]\n      }\n    ]\n  }\n}\n";
        let hooks = dir.join(".codex").join("hooks.json");
        std::fs::write(&hooks, original).unwrap();
        let target = Target::windows(dir.clone());
        Codex.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(std::fs::read_to_string(hooks_json::backup_path(&hooks)).unwrap(), original);
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&hooks).unwrap()).unwrap();
        assert_eq!(written["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        Codex.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&hooks).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_codex_folder_means_not_installed() {
        let target = Target::windows(std::env::temp_dir().join("ilot-no-codex-here"));
        assert_eq!(Codex.status(&target), Status::NotInstalled);
    }

    /// Real WSL file operations on a throwaway home in `/tmp`: the command line Codex runs
    /// with `$SHELL -lc`, the relay script, and the file given back as it was.
    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn connect_and_disconnect_inside_wsl_on_a_throwaway_home() {
        let distro = crate::wsl::distros().into_iter().next().expect("a WSL distribution");
        let home = format!("/tmp/ilot-test-codex-{}", std::process::id());
        let hooks = crate::wsl::unc_path(&distro, &format!("{home}/.codex/hooks.json"));
        std::fs::create_dir_all(hooks.parent().unwrap()).unwrap();
        let target = Target::wsl(&distro, &home);
        let relay = std::env::current_exe().unwrap().with_file_name("ilot-hook.exe");
        Codex.connect(&target, &relay).unwrap();
        assert_eq!(Codex.status(&target), Status::Connected);
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&hooks).unwrap()).unwrap();
        assert_eq!(written["hooks"]["Stop"][0]["hooks"][0]["command"], format!("'{home}/.local/bin/ilot-hook' codex Stop"));
        Codex.disconnect(&target).unwrap();
        assert!(!hooks.exists());
        assert!(!crate::wsl::unc_path(&distro, &format!("{home}/.local/bin/ilot-hook")).exists());
        std::fs::remove_dir_all(crate::wsl::unc_path(&distro, &home)).unwrap();
    }

    #[test]
    fn codex_s_titles_follow_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(Codex.parse("notify", &notify_payload(), 1).unwrap().title, "finished");
        let interrupt = json!({ "session_id": "s1", "cwd": "/p" });
        assert_eq!(Codex.parse("Interrupt", &interrupt, 1).unwrap().title, "interrupted");
    }
}
