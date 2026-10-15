//! The common event format every connector translates its agent's messages into
//! (docs/02-ARCHITECTURE.md, "Le format d'événement commun").

use crate::connectors::custom_agent::CustomAgent;
use crate::connectors::diff::FileDiff;
use crate::locale::{Text, tr};
use crate::terminal::Host;
use serde::{Deserialize, Serialize};

/// Where an event comes from. The first four are îlot's original AIs; the next six are
/// followed for their activity only (docs/AGENTS.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    ClaudeCode,
    GeminiCli,
    Codex,
    Generic,
    Cursor,
    Copilot,
    #[serde(rename = "opencode")]
    OpenCode,
    Amp,
    Bob,
    Kiro,
}

impl Source {
    pub const ALL: [Source; 10] = [
        Self::ClaudeCode,
        Self::GeminiCli,
        Self::Codex,
        Self::Generic,
        Self::Cursor,
        Self::Copilot,
        Self::OpenCode,
        Self::Amp,
        Self::Bob,
        Self::Kiro,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::GeminiCli => "gemini-cli",
            Self::Codex => "codex",
            Self::Generic => "generic",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
            Self::OpenCode => "opencode",
            Self::Amp => "amp",
            Self::Bob => "bob",
            Self::Kiro => "kiro",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.id() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    SessionStart,
    Working,
    Tool,
    Permission,
    /// The agent waits for the user in its own terminal (a permission or a question îlot
    /// cannot answer): only shown.
    Asking,
    Notification,
    TurnDone,
    /// The user stopped the turn: it ends without a banner.
    Interrupted,
    Error,
    SessionEnd,
}

/// What a tool does, whatever the language: the counters read it, `Activity::verb` says
/// it to the user.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Run,
    Read,
    Edit,
    Write,
    /// Lists a folder.
    Browse,
    Search,
    /// Reads a web page.
    Fetch,
    Delegate,
    Plan,
    Ask,
    #[default]
    Use,
    Think,
    /// A generic tool's step (`ilot emit`).
    Work,
}

impl Action {
    /// The verb the island shows before the target.
    pub fn verb(self) -> Text<'static> {
        match self {
            Self::Run => Text::VerbRun,
            Self::Read => Text::VerbRead,
            Self::Edit => Text::VerbEdit,
            Self::Write => Text::VerbWrite,
            Self::Browse => Text::VerbBrowse,
            Self::Search => Text::VerbSearch,
            Self::Fetch => Text::VerbFetch,
            Self::Delegate => Text::VerbDelegate,
            Self::Plan => Text::VerbPlan,
            Self::Ask => Text::VerbAsk,
            Self::Use => Text::VerbUse,
            Self::Think => Text::VerbThink,
            Self::Work => Text::VerbWork,
        }
    }
}

/// What the agent does right now ("édite api.py"), and the lines it changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub action: Action,
    /// `action` in the language of the moment.
    pub verb: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub added: u32,
    pub removed: u32,
}

impl Activity {
    pub fn new(action: Action, target: Option<String>) -> Self {
        Self { action, verb: tr(action.verb()), target, added: 0, removed: 0 }
    }

    /// The title of a tool's event: "édite api.py".
    pub fn title(&self) -> String {
        match &self.target {
            Some(target) => format!("{} {target}", self.verb),
            None => self.verb.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub source: Source,
    pub session_id: String,
    /// Name of the working folder.
    pub project: String,
    pub cwd: String,
    pub kind: EventKind,
    /// Short text for the island.
    pub title: String,
    pub detail: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub at: u64,
    pub activity: Option<Activity>,
    /// Name the user gave a generic source (`ilot emit --source ci`), shown instead of
    /// the AI's name.
    pub name: Option<String>,
    /// A subagent of the session starts, stops or shows it still works.
    #[serde(skip)]
    pub subagent: Option<SubagentStep>,
    /// The badge a hook asked for with its `ilot_agent` key (name and color of its own).
    #[serde(skip)]
    pub agent: Option<CustomAgent>,
    /// What a successful edit changed, from the hook's own payload (memory only).
    #[serde(skip)]
    pub diff: Option<FileDiff>,
    /// Where the agent runs, as the relay saw it (terminal, editor).
    #[serde(skip)]
    pub terminal: Option<Host>,
}

/// What an event says about one subagent. Its tools carry its `agent_id`: a subagent whose
/// start was missed still counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubagentStep {
    Start { id: Option<String>, kind: Option<String> },
    Stop { id: Option<String> },
    Seen { id: String, kind: Option<String> },
}

impl AgentEvent {
    pub fn new(source: Source, session_id: &str, cwd: &str, kind: EventKind, title: impl Into<String>, at: u64) -> Self {
        Self {
            source,
            session_id: session_id.to_owned(),
            project: project_name(cwd),
            cwd: cwd.to_owned(),
            kind,
            title: title.into(),
            detail: None,
            at,
            activity: None,
            name: None,
            subagent: None,
            agent: None,
            diff: None,
            terminal: None,
        }
    }
}

/// Last component of a WSL or Windows path: `/home/user/projects/epilot` → `epilot`.
pub fn project_name(cwd: &str) -> String {
    cwd.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("?")
        .to_owned()
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_is_the_last_folder() {
        assert_eq!(project_name("/home/user/projects/epilot"), "epilot");
        assert_eq!(project_name("/home/user/projects/epilot/"), "epilot");
        assert_eq!(project_name(r"C:\Users\HP 240\dev\ilot"), "ilot");
        assert_eq!(project_name(""), "?");
        assert_eq!(project_name("/"), "?");
    }

    #[test]
    fn sources_round_trip_through_their_ids() {
        let ids = ["claude-code", "gemini-cli", "codex", "generic", "cursor", "copilot", "opencode", "amp", "bob", "kiro"];
        for id in ids {
            let source = Source::from_id(id).unwrap();
            assert_eq!(serde_json::to_value(source).unwrap(), id);
            assert_eq!(source.id(), id);
        }
        assert_eq!(Source::ALL.len(), ids.len());
        assert_eq!(Source::from_id("chatgpt"), None);
        assert_eq!(Source::from_id("open-code"), None);
    }
}
