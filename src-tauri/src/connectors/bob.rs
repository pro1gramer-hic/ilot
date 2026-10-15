//! IBM Bob (Bob Shell and Bob IDE): hooks in `~/.bob/settings/settings.json`, activity only.
//! Format from https://bob.ibm.com/docs/shell/configuration/lifecycle-hooks and
//! https://bob.ibm.com/docs/ide/configuration/lifecycle-hooks (read on 11 October 2026).
//!
//! - Same shape as Claude Code's hooks (event → groups with a `matcher` → handlers);
//!   `command` is one line run by `sh -c` (WSL) or `cmd /c` (Windows), `timeout` in
//!   seconds.
//! - The payload names its fields `event`, `session_id`, `prompt`, `tool`, `input` and
//!   `output`, and has no `cwd`: Bob runs its hooks in the task's folder, which the relay
//!   sends along (`RelayEnv::cwd`).
//! - Bob can only block (exit code 2), never let anything through: no permission from îlot.
//!   The output of `SessionStart` and `UserPromptSubmit` goes into the model's context:
//!   the relay prints nothing.

use super::hook_events;
use super::hooks_json::{self, Call, HandlerFormat, HookSpec, ShellKind};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, Source};
use serde_json::{Value, json};
use std::io;
use std::path::Path;

const SETTINGS: &str = ".bob/settings/settings.json";
const SPECS: [HookSpec<'static>; 4] = [
    HookSpec::new("SessionStart", 10, false),
    HookSpec::new("UserPromptSubmit", 10, false),
    HookSpec { matcher: Some(".*"), ..HookSpec::new("PostToolUse", 10, false) },
    HookSpec::new("Stop", 10, false),
];

fn format(target: &Target) -> HandlerFormat {
    let shell = if target.is_windows() { ShellKind::Cmd } else { ShellKind::Posix };
    HandlerFormat { call: Call::Shell(shell), timeout_in_ms: false, background_key: None, name: None, flat: false, version: None }
}

pub struct Bob;

impl Connector for Bob {
    fn id(&self) -> &'static str {
        "bob"
    }

    fn display_name(&self) -> String {
        "IBM Bob".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    /// Installed once `~/.bob` exists, even before Bob wrote its settings folder.
    fn status(&self, target: &Target) -> Status {
        if !target.home_file(".bob").is_dir() {
            return Status::NotInstalled;
        }
        match hooks_json::status(&target.home_file(SETTINGS), &SPECS) {
            Status::NotInstalled => Status::Installed,
            status => status,
        }
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        hooks_json::plug(&target.home_file(SETTINGS), &relay, self.id(), &SPECS, &format(target))
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        hooks_json::unplug(&target.home_file(SETTINGS))?;
        tidy_relay(target)
    }

    /// Bob's fields under the names `hook_events` reads.
    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        let mut shaped = json!({ "session_id": text(payload, "session_id")? });
        for (theirs, ours) in [("cwd", "cwd"), ("prompt", "prompt"), ("tool", "tool_name"), ("input", "tool_input"), (ilot_bridge::protocol::TRUNCATED_FLAG, ilot_bridge::protocol::TRUNCATED_FLAG)] {
            if let Some(value) = payload.get(theirs) {
                shaped[ours] = value.clone();
            }
        }
        hook_events::parse(Source::Bob, event, &shaped, at)
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

    /// Payloads with the fields the lifecycle-hooks page lists for each event, in
    /// tests/fixtures/bob/. Bob sends no `cwd`: the core adds the relay's own folder.
    fn fixture(event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/bob/{event}.json", env!("CARGO_MANIFEST_DIR"));
        let mut payload: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        payload["cwd"] = json!("/home/user/projects/demo");
        payload
    }

    #[test]
    fn documented_events_parse() {
        for event in ["SessionStart", "UserPromptSubmit", "PostToolUse", "Stop"] {
            let parsed = Bob.parse(event, &fixture(event), 1).unwrap();
            assert_eq!((parsed.source, parsed.session_id.as_str(), parsed.project.as_str()), (Source::Bob, "b0b5e551-0n-7e57", "demo"));
        }
        assert_eq!(Bob.parse("UserPromptSubmit", &fixture("UserPromptSubmit"), 1).unwrap().detail.as_deref(), Some("Refactor the billing module"));
        let write = Bob.parse("PostToolUse", &fixture("PostToolUse"), 1).unwrap();
        assert_eq!(write.title, "écrit billing.py");
        assert_eq!(write.activity.map(|a| a.added), Some(2));
        assert_eq!(write.diff.map(|d| d.file), Some("billing.py".into()));
        assert_eq!(Bob.parse("Stop", &fixture("Stop"), 1).unwrap().kind, EventKind::TurnDone);
    }

    #[test]
    fn connect_then_disconnect_restores_bob_s_settings() {
        let dir = std::env::temp_dir().join(format!("ilot-bob-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let target = Target::windows(dir.clone());
        assert_eq!(Bob.status(&target), Status::NotInstalled);
        std::fs::create_dir_all(dir.join(".bob")).unwrap();
        assert_eq!(Bob.status(&target), Status::Installed, "no settings folder yet");
        std::fs::create_dir_all(dir.join(".bob").join("settings")).unwrap();
        let settings = dir.join(".bob").join("settings").join("settings.json");
        let original = "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"^write_file$\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"./check.sh\",\n            \"timeout\": 5\n          }\n        ]\n      }\n    ]\n  }\n}\n";
        std::fs::write(&settings, original).unwrap();
        Bob.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(Bob.status(&target), Status::Connected);
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let relay = dir.join("ilot-hook.exe").display().to_string();
        assert_eq!(written["hooks"]["PostToolUse"][0]["matcher"], ".*");
        assert_eq!(written["hooks"]["Stop"][0]["hooks"][0], json!({ "type": "command", "command": format!("\"{relay}\" bob Stop"), "timeout": 10 }));
        assert_eq!(std::fs::read_to_string(hooks_json::backup_path(&settings)).unwrap(), original);
        Bob.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bob_is_followed_only() {
        assert!(!Bob.capabilities().permissions);
        assert!(Bob.permission("PostToolUse", &fixture("PostToolUse")).is_none());
    }
}
