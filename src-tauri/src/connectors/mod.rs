//! Connectors: one per AI. Each one finds the installations, plugs îlot's relay into the
//! agent's config on the user's click only, and translates the agent's messages into
//! the common `AgentEvent` (docs/04-CONNECTEURS-IA.md, docs/AGENTS.md).

pub mod activity;
pub mod amp;
pub mod bob;
pub mod claude_code;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod custom_agent;
pub mod diff;
pub mod gemini_cli;
pub mod generic;
pub mod hook_events;
pub mod hooks_json;
pub mod kiro;
pub mod opencode;
pub mod owned_file;
pub mod questions;

use crate::events::AgentEvent;
use crate::risk::Risk;
use crate::wsl;
use serde::Serialize;
use serde_json::Value;
use std::io;
use std::path::{Path, PathBuf};

/// Where an agent is installed: Windows itself, or a WSL distribution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Windows { home: PathBuf },
    Wsl { distro: String, home: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    /// `windows` or `wsl:<distro>`.
    pub id: String,
    pub label: String,
    #[serde(skip)]
    pub place: Place,
}

impl Target {
    pub fn windows(home: PathBuf) -> Self {
        Self { id: "windows".into(), label: "Windows".into(), place: Place::Windows { home } }
    }

    pub fn wsl(distro: &str, home: &str) -> Self {
        Self {
            id: format!("wsl:{distro}"),
            label: format!("WSL · {distro}"),
            place: Place::Wsl { distro: distro.into(), home: home.into() },
        }
    }

    pub fn is_windows(&self) -> bool {
        matches!(self.place, Place::Windows { .. })
    }

    /// Windows path of a file relative to the user's home in this place.
    pub fn home_file(&self, relative: &str) -> PathBuf {
        match &self.place {
            Place::Windows { home } => relative.split('/').fold(home.clone(), |p, part| p.join(part)),
            Place::Wsl { distro, home } => wsl::unc_path(distro, &format!("{home}/{relative}")),
        }
    }

    /// The command the agent's hooks call: the relay itself on Windows, the relay
    /// script inside WSL (installed on the way).
    pub fn install_relay(&self, relay_exe: &Path) -> io::Result<String> {
        match &self.place {
            Place::Windows { .. } => relay_exe
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| io::Error::other("non UTF-8 path")),
            Place::Wsl { distro, home } => wsl::install_relay_script(distro, home, relay_exe),
        }
    }
}

/// Installations of the current user: Windows, then each WSL distribution.
pub fn all_targets() -> Vec<Target> {
    let mut targets = Vec::new();
    if let Some(home) = std::env::var_os("USERPROFILE") {
        targets.push(Target::windows(PathBuf::from(home)));
    }
    for distro in wsl::distros() {
        if let Some(home) = wsl::linux_home(&distro) {
            targets.push(Target::wsl(&distro, &home));
        }
    }
    targets
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    NotInstalled,
    Installed,
    Connected,
    NeedsRepair,
}

/// What a connector can follow, shown in the settings.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Caps {
    pub activity: bool,
    pub permissions: bool,
}

/// How the user answered a permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Deny,
    Always,
    Allow,
    /// The user answers in the agent's terminal: the relay prints nothing.
    Terminal,
    /// « Tout suspendre » refuses it. Given by the core only: the interface cannot send it.
    #[serde(skip)]
    Suspended,
}

impl Answer {
    pub fn allows(self) -> bool {
        matches!(self, Answer::Allow | Answer::Always)
    }
}

/// A permission request as the island shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionView {
    pub title: String,
    pub detail: String,
    /// What a file tool will write, as `-` and `+` lines.
    pub content: Option<String>,
    /// Other fields of the request that change what happens, as `name: value`.
    pub extras: Vec<String>,
    pub reason: Option<String>,
    pub truncated: bool,
    pub can_always: bool,
    /// For a shell command: `high` with what makes it risky (`risk::assess`).
    pub risk: Risk,
    pub warnings: Vec<String>,
    /// Questions the agent asks the user (Claude's `AskUserQuestion`): the island shows
    /// them instead of a permission. Empty for a permission.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<questions::Question>,
}

pub trait Connector: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> String;
    fn capabilities(&self) -> Caps;
    /// Whether the agent calls the relay (`ilot-hook`), which stays installed in a WSL
    /// distribution as long as one such connector is plugged there.
    fn uses_relay(&self) -> bool {
        true
    }
    fn status(&self, target: &Target) -> Status;
    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()>;
    fn disconnect(&self, target: &Target) -> io::Result<()>;
    /// Translates one hook call; `None` for events the island does not show.
    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent>;
    /// For a waiting event: what the island shows.
    fn permission(&self, event: &str, payload: &Value) -> Option<PermissionView>;
    /// The exact text the relay prints for the agent once the user answered.
    fn answer(&self, event: &str, payload: &Value, answer: Answer) -> Option<String>;
    /// The exact text the relay prints for the agent once the user answered its
    /// questions; `None` when the answers do not fit them (nothing is sent).
    fn answer_questions(&self, _event: &str, _payload: &Value, _answers: &[questions::QuestionAnswer]) -> Option<String> {
        None
    }
}

/// The original four, then the agents followed for their activity (docs/AGENTS.md).
pub fn registry() -> Vec<Box<dyn Connector>> {
    vec![
        Box::new(claude_code::ClaudeCode),
        Box::new(gemini_cli::GeminiCli),
        Box::new(codex::Codex),
        Box::new(generic::Generic),
        Box::new(cursor::Cursor),
        Box::new(copilot::Copilot),
        Box::new(opencode::OpenCode),
        Box::new(amp::Amp),
        Box::new(bob::Bob),
        Box::new(kiro::Kiro),
    ]
}

pub fn find(id: &str) -> Option<Box<dyn Connector>> {
    registry().into_iter().find(|c| c.id() == id)
}

/// Removes the WSL relay script once no connector uses this distribution any more.
pub fn tidy_relay(target: &Target) -> io::Result<()> {
    let Place::Wsl { distro, home } = &target.place else { return Ok(()) };
    let in_use = registry()
        .iter()
        .filter(|c| c.uses_relay())
        .any(|c| matches!(c.status(target), Status::Connected | Status::NeedsRepair));
    if in_use { Ok(()) } else { wsl::remove_script(distro, home, wsl::RELAY_SCRIPT) }
}

/// Short string field of a JSON payload.
pub fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_files_resolve_on_windows_and_wsl() {
        let windows = Target::windows(PathBuf::from(r"C:\Users\HP 240"));
        assert_eq!(windows.home_file(".claude/settings.json"), PathBuf::from(r"C:\Users\HP 240\.claude\settings.json"));
        let wsl = Target::wsl("Ubuntu", "/home/hic");
        assert_eq!(wsl.id, "wsl:Ubuntu");
        assert_eq!(
            wsl.home_file(".claude/settings.json"),
            PathBuf::from(r"\\wsl.localhost\Ubuntu\home\hic\.claude\settings.json")
        );
    }

    #[test]
    fn every_connector_is_listed_once_with_its_source() {
        let ids: Vec<_> = registry().iter().map(|c| c.id()).collect();
        assert_eq!(ids, ["claude-code", "gemini-cli", "codex", "generic", "cursor", "copilot", "opencode", "amp", "bob", "kiro"]);
        for id in ids {
            assert_eq!(crate::events::Source::from_id(id).map(crate::events::Source::id), Some(id));
            assert_eq!(find(id).unwrap().id(), id);
        }
    }

    #[test]
    fn only_claude_and_codex_answer_for_the_user() {
        for connector in registry() {
            let answers = matches!(connector.id(), "claude-code" | "codex");
            assert_eq!(connector.capabilities().permissions, answers, "{}", connector.id());
        }
    }

    #[test]
    fn answers_parse_from_the_interface() {
        let answer: Answer = serde_json::from_str("\"always\"").unwrap();
        assert_eq!(answer, Answer::Always);
        assert!(serde_json::from_str::<Answer>("\"maybe\"").is_err());
        assert!(serde_json::from_str::<Answer>("\"suspended\"").is_err(), "only the core suspends");
    }
}
