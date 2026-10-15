//! Kiro (CLI V3 and IDE): a hooks file of îlot's own, `~/.kiro/hooks/ilot.json`, activity
//! only. Format from https://kiro.dev/docs/hooks and https://kiro.dev/docs/hooks/types
//! (read on 11 October 2026): `{"version": "v1", "hooks": [{"name", "trigger", "action":
//! {"type": "command", "command"}, "timeout"}]}`, the event as JSON on stdin with
//! `hook_event_name`, `cwd`, `session_id`, `tool_name`, `tool_input`, `tool_response`.
//!
//! - WSL only: the page does not say which shell runs `command` under Windows, and a
//!   command its shell cannot read would fail with a code Kiro reads (1 on `Stop` starts
//!   another turn of the agent). Inside WSL, `command` is a line for `sh`.
//! - Kiro can only block a tool, never let one through: no permission from îlot.
//! - `KIRO_HOME` (another folder than `~/.kiro`) is not followed.

use super::hook_events;
use super::hooks_json::{ShellKind, relay_args, to_pretty};
use super::owned_file::{OwnedFile, Owner};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, Source};
use crate::locale::{Text, tr};
use serde_json::{Value, json};
use std::io;
use std::path::Path;

const SOURCE: &str = "kiro";
const TRIGGERS: [&str; 5] = ["SessionStart", "SessionEnd", "UserPromptSubmit", "PostToolUse", "Stop"];

fn owner(content: &str) -> Owner {
    let Ok(file) = serde_json::from_str::<Value>(content) else { return Owner::Other };
    let Some(hooks) = file.get("hooks").and_then(Value::as_array).filter(|hooks| !hooks.is_empty()) else { return Owner::Other };
    let ours = |hook: &Value| {
        let args = hook.pointer("/action/command").and_then(Value::as_str).and_then(relay_args);
        args.is_some_and(|args| args.first().map(String::as_str) == Some(SOURCE))
    };
    if !hooks.iter().all(ours) {
        return Owner::Other;
    }
    let triggers: Vec<&str> = hooks.iter().filter_map(|hook| text(hook, "trigger")).collect();
    if triggers == TRIGGERS { Owner::Current } else { Owner::Older }
}

const FILE: OwnedFile = OwnedFile { path: ".kiro/hooks/ilot.json", agent_dir: ".kiro", owner };

/// The hooks file, for the relay script inside WSL.
pub fn render(relay: &str) -> io::Result<String> {
    let hooks = TRIGGERS
        .iter()
        .map(|trigger| {
            let command = ShellKind::Posix.command_line(relay, &[SOURCE, trigger])?;
            Ok(json!({ "name": format!("ilot-{trigger}"), "trigger": trigger, "action": { "type": "command", "command": command }, "timeout": 10 }))
        })
        .collect::<io::Result<Vec<_>>>()?;
    to_pretty(&json!({ "version": "v1", "hooks": hooks }), "  ")
}

pub struct Kiro;

impl Connector for Kiro {
    fn id(&self) -> &'static str {
        SOURCE
    }

    fn display_name(&self) -> String {
        "Kiro".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn status(&self, target: &Target) -> Status {
        if target.is_windows() { Status::NotInstalled } else { FILE.status(target) }
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        if target.is_windows() {
            return Err(io::Error::other(tr(Text::KiroInWsl)));
        }
        let relay = target.install_relay(relay_exe)?;
        FILE.write(target, &render(&relay)?)
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        FILE.remove(target)?;
        tidy_relay(target)
    }

    /// Kiro's events read like Claude Code's. One without `session_id` (the page shows it
    /// for `agentSpawn` only) stays in a session of its folder.
    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        if text(payload, "session_id").is_some() {
            return hook_events::parse(Source::Kiro, event, payload, at);
        }
        let mut named = payload.clone();
        named["session_id"] = json!(format!("kiro@{}", text(payload, "cwd")?));
        hook_events::parse(Source::Kiro, event, &named, at)
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
    use crate::events::EventKind;

    /// `SessionStart` is the documented `agentSpawn` example and `PreToolUse` the
    /// documented MCP example (https://kiro.dev/docs/hooks/types); `PostToolUse` and `Stop`
    /// add the fields the page names. In tests/fixtures/kiro/.
    fn fixture(event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/kiro/{event}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn documented_events_parse() {
        for event in ["SessionStart", "PreToolUse", "PostToolUse", "Stop"] {
            let parsed = Kiro.parse(event, &fixture(event), 1).unwrap();
            assert_eq!((parsed.source, parsed.session_id.as_str(), parsed.project.as_str()), (Source::Kiro, "abc123-def456-789", "demo"));
        }
        assert_eq!(Kiro.parse("PostToolUse", &fixture("PostToolUse"), 1).unwrap().title, "utilise postgres_query");
        assert_eq!(Kiro.parse("Stop", &fixture("Stop"), 1).unwrap().kind, EventKind::TurnDone);
        let quiet = json!({ "hook_event_name": "UserPromptSubmit", "cwd": "/p/demo" });
        assert_eq!(Kiro.parse("UserPromptSubmit", &quiet, 1).unwrap().session_id, "kiro@/p/demo");
    }

    #[test]
    fn a_file_write_reads_as_an_edit() {
        // Shape of the `fs_write` tool Kiro CLI inherits (not on Kiro's pages): to check
        // on a real session.
        let payload = json!({ "session_id": "k", "cwd": "/p/demo", "tool_name": "fs_write",
            "tool_input": { "command": "str_replace", "path": "/p/demo/api.py", "old_str": "a", "new_str": "b\nc" } });
        let event = Kiro.parse("PostToolUse", &payload, 1).unwrap();
        assert_eq!(event.title, "édite api.py");
        assert_eq!(event.activity.map(|a| (a.added, a.removed)), Some((2, 1)));
        assert!(event.diff.is_some());
    }

    #[test]
    fn the_hooks_file_calls_the_relay_for_each_trigger() {
        let file: Value = serde_json::from_str(&render("/home/user/.local/bin/ilot-hook").unwrap()).unwrap();
        assert_eq!(file["version"], "v1");
        assert_eq!(
            file["hooks"][4],
            json!({ "name": "ilot-Stop", "trigger": "Stop", "action": { "type": "command", "command": "'/home/user/.local/bin/ilot-hook' kiro Stop" }, "timeout": 10 })
        );
        assert_eq!(owner(&render("/home/user/.local/bin/ilot-hook").unwrap()), Owner::Current);
        assert_eq!(owner("{\"version\":\"v1\",\"hooks\":[{\"name\":\"lint\",\"trigger\":\"PostFileSave\",\"action\":{\"type\":\"command\",\"command\":\"npx eslint --fix\"}}]}"), Owner::Other);
    }

    /// Real WSL file operations on a throwaway home in `/tmp`, never the user's own.
    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn connect_and_disconnect_inside_wsl_on_a_throwaway_home() {
        let distro = crate::wsl::distros().into_iter().next().expect("a WSL distribution");
        let home = format!("/tmp/ilot-test-kiro-{}", std::process::id());
        let hooks = crate::wsl::unc_path(&distro, &format!("{home}/.kiro/hooks"));
        std::fs::create_dir_all(&hooks).unwrap();
        std::fs::write(hooks.join("lint.json"), "{\"version\":\"v1\",\"hooks\":[]}").unwrap();
        let target = Target::wsl(&distro, &home);
        assert_eq!(Kiro.status(&target), Status::Installed);
        Kiro.connect(&target, &std::env::current_exe().unwrap().with_file_name("ilot-hook.exe")).unwrap();
        assert_eq!(Kiro.status(&target), Status::Connected);
        Kiro.disconnect(&target).unwrap();
        assert!(!hooks.join("ilot.json").exists());
        assert_eq!(std::fs::read_to_string(hooks.join("lint.json")).unwrap(), "{\"version\":\"v1\",\"hooks\":[]}");
        assert!(!crate::wsl::unc_path(&distro, &format!("{home}/.local/bin/ilot-hook")).exists());
        std::fs::remove_dir_all(crate::wsl::unc_path(&distro, &home)).unwrap();
    }

    #[test]
    fn kiro_is_wsl_only_and_followed_only() {
        let windows = Target::windows(std::env::temp_dir());
        assert_eq!(Kiro.status(&windows), Status::NotInstalled);
        assert!(Kiro.connect(&windows, Path::new(r"C:\ilot\ilot-hook.exe")).is_err());
        assert!(!Kiro.capabilities().permissions);
    }

    #[test]
    fn kiro_says_it_is_wsl_only_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let error = Kiro.connect(&Target::windows(std::env::temp_dir()), Path::new(r"C:\ilot\ilot-hook.exe")).unwrap_err();
        assert_eq!(error.to_string(), "Kiro connects in WSL only for now.");
    }
}
