//! Any other tool, through the `ilot` command (`crates/ilot-cli`): `ilot emit` and
//! `ilot run` send events of their own kinds (`working`, `turn_done`…) under the name the
//! user chose with `--source`, shown instead of an AI's name.
//!
//! Connecting installs `~/.local/bin/ilot` in a WSL distribution. Under Windows,
//! `ilot-cli.exe` ships next to îlot: there is nothing to plug.

use super::custom_agent::{clean, unreserved};
use super::{Answer, Caps, Connector, PermissionView, Place, Status, Target, text};
use crate::events::{Action, Activity, AgentEvent, EventKind, Source, project_name};
use crate::locale::{Text, tr};
use crate::wsl;
use serde_json::Value;
use std::io;
use std::path::Path;

/// The command next to the relay and îlot.
pub const CLI_EXE: &str = "ilot-cli.exe";
const NAME_CHARS: usize = 32;

/// `~/.local/bin/ilot` may already be the user's own: îlot never overwrites nor removes
/// a script it did not write. `None` when there is no such file.
fn ours(target: &Target) -> Option<bool> {
    std::fs::read_to_string(target.home_file(wsl::CLI_SCRIPT)).ok().map(|script| script.contains(wsl::SCRIPT_MARKER))
}

/// Any other tool. Its name never borrows the name of an AI îlot follows through its own
/// hooks: "Claude a terminé" sent by `ilot emit` shows as "Claude (autre)".
pub struct Generic;

impl Connector for Generic {
    fn id(&self) -> &'static str {
        "generic"
    }

    fn display_name(&self) -> String {
        tr(Text::ConnectorOther)
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: false }
    }

    fn uses_relay(&self) -> bool {
        false
    }

    fn status(&self, target: &Target) -> Status {
        match &target.place {
            Place::Windows { .. } => Status::NotInstalled,
            Place::Wsl { .. } => match ours(target) {
                Some(true) => Status::Connected,
                Some(false) => Status::NeedsRepair,
                None => Status::Installed,
            },
        }
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let Place::Wsl { distro, home } = &target.place else {
            return Err(io::Error::other(tr(Text::NothingToPlugOnWindows)));
        };
        if ours(target) == Some(false) {
            return Err(io::Error::other(tr(Text::NotWrittenByIlot { path: "~/.local/bin/ilot" })));
        }
        let cli = relay_exe.with_file_name(CLI_EXE);
        if !cli.exists() {
            return Err(io::Error::other(tr(Text::CommandMissing { path: &cli.display().to_string() })));
        }
        wsl::install_cli_script(distro, home, &cli).map(drop)
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        match &target.place {
            Place::Wsl { distro, home } if ours(target) == Some(true) => wsl::remove_script(distro, home, wsl::CLI_SCRIPT),
            _ => Ok(()),
        }
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        let name = unreserved(clean(text(payload, "name")?, NAME_CHARS)?);
        let session_id = text(payload, "session_id")?;
        let cwd = text(payload, "cwd").unwrap_or_default();
        let title = text(payload, "title").and_then(|t| clean(t, 120));
        let detail = text(payload, "detail").and_then(|d| clean(d, 120));
        let (kind, fallback) = match event {
            "session_start" => (EventKind::SessionStart, Text::AgentStarts),
            "working" => (EventKind::Working, Text::VerbWork),
            "turn_done" => (EventKind::TurnDone, Text::AgentFinished),
            "error" => (EventKind::Error, Text::AgentFailed),
            "session_end" => (EventKind::SessionEnd, Text::AgentClosedSession),
            _ => return None,
        };
        let title = title.unwrap_or_else(|| tr(fallback));
        let project = text(payload, "project").and_then(|p| clean(p, 64)).unwrap_or_else(|| project_name(cwd));
        let base = AgentEvent { project, name: Some(name), ..AgentEvent::new(Source::Generic, session_id, cwd, kind, &title, at) };
        Some(match kind {
            // The title is the task the session shows; the detail, what it does right now.
            EventKind::Working => AgentEvent {
                detail: Some(title),
                activity: Some(Activity::new(Action::Work, detail)),
                ..base
            },
            _ => AgentEvent { detail, ..base },
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
    use serde_json::json;

    /// What `ilot emit` and `ilot run` send as the event name (`crates/ilot-cli`).
    const KINDS: [&str; 5] = ["session_start", "working", "turn_done", "error", "session_end"];

    fn payload(title: &str) -> Value {
        json!({ "name": "ci", "session_id": "ci@epilot", "cwd": r"\\wsl.localhost\Ubuntu\home\user\epilot", "project": "epilot", "title": title })
    }

    #[test]
    fn every_kind_parses_under_the_chosen_name() {
        for kind in KINDS {
            let event = Generic.parse(kind, &payload("Analyse des logs"), 1).unwrap();
            assert_eq!(event.source, Source::Generic, "{kind}");
            assert_eq!(event.name.as_deref(), Some("ci"), "{kind}");
            assert_eq!(event.project, "epilot", "{kind}");
            assert_eq!(event.session_id, "ci@epilot", "{kind}");
        }
        assert!(Generic.parse("PermissionRequest", &payload("x"), 1).is_none());
    }

    #[test]
    fn working_gives_the_task_and_the_current_step() {
        let mut working = payload("Analyse des logs");
        working["detail"] = json!("lit app.log");
        let event = Generic.parse("working", &working, 1).unwrap();
        assert_eq!(event.kind, EventKind::Working);
        assert_eq!(event.detail.as_deref(), Some("Analyse des logs"));
        assert_eq!(event.activity.unwrap().target.as_deref(), Some("lit app.log"));
        let mut failed = payload("");
        failed["detail"] = json!("code de sortie 2");
        let event = Generic.parse("error", &failed, 1).unwrap();
        assert_eq!((event.title.as_str(), event.detail.as_deref()), ("a échoué", Some("code de sortie 2")));
    }

    #[test]
    fn names_of_the_followed_ais_are_marked() {
        for (name, shown) in [
            ("Claude", "Claude (autre)"),
            ("claude-code", "claude-code (autre)"),
            ("Gemini CLI v2", "Gemini CLI v2 (autre)"),
            ("CODEX", "CODEX (autre)"),
            ("Claudette", "Claudette"),
            ("ci", "ci"),
        ] {
            let mut named = payload("x");
            named["name"] = json!(name);
            assert_eq!(Generic.parse("working", &named, 1).unwrap().name.unwrap(), shown);
        }
    }

    #[test]
    fn names_cannot_disguise_themselves() {
        let mut spoof = payload("x");
        spoof["name"] = json!("Claude\u{202e}\u{200b}\nrm -rf");
        assert_eq!(Generic.parse("working", &spoof, 1).unwrap().name.as_deref(), Some("Claude (autre)"));
        spoof["name"] = json!("a".repeat(100));
        assert_eq!(Generic.parse("working", &spoof, 1).unwrap().name.unwrap(), format!("{}…", "a".repeat(NAME_CHARS)));
        spoof["name"] = json!("\u{200b}");
        assert!(Generic.parse("working", &spoof, 1).is_none(), "an empty name is refused");
    }

    #[test]
    fn the_project_defaults_to_the_working_folder() {
        let mut no_project = payload("x");
        no_project.as_object_mut().unwrap().remove("project");
        no_project["cwd"] = json!(r"C:\Users\HP 240\dev\site");
        assert_eq!(Generic.parse("working", &no_project, 1).unwrap().project, "site");
    }

    #[test]
    fn nothing_to_plug_under_windows_and_no_relay_needed() {
        let windows = Target::windows(std::env::temp_dir());
        assert_eq!(Generic.status(&windows), Status::NotInstalled);
        assert!(Generic.connect(&windows, Path::new(r"C:\ilot\ilot-hook.exe")).is_err());
        assert!(!Generic.uses_relay());
        assert!(!Generic.capabilities().permissions);
    }

    /// Real WSL: installs `~/.local/bin/ilot` on a throwaway home, runs it, removes it.
    #[test]
    #[ignore = "needs WSL and ilot-cli.exe built; run with --ignored"]
    fn the_ilot_script_works_inside_wsl_on_a_throwaway_home() {
        use std::process::Command;
        let distro = crate::wsl::distros().into_iter().next().expect("a WSL distribution");
        let home = format!("/tmp/ilot-test-generic-{}", std::process::id());
        let target = Target::wsl(&distro, &home);
        std::fs::create_dir_all(crate::wsl::unc_path(&distro, &home)).unwrap();
        let relay = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("ilot-hook.exe");
        assert_eq!(Generic.status(&target), Status::Installed);
        Generic.connect(&target, &relay).unwrap();
        assert_eq!(Generic.status(&target), Status::Connected);
        let script = format!("{home}/{}", wsl::CLI_SCRIPT);
        let run = |args: &[&str]| {
            let mut all = vec!["-d", distro.as_str(), "--cd", "/tmp", "--exec", script.as_str()];
            all.extend_from_slice(args);
            Command::new("wsl.exe").args(&all).output().unwrap()
        };
        // The command runs in Linux, its arguments untouched, and gives its exit code back.
        let out = run(&["run", "--source", "test", "--", "sh", "-c", "printf '%s|' \"$@\"; exit 3", "x", "a b", "$HOME", "*"]);
        assert_eq!(out.status.code(), Some(3), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "a b|$HOME|*|");
        assert_eq!(run(&["run", "--source", "test", "--", "true"]).status.code(), Some(0));
        assert_eq!(run(&["emit", "--source", "test", "--kind", "nope"]).status.code(), Some(2));
        assert_eq!(run(&["run", "--source"]).status.code(), Some(2));
        Generic.disconnect(&target).unwrap();
        assert_eq!(Generic.status(&target), Status::Installed);
        // A `~/.local/bin/ilot` of the user's own is never overwritten nor removed.
        let own = crate::wsl::unc_path(&distro, &script);
        std::fs::write(&own, "#!/bin/sh\necho mine\n").unwrap();
        assert_eq!(Generic.status(&target), Status::NeedsRepair);
        assert!(Generic.connect(&target, &relay).is_err());
        Generic.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&own).unwrap(), "#!/bin/sh\necho mine\n");
        std::fs::remove_dir_all(crate::wsl::unc_path(&distro, &home)).unwrap();
    }

    #[test]
    fn titles_names_and_errors_follow_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        let event = Generic.parse("error", &payload(""), 1).unwrap();
        assert_eq!(event.title, "failed");
        let working = Generic.parse("working", &json!({ "name": "Claude", "session_id": "s", "cwd": "/p", "title": "Build" }), 1).unwrap();
        assert_eq!((working.name.as_deref(), working.activity.unwrap().verb.as_str()), (Some("Claude (other)"), "works"));
        assert_eq!(Generic.display_name(), "Other");
        let windows = Target::windows(std::env::temp_dir());
        let error = Generic.connect(&windows, Path::new(r"C:\ilot\ilot-hook.exe")).unwrap_err();
        assert_eq!(error.to_string(), "Nothing to connect on Windows: ilot-cli.exe comes with îlot.");
    }
}
