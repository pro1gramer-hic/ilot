//! OpenCode: a plugin of îlot's own, `~/.config/opencode/plugins/ilot.ts`
//! (`src-tauri/plugins/opencode.ts`), activity only. OpenCode has no command hooks; its
//! plugins run inside it and get its events (https://opencode.ai/docs/plugins; the types of
//! v1.19.0, https://github.com/anomalyco/opencode/blob/v1.19.0/packages/plugin/src/index.ts).
//!
//! - The plugin starts the relay with an array of arguments (no shell) and the event as
//!   JSON on stdin, under Claude Code's names (`SessionStart`, `PostToolUse`…), which
//!   `hook_events` reads.
//! - A subagent is a session with a `parentID`: its events go to its parent's session,
//!   with its own id as `agent_id`.
//! - No permission from îlot: OpenCode could take an answer from a plugin, but what happens
//!   when its terminal answers too is not documented. A request is only shown, as waiting
//!   in the terminal (`permission.updated`).
//! - OpenCode 2 changes the plugin API: the plugin targets v1 and will need a new revision.

use super::activity::first_line;
use super::hook_events;
use super::owned_file::{OwnedFile, Owner, plugin_owner};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, EventKind, Source};
use crate::locale::{Text, tr};
use serde_json::Value;
use std::io;
use std::path::Path;

const PLUGIN: &str = include_str!("../../plugins/opencode.ts");
const REVISION: &str = "// ilot-plugin-revision: 1";
/// Where the plugin names the relay; îlot writes the path there as a JSON string.
pub const RELAY_SLOT: &str = "\"__ILOT_RELAY__\"";

fn owner(text: &str) -> Owner {
    plugin_owner(text, REVISION)
}

const FILE: OwnedFile = OwnedFile { path: ".config/opencode/plugins/ilot.ts", agent_dir: ".config/opencode", owner };

/// A plugin's source with the relay's path in it, as a JavaScript string.
pub fn with_relay(source: &str, relay: &str) -> io::Result<String> {
    let literal = serde_json::to_string(relay).map_err(io::Error::other)?;
    Ok(source.replacen(RELAY_SLOT, &literal, 1))
}

pub struct OpenCode;

impl Connector for OpenCode {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> String {
        "OpenCode".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn status(&self, target: &Target) -> Status {
        FILE.status(target)
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        FILE.write(target, &with_relay(PLUGIN, &relay)?)
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        FILE.remove(target)?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        if event == "Notification" && text(payload, "notification_type") == Some("permission_prompt") {
            let base = AgentEvent::new(
                Source::OpenCode,
                text(payload, "session_id")?,
                text(payload, "cwd").unwrap_or_default(),
                EventKind::Asking,
                tr(Text::AgentWaitsForOkInTerminal),
                at,
            );
            return Some(AgentEvent { detail: text(payload, "message").filter(|m| !m.is_empty()).map(first_line), ..base });
        }
        hook_events::parse(Source::OpenCode, event, payload, at)
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
    use crate::events::SubagentStep;
    use crate::sessions::Sessions;

    /// What the plugin sends for documented OpenCode events (tests/fixtures/opencode/,
    /// checked against the plugin itself by tests/unit/agent-plugins.test.ts).
    fn sent() -> Vec<(String, Value)> {
        let path = format!("{}/../tests/fixtures/opencode/events.json", env!("CARGO_MANIFEST_DIR"));
        let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        file["calls"].as_array().unwrap().iter().map(|c| (c["sent"][0].as_str().unwrap().to_owned(), c["sent"][1].clone())).collect()
    }

    #[test]
    fn a_session_with_a_subagent_reads_as_one_session() {
        let mut sessions = Sessions::default();
        let events: Vec<AgentEvent> = sent().iter().map(|(event, payload)| OpenCode.parse(event, payload, 1).unwrap()).collect();
        assert!(events.iter().all(|e| e.source == Source::OpenCode && e.session_id == "ses_01JZ8Q4V3K2M" && e.project == "demo"));
        assert_eq!(events[1].detail.as_deref(), Some("Add a health check route"));
        assert_eq!(events[2].title, "lance npm test");
        assert_eq!(events[3].title, "édite server.ts");
        assert_eq!(events[3].activity.as_ref().map(|a| (a.added, a.removed)), Some((1, 0)), "one line added above an unchanged one");
        assert!(events[3].diff.is_some());
        assert_eq!(events[4].subagent, Some(SubagentStep::Start { id: Some("ses_01JZ8Q9X7T".into()), kind: Some("Find the routes (@explore subagent)".into()) }));
        assert_eq!(events[5].subagent, Some(SubagentStep::Seen { id: "ses_01JZ8Q9X7T".into(), kind: None }));
        assert_eq!(events[6].subagent, Some(SubagentStep::Stop { id: Some("ses_01JZ8Q9X7T".into()) }));
        assert_eq!((events[7].kind, events[7].detail.as_deref()), (EventKind::Asking, Some("rm -rf dist")));
        assert_eq!(events[8].kind, EventKind::TurnDone);
        for event in &events[..5] {
            sessions.apply(event);
        }
        assert_eq!(sessions.list().len(), 1);
        assert_eq!(sessions.list()[0].subagents.len(), 1);
    }

    #[test]
    fn the_plugin_names_the_relay_as_a_string() {
        let plugin = with_relay(PLUGIN, "/home/o'neil/.local/bin/ilot-hook").unwrap();
        assert!(plugin.contains("const RELAY = \"/home/o'neil/.local/bin/ilot-hook\";"));
        let windows = with_relay(PLUGIN, r#"C:\Users\A "B"\ilot-hook.exe"#).unwrap();
        assert!(windows.contains(r#"const RELAY = "C:\\Users\\A \"B\"\\ilot-hook.exe";"#), "escaped, never code");
        assert_eq!(owner(&plugin), Owner::Current);
        assert!(!plugin.contains("__ILOT_RELAY__"));
        assert_eq!(owner("export const Mine = async () => ({});"), Owner::Other);
        assert!(!OpenCode.capabilities().permissions);
    }

    #[test]
    fn connect_then_disconnect_leaves_the_plugins_folder_as_it_was() {
        let dir = std::env::temp_dir().join(format!("ilot-opencode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let plugins = dir.join(".config").join("opencode").join("plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(plugins.join("mine.ts"), "export const Mine = async () => ({});\n").unwrap();
        let target = Target::windows(dir.clone());
        assert_eq!(OpenCode.status(&target), Status::Installed);
        OpenCode.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(OpenCode.status(&target), Status::Connected);
        OpenCode.disconnect(&target).unwrap();
        assert!(!plugins.join("ilot.ts").exists());
        assert_eq!(std::fs::read_to_string(plugins.join("mine.ts")).unwrap(), "export const Mine = async () => ({});\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_approval_is_announced_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let payload = serde_json::json!({ "session_id": "s1", "cwd": "/p", "notification_type": "permission_prompt", "message": "Run ls?" });
        assert_eq!(OpenCode.parse("Notification", &payload, 1).unwrap().title, "waits for your OK in its terminal");
    }
}
