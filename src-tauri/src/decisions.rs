//! The decisions journal: what became of each permission request, for the panel
//! (« Dernière décision », the Agents tab). Kept in memory only, never written to disk,
//! and capped: the oldest entries leave first.

use crate::connectors::Answer;
use crate::events::Source;
use crate::permissions::{ANSWER_MARGIN, PERMISSION_TIMEOUT_MS, Pending};
use serde::Serialize;
use std::collections::VecDeque;
use std::time::Duration;

/// Entries kept: a long day of work, a few kilobytes each at most.
pub const MAX_DECISIONS: usize = 200;
/// A longer command is cut in the journal (the request itself was shown whole).
pub const MAX_COMMAND_CHARS: usize = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionKind {
    Allow,
    Deny,
    Always,
    /// Refused by « Tout suspendre ».
    Suspended,
    /// Answered in the agent's terminal, or given up by the agent.
    Terminal,
    /// Nobody answered in time: the agent asked in its terminal.
    Expired,
}

impl From<Answer> for DecisionKind {
    fn from(answer: Answer) -> Self {
        match answer {
            Answer::Allow => Self::Allow,
            Answer::Deny => Self::Deny,
            Answer::Always => Self::Always,
            Answer::Terminal => Self::Terminal,
            Answer::Suspended => Self::Suspended,
        }
    }
}

/// One line of the journal (type `Decision` in `src/lib/ipc.ts`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Decision {
    /// Epoch milliseconds.
    pub at: u64,
    pub source: Source,
    pub project: String,
    /// What was asked: the command, the file or the address.
    pub command: String,
    pub decision: DecisionKind,
}

impl Decision {
    pub fn of(pending: &Pending, decision: DecisionKind, at: u64) -> Self {
        let detail = &pending.view.detail;
        let command = match detail.char_indices().nth(MAX_COMMAND_CHARS) {
            Some((cut, _)) => format!("{}…", &detail[..cut]),
            None => detail.clone(),
        };
        Self { at, source: pending.source, project: pending.project.clone(), command, decision }
    }
}

/// How a request that left without an answer from îlot ended: expired when it waited
/// until the end (the interface already said « Réponds dans le terminal »), otherwise
/// the agent stopped waiting, most likely answered in its terminal.
pub fn ended_without_answer(waited: Duration) -> DecisionKind {
    if waited + ANSWER_MARGIN >= Duration::from_millis(PERMISSION_TIMEOUT_MS) {
        DecisionKind::Expired
    } else {
        DecisionKind::Terminal
    }
}

#[derive(Default)]
pub struct Journal {
    entries: VecDeque<Decision>,
}

impl Journal {
    pub fn add(&mut self, decision: Decision) {
        self.entries.push_front(decision);
        self.entries.truncate(MAX_DECISIONS);
    }

    /// Newest first.
    pub fn list(&self) -> Vec<Decision> {
        self.entries.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(at: u64) -> Decision {
        Decision { at, source: Source::Codex, project: "demo".into(), command: "ls".into(), decision: DecisionKind::Deny }
    }

    #[test]
    fn the_journal_keeps_the_newest_first_and_forgets_the_oldest() {
        let mut journal = Journal::default();
        for at in 0..(MAX_DECISIONS as u64 + 5) {
            journal.add(decision(at));
        }
        let list = journal.list();
        assert_eq!(list.len(), MAX_DECISIONS);
        assert_eq!(list[0].at, MAX_DECISIONS as u64 + 4);
        assert_eq!(list.last().unwrap().at, 5);
    }

    #[test]
    fn decisions_serialize_for_the_interface() {
        let value = serde_json::to_value(decision(42)).unwrap();
        assert_eq!(value, serde_json::json!({ "at": 42, "source": "codex", "project": "demo", "command": "ls", "decision": "deny" }));
        for (answer, name) in [
            (Answer::Allow, "allow"),
            (Answer::Deny, "deny"),
            (Answer::Always, "always"),
            (Answer::Terminal, "terminal"),
            (Answer::Suspended, "suspended"),
        ] {
            assert_eq!(serde_json::to_value(DecisionKind::from(answer)).unwrap(), name);
        }
        assert_eq!(serde_json::to_value(DecisionKind::Expired).unwrap(), "expired");
    }

    #[test]
    fn a_long_command_is_cut_in_the_journal() {
        use crate::connectors::PermissionView;
        let (reply, _receiver) = tokio::sync::oneshot::channel();
        let view = PermissionView {
            title: "Codex veut exécuter une commande".into(),
            detail: "é".repeat(MAX_COMMAND_CHARS + 10),
            content: None,
            extras: Vec::new(),
            reason: None,
            truncated: false,
            can_always: false,
            risk: crate::risk::Risk::Low,
            warnings: Vec::new(),
            questions: Vec::new(),
        };
        let pending = Pending {
            id: "a".into(),
            source: Source::Codex,
            session_key: "codex:s1".into(),
            project: "demo".into(),
            event: "PermissionRequest".into(),
            payload: serde_json::Value::Null,
            view,
            folder: "~/demo".into(),
            received_at: 1,
            timeout_ms: PERMISSION_TIMEOUT_MS,
            waiting_since: std::time::Instant::now(),
            reply,
        };
        let decision = Decision::of(&pending, DecisionKind::Allow, 7);
        assert_eq!(decision.command.chars().count(), MAX_COMMAND_CHARS + 1);
        assert!(decision.command.ends_with("é…"));
        assert_eq!((decision.at, decision.project.as_str()), (7, "demo"));
    }

    #[test]
    fn a_request_that_waited_until_the_end_expired() {
        let timeout = Duration::from_millis(PERMISSION_TIMEOUT_MS);
        assert_eq!(ended_without_answer(timeout), DecisionKind::Expired);
        assert_eq!(ended_without_answer(timeout - ANSWER_MARGIN), DecisionKind::Expired, "the relay leaves a little early");
        assert_eq!(ended_without_answer(Duration::from_secs(3)), DecisionKind::Terminal);
    }
}
