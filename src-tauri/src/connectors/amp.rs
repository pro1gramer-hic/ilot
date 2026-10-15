//! Amp: a plugin of îlot's own, `~/.config/amp/plugins/ilot.ts` (`src-tauri/plugins/amp.ts`),
//! inside WSL, activity only. Amp has no command hooks; its plugins get its events
//! (https://ampcode.com/docs/customize/plugins, https://ampcode.com/docs/plugin-api).
//!
//! - WSL only: Amp's CLI runs under Windows through WSL (https://ampcode.com/docs/cli).
//!   `$XDG_CONFIG_HOME` (another folder than `~/.config`) is not followed.
//! - The plugin listens to `session.start`, `agent.start`, `tool.result` and `agent.end`,
//!   never to `tool.call`, where a plugin decides whether a tool runs. Amp asks for no
//!   approval before its tools (https://ampcode.com/docs/tools): îlot has nothing to answer.
//! - No `session.end` exists: a session leaves the list after two hours of silence.

use super::hook_events;
use super::opencode::with_relay;
use super::owned_file::{OwnedFile, Owner, plugin_owner};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, EventKind, Source};
use crate::locale::{Text, tr};
use serde_json::Value;
use std::io;
use std::path::Path;

const PLUGIN: &str = include_str!("../../plugins/amp.ts");
const REVISION: &str = "// ilot-plugin-revision: 1";

fn owner(text: &str) -> Owner {
    plugin_owner(text, REVISION)
}

const FILE: OwnedFile = OwnedFile { path: ".config/amp/plugins/ilot.ts", agent_dir: ".config/amp", owner };

pub struct Amp;

impl Connector for Amp {
    fn id(&self) -> &'static str {
        "amp"
    }

    fn display_name(&self) -> String {
        "Amp".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn status(&self, target: &Target) -> Status {
        if target.is_windows() { Status::NotInstalled } else { FILE.status(target) }
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        if target.is_windows() {
            return Err(io::Error::other(tr(Text::AmpInWsl)));
        }
        let relay = target.install_relay(relay_exe)?;
        FILE.write(target, &with_relay(PLUGIN, &relay)?)
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        FILE.remove(target)?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        if event == "Interrupt" {
            let cwd = text(payload, "cwd").unwrap_or_default();
            return Some(AgentEvent::new(Source::Amp, text(payload, "session_id")?, cwd, EventKind::Interrupted, tr(Text::AgentInterrupted), at));
        }
        hook_events::parse(Source::Amp, event, payload, at)
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
    use serde_json::json;

    /// What the plugin sends for documented Amp events (tests/fixtures/amp/, checked
    /// against the plugin itself by tests/unit/agent-plugins.test.ts).
    fn sent() -> Vec<(String, Value)> {
        let path = format!("{}/../tests/fixtures/amp/events.json", env!("CARGO_MANIFEST_DIR"));
        let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        file["calls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| !c["sent"].is_null())
            .map(|c| (c["sent"][0].as_str().unwrap().to_owned(), c["sent"][1].clone()))
            .collect()
    }

    #[test]
    fn a_thread_reads_as_one_session() {
        let events: Vec<AgentEvent> = sent().iter().map(|(event, payload)| Amp.parse(event, payload, 1).unwrap()).collect();
        assert!(events.iter().all(|e| e.source == Source::Amp && e.session_id == "T-7f3c2a10-5b2e-4c8f-9d1a-2e6b8c0f4a31" && e.project == "demo"));
        let kinds: Vec<EventKind> = events.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, [EventKind::SessionStart, EventKind::Working, EventKind::Tool, EventKind::TurnDone, EventKind::Interrupted]);
        assert_eq!(events[1].detail.as_deref(), Some("Explain the billing module"));
        assert_eq!(events[2].title, "utilise oracle");
    }

    #[test]
    fn amp_s_file_tools_read_as_edits() {
        // Tool names of `amp tools list` (not on Amp's pages): to check on a real session.
        let edit = json!({ "session_id": "T-1", "cwd": "/p/demo", "tool_name": "edit_file", "tool_input": { "path": "/p/demo/a.rs", "old_str": "a", "new_str": "b" } });
        let event = Amp.parse("PostToolUse", &edit, 1).unwrap();
        assert_eq!(event.title, "édite a.rs");
        assert!(event.diff.is_some());
    }

    /// Real WSL file operations on a throwaway home in `/tmp`, never the user's own.
    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn connect_and_disconnect_inside_wsl_on_a_throwaway_home() {
        let distro = crate::wsl::distros().into_iter().next().expect("a WSL distribution");
        let home = format!("/tmp/ilot-test-amp-{}", std::process::id());
        let plugins = crate::wsl::unc_path(&distro, &format!("{home}/.config/amp/plugins"));
        std::fs::create_dir_all(&plugins).unwrap();
        let target = Target::wsl(&distro, &home);
        assert_eq!(Amp.status(&target), Status::Installed);
        Amp.connect(&target, &std::env::current_exe().unwrap().with_file_name("ilot-hook.exe")).unwrap();
        assert_eq!(Amp.status(&target), Status::Connected);
        let plugin = std::fs::read_to_string(plugins.join("ilot.ts")).unwrap();
        assert!(plugin.contains(&format!("const RELAY = \"{home}/.local/bin/ilot-hook\";")));
        Amp.disconnect(&target).unwrap();
        assert!(!plugins.join("ilot.ts").exists());
        std::fs::remove_dir_all(crate::wsl::unc_path(&distro, &home)).unwrap();
    }

    #[test]
    fn amp_is_wsl_only_and_followed_only() {
        let windows = Target::windows(std::env::temp_dir());
        assert_eq!(Amp.status(&windows), Status::NotInstalled);
        assert!(Amp.connect(&windows, Path::new(r"C:\ilot\ilot-hook.exe")).is_err());
        assert!(!Amp.capabilities().permissions);
        let plugin = with_relay(PLUGIN, "/home/user/.local/bin/ilot-hook").unwrap();
        assert_eq!(owner(&plugin), Owner::Current);
        assert!(!plugin.contains("\"tool.call\""), "never where a tool is allowed or refused");
    }

    #[test]
    fn amp_speaks_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let error = Amp.connect(&Target::windows(std::env::temp_dir()), Path::new(r"C:\ilot\ilot-hook.exe")).unwrap_err();
        assert_eq!(error.to_string(), "Amp runs in WSL: connect it in your distribution.");
        assert_eq!(Amp.parse("Interrupt", &json!({ "session_id": "s1", "cwd": "/p" }), 1).unwrap().title, "interrupted");
    }
}
