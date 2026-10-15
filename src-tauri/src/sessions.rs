//! Agent sessions built from events: status, current activity, lines changed during the
//! turn, and the rules for pausing and forgetting quiet sessions (docs/04, "Compteurs").
//! A session is "active" while a turn runs (from the prompt to its end: `Stop`,
//! `AfterAgent`, `turn_done`…): that is what the counters show, the same way for every
//! AI. Sessions live in memory only.

use crate::connectors::custom_agent::CustomAgent;
use crate::connectors::diff::FileDiff;
use crate::events::{Activity, AgentEvent, EventKind, Source, SubagentStep};
use crate::terminal::Host;
use serde::Serialize;

/// No event for this long: the session is shown as paused.
pub const PAUSE_AFTER_MS: u64 = 30 * 60 * 1000;
/// No event for this long: the session leaves the list.
pub const FORGET_AFTER_MS: u64 = 2 * 60 * 60 * 1000;
/// A subagent silent this long is counted as gone (its `SubagentStop` was missed).
pub const SUBAGENT_QUIET_MS: u64 = 20 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    Active,
    Waiting,
    Paused,
    Done,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// `<source>:<session id>`.
    pub id: String,
    pub source: Source,
    /// Name chosen for a generic source; the AI's own name otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub project: String,
    pub task: String,
    pub status: SessionStatus,
    pub activity: Option<Activity>,
    /// Subagents working for this session now. They run in the background by default and
    /// may outlive the turn that started them.
    pub subagents: Vec<Subagent>,
    /// The badge its hooks asked for (`ilot_agent`): its own name and color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<CustomAgent>,
    /// It waits for an answer in its own terminal, which îlot cannot give.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub in_terminal: bool,
    /// Where it runs, as far as the relay could tell (« Aller au terminal »).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<Host>,
    /// Its latest change, in memory only, fetched on demand (`session_diff`).
    #[serde(skip)]
    diff: Option<FileDiff>,
    #[serde(skip)]
    last_event: u64,
    #[serde(skip)]
    turn_started: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subagent {
    /// The agent's `agent_id`, or one made by îlot when it sent none.
    pub id: String,
    /// `agent_type`: "Explore", "Plan", a custom agent's name…
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip)]
    last_seen: u64,
}

/// An agent finished its turn: what the banner says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    pub source: Source,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub project: String,
    pub duration_ms: u64,
    pub added: u32,
    pub removed: u32,
    /// The session (`Session::id`), to open its latest change from the banner.
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<CustomAgent>,
    /// The file of its latest change when the turn changed lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Default)]
pub struct Sessions {
    list: Vec<Session>,
}

pub fn session_key(source: Source, session_id: &str) -> String {
    format!("{}:{session_id}", source.id())
}

impl Sessions {
    pub fn list(&self) -> &[Session] {
        &self.list
    }

    pub fn get(&self, key: &str) -> Option<&Session> {
        self.list.iter().find(|s| s.id == key)
    }

    /// The latest change of a session (memory only).
    pub fn diff(&self, key: &str) -> Option<&FileDiff> {
        self.get(key)?.diff.as_ref()
    }

    /// The session « Aller au terminal » goes to when none is named: one that waits for
    /// the user, else one at work, else the latest one heard from.
    pub fn followed(&self) -> Option<&Session> {
        self.list
            .iter()
            .find(|s| s.status == SessionStatus::Waiting)
            .or_else(|| self.list.iter().find(|s| s.status == SessionStatus::Active))
            .or_else(|| self.list.iter().max_by_key(|s| s.last_event))
    }

    /// Applies one event. Returns what to announce when a turn ends.
    pub fn apply(&mut self, event: &AgentEvent) -> Option<Finished> {
        let key = session_key(event.source, &event.session_id);
        if event.kind == EventKind::SessionEnd {
            self.list.retain(|s| s.id != key);
            return None;
        }
        let index = match self.list.iter().position(|s| s.id == key) {
            Some(index) => index,
            None => {
                self.list.push(Session {
                    id: key,
                    source: event.source,
                    name: event.name.clone(),
                    project: event.project.clone(),
                    task: String::new(),
                    status: SessionStatus::Paused,
                    activity: None,
                    subagents: Vec::new(),
                    agent: None,
                    in_terminal: false,
                    terminal: None,
                    diff: None,
                    last_event: event.at,
                    turn_started: 0,
                });
                self.list.len() - 1
            }
        };
        let session = &mut self.list[index];
        session.last_event = event.at;
        if let Some(step) = &event.subagent {
            track(&mut session.subagents, step, event.at);
        }
        if event.agent.is_some() {
            session.agent.clone_from(&event.agent);
        }
        if event.terminal.is_some() {
            session.terminal = event.terminal;
        }
        if event.diff.is_some() {
            session.diff.clone_from(&event.diff);
        }
        if !matches!(event.kind, EventKind::SessionStart | EventKind::Notification | EventKind::SessionEnd) {
            session.in_terminal = event.kind == EventKind::Asking;
        }
        match event.kind {
            EventKind::SessionStart | EventKind::Notification | EventKind::SessionEnd => None,
            EventKind::Working => {
                session.status = SessionStatus::Active;
                session.turn_started = event.at;
                session.activity = event.activity.clone();
                if let Some(task) = &event.detail {
                    session.task.clone_from(task);
                }
                None
            }
            EventKind::Tool => {
                session.status = SessionStatus::Active;
                if session.turn_started == 0 {
                    session.turn_started = event.at;
                }
                if let Some(activity) = &event.activity {
                    let (added, removed) = session.activity.as_ref().map_or((0, 0), |a| (a.added, a.removed));
                    session.activity = Some(Activity {
                        action: activity.action,
                        verb: activity.verb.clone(),
                        target: activity.target.clone(),
                        added: added + activity.added,
                        removed: removed + activity.removed,
                    });
                }
                None
            }
            EventKind::Permission | EventKind::Asking => {
                session.status = SessionStatus::Waiting;
                None
            }
            // A turn ends once: a second end (Codex `notify` after `Stop`, Gemini retrying
            // `AfterAgent`) is not announced again.
            EventKind::TurnDone if session.status == SessionStatus::Done => None,
            EventKind::TurnDone => {
                session.status = SessionStatus::Done;
                let (added, removed) = session.activity.as_ref().map_or((0, 0), |a| (a.added, a.removed));
                let duration_ms = if session.turn_started > 0 { event.at.saturating_sub(session.turn_started) } else { 0 };
                session.turn_started = 0;
                Some(Finished {
                    source: session.source,
                    name: session.name.clone(),
                    project: session.project.clone(),
                    duration_ms,
                    added,
                    removed,
                    session_id: session.id.clone(),
                    agent: session.agent.clone(),
                    file: session.diff.as_ref().filter(|_| added + removed > 0).map(|diff| diff.file.clone()),
                })
            }
            EventKind::Interrupted => {
                session.status = SessionStatus::Done;
                session.turn_started = 0;
                None
            }
            EventKind::Error => {
                session.status = SessionStatus::Failed;
                session.turn_started = 0;
                None
            }
        }
    }

    /// The user answered a permission from îlot: the agent goes on.
    pub fn resume(&mut self, key: &str) {
        if let Some(session) = self.list.iter_mut().find(|s| s.id == key && s.status == SessionStatus::Waiting) {
            session.status = SessionStatus::Active;
        }
    }

    /// Pauses quiet sessions, forgets old ones and the subagents gone silent. Returns true
    /// when the list changed.
    pub fn tick(&mut self, now: u64) -> bool {
        let before = self.list.clone();
        self.list.retain(|s| now.saturating_sub(s.last_event) < FORGET_AFTER_MS);
        for session in &mut self.list {
            let live = matches!(session.status, SessionStatus::Active | SessionStatus::Waiting);
            if live && now.saturating_sub(session.last_event) >= PAUSE_AFTER_MS {
                session.status = SessionStatus::Paused;
            }
            session.subagents.retain(|a| now.saturating_sub(a.last_seen) < SUBAGENT_QUIET_MS);
        }
        self.list != before
    }
}

/// Keeps the session's subagents up to date. A stop without an id closes the oldest one
/// without an id from the agent.
fn track(subagents: &mut Vec<Subagent>, step: &SubagentStep, at: u64) {
    match step {
        SubagentStep::Start { id, kind } => {
            let id = id.clone().unwrap_or_else(|| format!("ilot-{at}-{}", subagents.len()));
            seen(subagents, id, kind, at);
        }
        SubagentStep::Seen { id, kind } => seen(subagents, id.clone(), kind, at),
        SubagentStep::Stop { id: Some(id) } => subagents.retain(|a| &a.id != id),
        SubagentStep::Stop { id: None } => {
            if let Some(index) = subagents.iter().position(|a| a.id.starts_with("ilot-")) {
                subagents.remove(index);
            }
        }
    }
}

fn seen(subagents: &mut Vec<Subagent>, id: String, kind: &Option<String>, at: u64) {
    match subagents.iter_mut().find(|a| a.id == id) {
        Some(agent) => {
            agent.last_seen = at;
            if agent.kind.is_none() {
                agent.kind.clone_from(kind);
            }
        }
        None => subagents.push(Subagent { id, kind: kind.clone(), last_seen: at }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Action;

    fn event(kind: EventKind, at: u64) -> AgentEvent {
        AgentEvent::new(Source::ClaudeCode, "s1", "/home/user/epilot", kind, "", at)
    }

    fn edit(at: u64, added: u32, removed: u32) -> AgentEvent {
        AgentEvent {
            activity: Some(Activity { action: Action::Edit, verb: "édite".into(), target: Some("api.py".into()), added, removed }),
            ..event(EventKind::Tool, at)
        }
    }

    #[test]
    fn a_turn_runs_from_the_prompt_to_stop() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::SessionStart, 0));
        assert_eq!(sessions.list()[0].status, SessionStatus::Paused);
        let prompt = AgentEvent { detail: Some("corrige le fuseau horaire".into()), ..event(EventKind::Working, 1_000) };
        sessions.apply(&prompt);
        assert_eq!(sessions.list()[0].status, SessionStatus::Active);
        assert_eq!(sessions.list()[0].task, "corrige le fuseau horaire");
        sessions.apply(&edit(2_000, 10, 2));
        sessions.apply(&edit(3_000, 2, 1));
        let activity = sessions.list()[0].activity.clone().unwrap();
        assert_eq!((activity.verb.as_str(), activity.added, activity.removed), ("édite", 12, 3));
        let finished = sessions.apply(&event(EventKind::TurnDone, 241_000)).unwrap();
        assert_eq!(
            finished,
            Finished {
                source: Source::ClaudeCode,
                name: None,
                project: "epilot".into(),
                duration_ms: 240_000,
                added: 12,
                removed: 3,
                session_id: "claude-code:s1".into(),
                agent: None,
                file: None,
            }
        );
        assert_eq!(sessions.list()[0].status, SessionStatus::Done);
    }

    #[test]
    fn a_new_prompt_starts_counting_lines_again() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        sessions.apply(&edit(1, 5, 0));
        sessions.apply(&event(EventKind::TurnDone, 2));
        sessions.apply(&event(EventKind::Working, 3));
        assert_eq!(sessions.apply(&event(EventKind::TurnDone, 4)).unwrap().added, 0);
    }

    #[test]
    fn a_permission_makes_the_session_wait_until_it_goes_on() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        sessions.apply(&event(EventKind::Permission, 1));
        assert_eq!(sessions.list()[0].status, SessionStatus::Waiting);
        sessions.resume("claude-code:s1");
        assert_eq!(sessions.list()[0].status, SessionStatus::Active);
        sessions.apply(&event(EventKind::Permission, 2));
        sessions.apply(&edit(3, 0, 0));
        assert_eq!(sessions.list()[0].status, SessionStatus::Active);
    }

    #[test]
    fn errors_fail_and_session_end_removes() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        sessions.apply(&event(EventKind::Error, 1));
        assert_eq!(sessions.list()[0].status, SessionStatus::Failed);
        sessions.apply(&event(EventKind::SessionEnd, 2));
        assert!(sessions.list().is_empty());
    }

    #[test]
    fn quiet_sessions_pause_then_disappear() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        assert!(!sessions.tick(PAUSE_AFTER_MS - 1));
        assert!(sessions.tick(PAUSE_AFTER_MS));
        assert_eq!(sessions.list()[0].status, SessionStatus::Paused);
        assert!(sessions.tick(FORGET_AFTER_MS));
        assert!(sessions.list().is_empty());
    }

    #[test]
    fn sessions_of_different_agents_stay_apart() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        sessions.apply(&AgentEvent { source: Source::Codex, ..event(EventKind::Working, 0) });
        assert_eq!(sessions.list().len(), 2);
        assert_eq!(sessions.list()[1].id, "codex:s1");
    }

    fn live_counts(sessions: &Sessions) -> [usize; 4] {
        let live = |source| {
            sessions
                .list()
                .iter()
                .filter(|s| s.source == source && matches!(s.status, SessionStatus::Active | SessionStatus::Waiting))
                .count()
        };
        [live(Source::ClaudeCode), live(Source::GeminiCli), live(Source::Codex), live(Source::Generic)]
    }

    fn from(source: Source, session: &str, kind: EventKind, at: u64) -> AgentEvent {
        AgentEvent::new(source, session, "/home/user/epilot", kind, "", at)
    }

    #[test]
    fn three_sessions_of_three_ais_in_parallel_are_counted_right() {
        let mut sessions = Sessions::default();
        let ais = [(Source::ClaudeCode, "c1"), (Source::GeminiCli, "g1"), (Source::Codex, "x1")];
        for (source, id) in ais {
            sessions.apply(&from(source, id, EventKind::SessionStart, 0));
        }
        assert_eq!(live_counts(&sessions), [0, 0, 0, 0], "an open session without a turn does not count");
        for (source, id) in ais {
            sessions.apply(&from(source, id, EventKind::Working, 1));
            sessions.apply(&from(source, id, EventKind::Tool, 2));
        }
        assert_eq!(live_counts(&sessions), [1, 1, 1, 0]);
        // Gemini says it waits in its terminal; Codex asks îlot: both still count.
        sessions.apply(&from(Source::GeminiCli, "g1", EventKind::Permission, 3));
        sessions.apply(&from(Source::Codex, "x1", EventKind::Permission, 3));
        assert_eq!(live_counts(&sessions), [1, 1, 1, 0]);
        // Claude ends its turn, the user stops Codex, a second Claude session starts working.
        assert!(sessions.apply(&from(Source::ClaudeCode, "c1", EventKind::TurnDone, 4)).is_some());
        assert!(sessions.apply(&from(Source::Codex, "x1", EventKind::Interrupted, 4)).is_none());
        sessions.apply(&from(Source::ClaudeCode, "c2", EventKind::Working, 5));
        assert_eq!(live_counts(&sessions), [1, 1, 0, 0]);
        assert_eq!(sessions.list().len(), 4, "finished sessions stay listed");
        // Gemini goes on after the user's answer, then ends; Claude's new session closes.
        sessions.apply(&from(Source::GeminiCli, "g1", EventKind::Tool, 6));
        sessions.apply(&from(Source::GeminiCli, "g1", EventKind::TurnDone, 7));
        sessions.apply(&from(Source::ClaudeCode, "c2", EventKind::SessionEnd, 8));
        assert_eq!(live_counts(&sessions), [0, 0, 0, 0]);
        assert_eq!(sessions.list().len(), 3);
        // The same rules pause and forget every AI's sessions.
        sessions.apply(&from(Source::Codex, "x1", EventKind::Working, 10));
        sessions.tick(10 + PAUSE_AFTER_MS);
        assert_eq!(live_counts(&sessions), [0, 0, 0, 0]);
        sessions.tick(10 + FORGET_AFTER_MS);
        assert!(sessions.list().is_empty());
    }

    #[test]
    fn subagents_work_inside_their_parent_session() {
        let mut sessions = Sessions::default();
        for source in [Source::ClaudeCode, Source::Codex] {
            sessions.apply(&from(source, "parent", EventKind::Working, 0));
            // SubagentStart, the subagent's tools and SubagentStop carry the parent's id.
            for _ in 0..3 {
                sessions.apply(&from(source, "parent", EventKind::Tool, 1));
            }
        }
        assert_eq!(sessions.list().len(), 2);
        assert_eq!(live_counts(&sessions), [1, 0, 1, 0]);
    }

    fn sub(at: u64, step: SubagentStep) -> AgentEvent {
        AgentEvent { subagent: Some(step), ..event(EventKind::Tool, at) }
    }

    fn kinds(sessions: &Sessions) -> Vec<(String, Option<String>)> {
        sessions.list()[0].subagents.iter().map(|a| (a.id.clone(), a.kind.clone())).collect()
    }

    #[test]
    fn subagents_are_counted_from_start_to_stop() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        let start = |id: &str, kind: &str| SubagentStep::Start { id: Some(id.into()), kind: Some(kind.into()) };
        sessions.apply(&sub(1, start("a1", "Explore")));
        sessions.apply(&sub(2, start("a2", "Explore")));
        sessions.apply(&sub(3, start("a3", "Plan")));
        assert_eq!(sessions.list()[0].subagents.len(), 3);
        sessions.apply(&sub(4, SubagentStep::Stop { id: Some("a2".into()) }));
        assert_eq!(kinds(&sessions), [("a1".into(), Some("Explore".into())), ("a3".into(), Some("Plan".into()))]);
        // Background subagents outlive the turn.
        sessions.apply(&event(EventKind::TurnDone, 5));
        assert_eq!(sessions.list()[0].subagents.len(), 2);
    }

    #[test]
    fn a_subagent_whose_start_was_missed_counts_from_its_tools() {
        let mut sessions = Sessions::default();
        sessions.apply(&sub(1, SubagentStep::Seen { id: "a1".into(), kind: Some("Explore".into()) }));
        sessions.apply(&sub(2, SubagentStep::Seen { id: "a1".into(), kind: None }));
        assert_eq!(kinds(&sessions), [("a1".into(), Some("Explore".into()))]);
    }

    #[test]
    fn subagents_without_an_id_close_one_by_one() {
        let mut sessions = Sessions::default();
        sessions.apply(&sub(1, SubagentStep::Start { id: None, kind: None }));
        sessions.apply(&sub(2, SubagentStep::Start { id: None, kind: None }));
        sessions.apply(&sub(3, SubagentStep::Stop { id: None }));
        assert_eq!(sessions.list()[0].subagents.len(), 1);
    }

    #[test]
    fn silent_subagents_are_forgotten() {
        let mut sessions = Sessions::default();
        sessions.apply(&sub(0, SubagentStep::Start { id: Some("a1".into()), kind: None }));
        sessions.apply(&sub(SUBAGENT_QUIET_MS / 2, SubagentStep::Start { id: Some("a2".into()), kind: None }));
        assert!(sessions.tick(SUBAGENT_QUIET_MS));
        assert_eq!(kinds(&sessions), [("a2".into(), None)]);
    }

    #[test]
    fn a_turn_is_announced_once() {
        let mut sessions = Sessions::default();
        sessions.apply(&from(Source::Codex, "x1", EventKind::Working, 0));
        assert!(sessions.apply(&from(Source::Codex, "x1", EventKind::TurnDone, 1)).is_some());
        assert!(sessions.apply(&from(Source::Codex, "x1", EventKind::TurnDone, 2)).is_none(), "notify after Stop");
        sessions.apply(&from(Source::Codex, "x1", EventKind::Working, 3));
        assert!(sessions.apply(&from(Source::Codex, "x1", EventKind::TurnDone, 4)).is_some());
    }

    #[test]
    fn the_latest_change_stays_in_memory_with_its_session() {
        use crate::connectors::diff;
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        let change = diff::from_tool("Edit", &serde_json::json!({ "file_path": "/p/api.py", "old_string": "a", "new_string": "b" }), false);
        sessions.apply(&AgentEvent { diff: change.clone(), ..edit(1, 1, 1) });
        sessions.apply(&edit(2, 0, 0));
        assert_eq!(sessions.diff("claude-code:s1"), change.as_ref(), "kept until the next change");
        assert!(serde_json::to_value(&sessions.list()[0]).unwrap().get("diff").is_none(), "never sent with the list");
        let finished = sessions.apply(&event(EventKind::TurnDone, 3)).unwrap();
        assert_eq!((finished.session_id.as_str(), finished.file.as_deref()), ("claude-code:s1", Some("api.py")));
        sessions.apply(&event(EventKind::SessionEnd, 4));
        assert_eq!(sessions.diff("claude-code:s1"), None, "gone with the session");
    }

    #[test]
    fn a_question_in_the_terminal_is_only_shown() {
        let mut sessions = Sessions::default();
        sessions.apply(&event(EventKind::Working, 0));
        sessions.apply(&event(EventKind::Asking, 1));
        let session = &sessions.list()[0];
        assert_eq!((session.status, session.in_terminal), (SessionStatus::Waiting, true));
        assert_eq!(serde_json::to_value(session).unwrap()["inTerminal"], true);
        sessions.apply(&edit(2, 0, 0));
        assert_eq!((sessions.list()[0].status, sessions.list()[0].in_terminal), (SessionStatus::Active, false));
        sessions.apply(&event(EventKind::Permission, 3));
        assert!(!sessions.list()[0].in_terminal, "a request îlot shows is answered in îlot");
    }

    #[test]
    fn a_badge_and_the_terminal_follow_the_session() {
        use crate::connectors::custom_agent::{AgentColor, CustomAgent};
        let mut sessions = Sessions::default();
        let badge = CustomAgent { id: "qwen".into(), name: "Qwen Code".into(), color: AgentColor::Teal };
        sessions.apply(&AgentEvent { agent: Some(badge.clone()), terminal: Some(Host::WindowsTerminal), ..event(EventKind::Working, 0) });
        sessions.apply(&edit(1, 0, 0));
        let value = serde_json::to_value(&sessions.list()[0]).unwrap();
        assert_eq!(value["agent"], serde_json::json!({ "id": "qwen", "name": "Qwen Code", "color": "teal" }));
        assert_eq!(value["terminal"], "windows-terminal");
        assert_eq!(sessions.apply(&event(EventKind::TurnDone, 2)).unwrap().agent, Some(badge));
    }

    #[test]
    fn the_followed_session_is_the_one_that_waits_then_works() {
        let mut sessions = Sessions::default();
        sessions.apply(&from(Source::Codex, "x1", EventKind::TurnDone, 5));
        sessions.apply(&from(Source::ClaudeCode, "c1", EventKind::Working, 1));
        assert_eq!(sessions.followed().map(|s| s.id.as_str()), Some("claude-code:c1"));
        sessions.apply(&from(Source::GeminiCli, "g1", EventKind::Asking, 2));
        assert_eq!(sessions.followed().map(|s| s.id.as_str()), Some("gemini-cli:g1"));
    }

    #[test]
    fn generic_sources_keep_the_name_the_user_chose() {
        let mut sessions = Sessions::default();
        let named = |kind, at| AgentEvent { name: Some("ci".into()), ..from(Source::Generic, "ci@epilot", kind, at) };
        sessions.apply(&named(EventKind::Working, 1_000));
        assert_eq!(sessions.list()[0].name.as_deref(), Some("ci"));
        assert_eq!(serde_json::to_value(&sessions.list()[0]).unwrap()["name"], "ci");
        let finished = sessions.apply(&named(EventKind::TurnDone, 61_000)).unwrap();
        assert_eq!((finished.name.as_deref(), finished.duration_ms), (Some("ci"), 60_000));
        assert_eq!(live_counts(&sessions), [0, 0, 0, 0]);
    }
}
