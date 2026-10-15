//! Where events from the relays meet the island: connectors translate, sessions update,
//! permissions wait for the user, and the interface is told about every change. Every
//! request ends in the decisions journal, kept in memory only.

use crate::connectors::diff::FileDiff;
use crate::connectors::questions::QuestionAnswer;
use crate::connectors::{self, Answer, custom_agent, cursor};
use crate::decisions::{self, Decision, DecisionKind, Journal};
use crate::events::{Source, now_ms};
use crate::permissions::{self, PERMISSION_TIMEOUT_MS, Pending, PermissionQueue};
use crate::pipe_server::{Handler, Outcome};
use crate::sessions::{Sessions, session_key};
use crate::stats::Stats;
use crate::terminal::Host;
use ilot_bridge::protocol::{Reply, Request, wait_for};
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;
use windows::Win32::Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom};

pub const EVENT_SESSIONS: &str = "sessions:changed";
pub const EVENT_FINISHED: &str = "agent:finished";
pub const EVENT_PERMISSION_ADDED: &str = "permission:added";
pub const EVENT_PERMISSION_REMOVED: &str = "permission:removed";
pub const EVENT_SUSPENDED: &str = "agents:suspended";
pub const EVENT_DECISION: &str = "decision:added";
/// A session has a new latest change (only its id: the change is fetched on demand).
pub const EVENT_DIFF: &str = "diff:changed";
/// Cursor conversations remembered as speaking through Cursor's own hooks.
const MAX_CURSOR_NATIVE: usize = 256;

/// Something the interface must hear about.
pub trait Notify: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

/// Commands, prompts and permissions only reach the island's window: the mini-player and
/// the YouTube window never hear about agents.
impl Notify for AppHandle {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = Emitter::emit_to(self, "main", event, payload);
    }
}

pub struct Core<N: Notify> {
    notify: N,
    sessions: Mutex<Sessions>,
    permissions: Mutex<PermissionQueue>,
    journal: Mutex<Journal>,
    /// Counters of the Monday report (numbers and project names only).
    stats: Stats,
    /// The Windows profile, to show Windows folders as `~\…`.
    windows_profile: Option<String>,
    /// Cursor conversations its own hooks speak for: what Claude Code's hooks say about
    /// them under Cursor is the same thing told twice, and is dropped.
    cursor_native: Mutex<HashSet<String>>,
}

impl<N: Notify> Core<N> {
    /// A core whose report counters stay in memory (tests).
    #[cfg(test)]
    pub fn new(notify: N) -> Self {
        Self::with_stats(notify, Stats::in_memory())
    }

    pub fn with_stats(notify: N, stats: Stats) -> Self {
        Self {
            notify,
            sessions: Mutex::default(),
            permissions: Mutex::default(),
            journal: Mutex::default(),
            stats,
            windows_profile: std::env::var("USERPROFILE").ok(),
            cursor_native: Mutex::default(),
        }
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// The latest change of a session, from its hooks (memory only).
    pub fn session_diff(&self, id: &str) -> Option<FileDiff> {
        self.sessions.lock().unwrap().diff(id).cloned()
    }

    /// Where a session runs and its project, for « Aller au terminal »: the session named,
    /// or the one that matters most now.
    pub fn terminal_of(&self, id: Option<&str>) -> Option<(Option<Host>, String)> {
        let sessions = self.sessions.lock().unwrap();
        let session = match id {
            Some(id) => sessions.get(id),
            None => sessions.followed(),
        }?;
        Some((session.terminal, session.project.clone()))
    }

    /// The user's answers to an agent's questions. Nothing is sent unless the request is
    /// a question shown in full and every answer fits its question; the request then
    /// leaves, and the agent goes on with the user's answers.
    pub fn answer_questions(&self, id: &str, answers: &[QuestionAnswer]) -> bool {
        let pending = {
            let mut queue = self.permissions.lock().unwrap();
            let Some(asked) = queue.get(id).filter(|p| !p.view.questions.is_empty() && !p.view.truncated) else { return false };
            let output = connectors::find(asked.source.id()).and_then(|c| c.answer_questions(&asked.event, &asked.payload, answers));
            let Some(output) = output else { return false };
            queue.take(id).map(|pending| (pending, output))
        };
        let Some((pending, output)) = pending else { return false };
        self.notify.emit(EVENT_PERMISSION_REMOVED, serde_json::json!({ "id": id }));
        let _ = pending.reply.send(Reply { output: Some(output) });
        self.sessions.lock().unwrap().resume(&pending.session_key);
        self.publish_sessions();
        true
    }

    /// True for an event Claude Code's hooks told about a Cursor conversation that Cursor's
    /// own hooks already speak for.
    fn told_twice(&self, source: Source, request: &Request) -> bool {
        if source != Source::Cursor {
            return false;
        }
        let Some(conversation) = cursor::conversation(&request.payload) else { return false };
        let mut native = self.cursor_native.lock().unwrap();
        if cursor::imported(&request.event) {
            return native.contains(conversation);
        }
        if native.len() >= MAX_CURSOR_NATIVE {
            native.clear();
        }
        native.insert(conversation.to_owned());
        false
    }

    fn publish_sessions(&self) {
        let list = serde_json::to_value(self.sessions.lock().unwrap().list()).unwrap_or_default();
        self.notify.emit(EVENT_SESSIONS, list);
    }

    pub fn sessions(&self) -> serde_json::Value {
        serde_json::to_value(self.sessions.lock().unwrap().list()).unwrap_or_default()
    }

    pub fn permissions(&self) -> serde_json::Value {
        serde_json::to_value(self.permissions.lock().unwrap().list()).unwrap_or_default()
    }

    pub fn suspended(&self) -> bool {
        self.permissions.lock().unwrap().is_suspended()
    }

    /// The decisions of this run, newest first.
    pub fn decisions(&self) -> Vec<Decision> {
        self.journal.lock().unwrap().list()
    }

    /// The user's answer. The relay gets the agent's exact expected output, or nothing
    /// when the user answers in the terminal. A request cut by the relay is never allowed
    /// from îlot (the user has not seen all of it), and "Toujours" only where it was
    /// offered, whatever the interface sends. « Tout suspendre » is not an answer.
    pub fn answer(&self, id: &str, answer: Answer) -> bool {
        if answer == Answer::Suspended {
            return false;
        }
        let pending = {
            let mut queue = self.permissions.lock().unwrap();
            // A question is answered with its answers (`answer_questions`), or left to the
            // terminal: « Autoriser » or « Refuser » mean nothing for it.
            let question = |p: &Pending| !p.view.questions.is_empty() && answer != Answer::Terminal;
            let refused = |p: &Pending| {
                (answer.allows() && p.view.truncated) || (answer == Answer::Always && !p.view.can_always) || question(p)
            };
            if queue.get(id).is_some_and(refused) {
                return false;
            }
            queue.take(id)
        };
        let Some(pending) = pending else { return false };
        self.notify.emit(EVENT_PERMISSION_REMOVED, serde_json::json!({ "id": id }));
        self.settle(pending, answer);
        self.publish_sessions();
        true
    }

    /// « Tout suspendre »: on, the waiting requests are refused and so is every new one
    /// as it comes, until it is turned off. Nothing is ever allowed by it. Agents are not
    /// paused (îlot has no way to): those that ask nothing keep working.
    pub fn suspend(&self, on: bool) {
        let refused = self.permissions.lock().unwrap().set_suspended(on);
        let any = !refused.is_empty();
        for pending in refused {
            self.notify.emit(EVENT_PERMISSION_REMOVED, serde_json::json!({ "id": pending.id }));
            self.settle(pending, Answer::Suspended);
        }
        if any {
            self.publish_sessions();
        }
        self.notify.emit(EVENT_SUSPENDED, serde_json::json!({ "on": on }));
    }

    /// Sends the answer to the waiting relay, notes the decision, and lets the agent go on.
    /// A question is no permission: it never goes to the decisions journal, and only its
    /// answers (`answer_questions`) send anything; any other way out leaves it to the
    /// terminal.
    fn settle(&self, pending: Pending, answer: Answer) {
        let question = !pending.view.questions.is_empty();
        let output = match answer {
            Answer::Terminal => None,
            _ if question => None,
            _ => connectors::find(pending.source.id()).and_then(|c| c.answer(&pending.event, &pending.payload, answer)),
        };
        if !question {
            self.record(&pending, answer.into());
        }
        let _ = pending.reply.send(Reply { output });
        self.sessions.lock().unwrap().resume(&pending.session_key);
    }

    fn record(&self, pending: &Pending, kind: DecisionKind) {
        let decision = Decision::of(pending, kind, now_ms());
        self.stats.decision(kind, decision.at);
        self.notify.emit(EVENT_DECISION, serde_json::to_value(&decision).unwrap_or_default());
        self.journal.lock().unwrap().add(decision);
    }

    /// Pauses quiet sessions and forgets old ones, and saves the report's counters
    /// (called every minute).
    pub fn tick(&self) {
        let now = now_ms();
        if self.sessions.lock().unwrap().tick(now) {
            self.publish_sessions();
        }
        if let Err(error) = self.stats.flush(now) {
            eprintln!("îlot: report counters not saved: {error}");
        }
    }
}

impl<N: Notify> Handler for Core<N> {
    fn handle(&self, mut request: Request) -> Outcome {
        let (Some(source), Some(connector)) = (Source::from_id(&request.source), connectors::find(&request.source)) else {
            return Outcome::Reply(Reply::default());
        };
        if self.told_twice(source, &request) {
            return Outcome::Reply(Reply::default());
        }
        let env = request.env.take().map(|env| env.checked()).unwrap_or_default();
        // IBM Bob sends no working folder: the relay's own is the task's.
        if let (Some(cwd), Some(payload)) = (&env.cwd, request.payload.as_object_mut())
            && !payload.contains_key("cwd")
        {
            payload.insert("cwd".into(), cwd.clone().into());
        }
        let at = now_ms();
        if let Some(mut event) = connector.parse(&request.event, &request.payload, at) {
            event.agent = custom_agent::read(&request.payload);
            event.terminal = Host::from_env(&env, source);
            self.stats.event(&event);
            let changed = event.diff.is_some();
            let finished = self.sessions.lock().unwrap().apply(&event);
            self.publish_sessions();
            if changed {
                self.notify.emit(EVENT_DIFF, serde_json::json!({ "id": session_key(source, &event.session_id) }));
            }
            if let Some(finished) = finished {
                self.notify.emit(EVENT_FINISHED, serde_json::to_value(finished).unwrap_or_default());
            }
        }
        if !request.wait {
            return Outcome::Reply(Reply::default());
        }
        let Some(view) = connector.permission(&request.event, &request.payload) else {
            return Outcome::Reply(Reply::default());
        };
        let session_id = connectors::text(&request.payload, "session_id").unwrap_or_default();
        let cwd = connectors::text(&request.payload, "cwd").unwrap_or_default();
        let transcript = connectors::text(&request.payload, "transcript_path");
        let home = permissions::agent_home(transcript, cwd, self.windows_profile.as_deref());
        let (reply, answer) = oneshot::channel();
        // The relay's wait for this event (a question waits less than a permission); the
        // pipe server gives up at the same time.
        let timeout_ms = wait_for(&request.source, &request.event).map_or(PERMISSION_TIMEOUT_MS, |wait| wait.as_millis() as u64);
        let pending = Pending {
            id: random_hex(16),
            source,
            session_key: session_key(source, session_id),
            project: crate::events::project_name(cwd),
            folder: permissions::display_folder(cwd, home.as_deref()),
            event: request.event,
            payload: request.payload,
            view,
            received_at: at,
            timeout_ms,
            waiting_since: Instant::now(),
            reply,
        };
        let id = pending.id.clone();
        let dto = serde_json::to_value(pending.dto()).unwrap_or_default();
        // Checked, queued and announced under one lock: no request slips in while « Tout
        // suspendre » starts, and no answer or suspension overtakes the announcement.
        let refused = {
            let mut queue = self.permissions.lock().unwrap();
            match queue.add(pending) {
                Ok(()) => {
                    self.notify.emit(EVENT_PERMISSION_ADDED, dto);
                    None
                }
                Err(refused) => Some(refused),
            }
        };
        // Suspended: refused as it comes, never shown.
        if let Some(refused) = refused {
            self.settle(*refused, Answer::Suspended);
            self.publish_sessions();
        }
        Outcome::Wait { id, answer }
    }

    /// The relay stopped waiting: the agent gave up (most likely answered in its terminal)
    /// or the time ran out.
    fn cancel(&self, id: &str) {
        let Some(pending) = self.permissions.lock().unwrap().take(id) else { return };
        self.notify.emit(EVENT_PERMISSION_REMOVED, serde_json::json!({ "id": id }));
        if pending.view.questions.is_empty() {
            self.record(&pending, decisions::ended_without_answer(pending.waiting_since.elapsed()));
        }
    }
}

/// The relay's wait and îlot's wait end together.
pub const PERMISSION_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(PERMISSION_TIMEOUT_MS);

/// Random bytes from the system generator, as hex.
pub fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    // SAFETY: the buffer is valid for its whole length; no algorithm handle is needed
    // with the system-preferred generator.
    let status = unsafe { BCryptGenRandom(None, &mut buffer, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    assert!(status.is_ok(), "system random generator failed");
    buffer.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ilot_bridge::client;
    use ilot_bridge::protocol::RelayEnv;
    use serde_json::{Value, json};

    #[derive(Default)]
    struct Recorder(Mutex<Vec<(String, Value)>>);

    impl Notify for std::sync::Arc<Recorder> {
        fn emit(&self, event: &str, payload: Value) {
            self.0.lock().unwrap().push((event.to_owned(), payload));
        }
    }

    fn fixture_of(source: &str, event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/{source}/{event}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn fixture(event: &str) -> Value {
        fixture_of("claude-code", event)
    }

    fn core() -> (Core<std::sync::Arc<Recorder>>, std::sync::Arc<Recorder>) {
        let recorder = std::sync::Arc::new(Recorder::default());
        (Core::new(recorder.clone()), recorder)
    }

    fn send(core: &Core<std::sync::Arc<Recorder>>, event: &str, wait: bool) -> Outcome {
        core.handle(client::request("t", "claude-code", event, fixture(event), wait))
    }

    fn emitted(recorder: &Recorder, name: &str) -> Vec<Value> {
        recorder.0.lock().unwrap().iter().filter(|(e, _)| e == name).map(|(_, p)| p.clone()).collect()
    }

    #[test]
    fn real_events_build_a_session_and_announce_the_end() {
        let (core, recorder) = core();
        for event in ["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "Stop"] {
            assert!(matches!(send(&core, event, false), Outcome::Reply(_)));
        }
        let sessions = core.sessions();
        assert_eq!(sessions[0]["status"], "done");
        assert_eq!(sessions[0]["project"], "demo");
        assert_eq!(emitted(&recorder, EVENT_FINISHED).len(), 1);
    }

    #[test]
    fn a_permission_waits_and_the_answer_reaches_the_relay() {
        let (core, recorder) = core();
        let Outcome::Wait { id, mut answer } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert_eq!(emitted(&recorder, EVENT_PERMISSION_ADDED)[0]["detail"], "python3 -c \"print('allowed-by-ilot')\"");
        assert_eq!(core.sessions()[0]["status"], "waiting");
        assert!(core.answer(&id, Answer::Deny));
        let reply = answer.try_recv().unwrap();
        assert!(reply.output.unwrap().contains("\"deny\""));
        assert_eq!(core.sessions()[0]["status"], "active");
        assert!(!core.answer(&id, Answer::Allow), "a request is answered once");
    }

    #[test]
    fn a_cut_request_is_never_allowed_from_ilot() {
        let (core, _) = core();
        let mut payload = fixture("PermissionRequest");
        payload[ilot_bridge::protocol::TRUNCATED_FLAG] = json!(true);
        let request = client::request("t", "claude-code", "PermissionRequest", payload, true);
        let Outcome::Wait { id, mut answer } = core.handle(request) else { panic!("should wait") };
        assert!(!core.answer(&id, Answer::Allow));
        assert!(!core.answer(&id, Answer::Always));
        assert!(answer.try_recv().is_err(), "still waiting");
        assert!(core.answer(&id, Answer::Terminal));
        assert_eq!(answer.try_recv().unwrap().output, None, "the terminal asks");
    }

    #[test]
    fn toujours_is_refused_where_it_was_not_offered() {
        let (core, _) = core();
        let Outcome::Wait { id, mut answer } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(!core.answer(&id, Answer::Always), "python3 -c: no rule offered");
        assert!(answer.try_recv().is_err(), "still waiting");
        assert!(core.answer(&id, Answer::Allow));
    }

    #[test]
    fn answering_in_the_terminal_sends_nothing() {
        let (core, recorder) = core();
        let Outcome::Wait { id, mut answer } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(core.answer(&id, Answer::Terminal));
        assert_eq!(answer.try_recv().unwrap().output, None);
        assert_eq!(emitted(&recorder, EVENT_PERMISSION_REMOVED).len(), 1);
        assert_eq!(core.permissions(), json!([]));
    }

    #[test]
    fn a_cancelled_permission_disappears_without_an_answer() {
        let (core, recorder) = core();
        let Outcome::Wait { id, .. } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        core.cancel(&id);
        assert_eq!(emitted(&recorder, EVENT_PERMISSION_REMOVED).len(), 1);
        assert_eq!(core.permissions(), json!([]));
    }

    #[test]
    fn unknown_sources_and_non_waiting_permissions_are_only_acknowledged() {
        let (core, _) = core();
        assert!(matches!(core.handle(client::request("t", "chatgpt", "Stop", json!({}), true)), Outcome::Reply(_)));
        assert!(matches!(send(&core, "PermissionRequest", false), Outcome::Reply(_)));
        assert_eq!(core.permissions(), json!([]));
    }

    #[test]
    fn a_codex_permission_gets_codex_answers_without_toujours() {
        let (core, recorder) = core();
        let request = client::request("t", "codex", "PermissionRequest", fixture_of("codex", "PermissionRequest"), true);
        let Outcome::Wait { id, mut answer } = core.handle(request) else { panic!("should wait") };
        let added = &emitted(&recorder, EVENT_PERMISSION_ADDED)[0];
        assert_eq!(added["source"], "codex");
        assert_eq!(added["canAlways"], false);
        assert_eq!(core.sessions()[0]["status"], "waiting");
        assert!(core.answer(&id, Answer::Allow));
        let output: Value = serde_json::from_str(&answer.try_recv().unwrap().output.unwrap()).unwrap();
        assert_eq!(output["hookSpecificOutput"]["decision"]["behavior"], "allow");
    }

    #[test]
    fn gemini_and_generic_events_only_update_sessions() {
        let (core, recorder) = core();
        let notification = fixture_of("gemini-cli", "Notification");
        assert!(matches!(core.handle(client::request("t", "gemini-cli", "Notification", notification, false)), Outcome::Reply(_)));
        assert_eq!(core.sessions()[0]["status"], "waiting");
        assert_eq!(core.permissions(), json!([]), "îlot cannot answer Gemini: no permission to show");
        let generic = json!({ "name": "ci", "session_id": "ci@site", "cwd": "/home/user/site", "title": "build" });
        core.handle(client::request("t", "generic", "working", generic.clone(), false));
        core.handle(client::request("t", "generic", "turn_done", generic, false));
        let finished = emitted(&recorder, EVENT_FINISHED);
        assert_eq!((finished[0]["name"].as_str(), finished[0]["source"].as_str()), (Some("ci"), Some("generic")));
    }

    #[test]
    fn every_answer_is_noted_in_the_journal() {
        let (core, recorder) = core();
        let Outcome::Wait { id, .. } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(core.answer(&id, Answer::Allow));
        let Outcome::Wait { id, .. } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(core.answer(&id, Answer::Terminal));
        let Outcome::Wait { id, .. } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        core.cancel(&id);
        let kinds: Vec<_> = core.decisions().iter().map(|d| d.decision).collect();
        assert_eq!(kinds, [DecisionKind::Terminal, DecisionKind::Terminal, DecisionKind::Allow], "newest first");
        let added = emitted(&recorder, EVENT_DECISION);
        assert_eq!(added.len(), 3);
        assert_eq!(added[0]["decision"], "allow");
        assert_eq!(added[0]["source"], "claude-code");
        assert_eq!(added[0]["project"], "demo");
        assert_eq!(added[0]["command"], "python3 -c \"print('allowed-by-ilot')\"");
        assert!(added[0]["at"].as_u64().unwrap() > 0);
    }

    #[test]
    fn an_allow_is_noted_only_once_the_user_gave_it() {
        let (core, _) = core();
        let Outcome::Wait { id, .. } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(!core.answer(&id, Answer::Always), "not offered");
        assert!(!core.answer("unknown", Answer::Allow));
        assert!(!core.answer(&id, Answer::Suspended), "the interface cannot suspend through an answer");
        assert!(core.decisions().is_empty());
        assert_eq!(core.permissions().as_array().unwrap().len(), 1, "still waiting");
    }

    #[test]
    fn tout_suspendre_refuses_the_waiting_request_and_every_new_one() {
        let (core, recorder) = core();
        let Outcome::Wait { id, mut answer } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        core.suspend(true);
        assert!(core.suspended());
        let output: Value = serde_json::from_str(&answer.try_recv().unwrap().output.unwrap()).unwrap();
        let decision = &output["hookSpecificOutput"]["decision"];
        assert_eq!((decision["behavior"].as_str(), decision["message"].as_str()), (Some("deny"), Some("Refusé : îlot a tout suspendu.")));
        assert_eq!(core.permissions(), json!([]));
        assert_eq!(emitted(&recorder, EVENT_PERMISSION_REMOVED)[0]["id"], id.as_str());
        assert!(!core.answer(&id, Answer::Allow), "nothing left to allow");

        // A new request is refused as it comes: never shown, never allowed.
        let Outcome::Wait { mut answer, .. } = send(&core, "PermissionRequest", true) else { panic!("answered through the relay") };
        assert!(answer.try_recv().unwrap().output.unwrap().contains("îlot a tout suspendu"));
        assert_eq!(emitted(&recorder, EVENT_PERMISSION_ADDED).len(), 1, "only the first one was shown");
        assert_eq!(emitted(&recorder, EVENT_PERMISSION_REMOVED).len(), 1);
        assert_eq!(core.sessions()[0]["status"], "active", "the agent is not left waiting");
        let kinds: Vec<_> = core.decisions().iter().map(|d| d.decision).collect();
        assert_eq!(kinds, [DecisionKind::Suspended, DecisionKind::Suspended]);
        assert_eq!(emitted(&recorder, EVENT_SUSPENDED), [json!({ "on": true })]);

        core.suspend(false);
        assert!(!core.suspended());
        assert_eq!(emitted(&recorder, EVENT_SUSPENDED).last(), Some(&json!({ "on": false })));
        let Outcome::Wait { id, mut answer } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(answer.try_recv().is_err(), "waits for the user again");
        assert!(core.answer(&id, Answer::Deny));
    }

    #[test]
    fn a_codex_request_is_refused_by_suspension_too() {
        let (core, _) = core();
        core.suspend(true);
        let request = client::request("t", "codex", "PermissionRequest", fixture_of("codex", "PermissionRequest"), true);
        let Outcome::Wait { mut answer, .. } = core.handle(request) else { panic!("answered through the relay") };
        let output: Value = serde_json::from_str(&answer.try_recv().unwrap().output.unwrap()).unwrap();
        assert_eq!(output["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(core.decisions()[0].source, Source::Codex);
    }

    #[test]
    fn requests_carry_their_risk_and_folder() {
        let (core, recorder) = core();
        let mut payload = fixture("PermissionRequest");
        payload["tool_input"]["command"] = json!("git push --force origin main");
        core.handle(client::request("t", "claude-code", "PermissionRequest", payload, true));
        let added = &emitted(&recorder, EVENT_PERMISSION_ADDED)[0];
        assert_eq!(added["risk"], "high");
        assert_eq!(added["warnings"], json!(["Pousse sur GitHub en forçant : l'historique distant sera écrasé."]));
        assert_eq!(added["folder"], "~/projects/demo", "home read from the transcript path");
        let codex = client::request("t", "codex", "PermissionRequest", fixture_of("codex", "PermissionRequest"), true);
        core.handle(codex);
        let added = &emitted(&recorder, EVENT_PERMISSION_ADDED)[1];
        assert_eq!((added["risk"].as_str(), added["folder"].as_str()), (Some("low"), Some("~/projects/demo")));
    }

    fn answers(picked: &[&[&str]]) -> Vec<QuestionAnswer> {
        picked.iter().map(|labels| QuestionAnswer { picked: labels.iter().map(|l| (*l).to_owned()).collect(), other: None }).collect()
    }

    #[test]
    fn a_question_waits_for_the_user_s_answers_only() {
        let (core, recorder) = core();
        let Outcome::Wait { id, mut answer } = send(&core, "AskUserQuestion", true) else { panic!("should wait") };
        let added = &emitted(&recorder, EVENT_PERMISSION_ADDED)[0];
        assert_eq!(added["questions"].as_array().unwrap().len(), 2);
        assert_eq!(added["timeoutMs"], 55_000, "a question waits less than a permission");
        assert_eq!(core.sessions()[0]["status"], "waiting");
        // « Autoriser », « Toujours » or « Refuser » mean nothing for a question.
        for refused in [Answer::Allow, Answer::Always, Answer::Deny] {
            assert!(!core.answer(&id, refused), "{refused:?}");
        }
        assert!(!core.answer_questions(&id, &answers(&[&["Vue"]])), "every question needs its answer");
        assert!(answer.try_recv().is_err(), "still waiting");
        assert!(core.answer_questions(&id, &answers(&[&["Vue"], &["Tests"]])));
        let output: Value = serde_json::from_str(&answer.try_recv().unwrap().output.unwrap()).unwrap();
        assert_eq!(output["hookSpecificOutput"]["updatedInput"]["answers"]["Which framework?"], "Vue");
        assert_eq!(core.sessions()[0]["status"], "active");
        assert!(core.decisions().is_empty(), "a question is no permission");
        assert!(!core.answer_questions(&id, &answers(&[&["Vue"], &["Tests"]])), "answered once");
    }

    #[test]
    fn a_question_goes_back_to_the_terminal() {
        let (core, _) = core();
        let Outcome::Wait { id, mut answer } = send(&core, "AskUserQuestion", true) else { panic!("should wait") };
        assert!(core.answer(&id, Answer::Terminal));
        assert_eq!(answer.try_recv().unwrap().output, None, "nothing printed: Claude asks in its terminal");
        // Suspended: a question is not refused but left to the terminal.
        core.suspend(true);
        let Outcome::Wait { mut answer, .. } = send(&core, "AskUserQuestion", true) else { panic!("answered through the relay") };
        assert_eq!(answer.try_recv().unwrap().output, None);
        assert!(core.decisions().is_empty());
        // Expired: gone without a decision either.
        core.suspend(false);
        let Outcome::Wait { id, .. } = send(&core, "AskUserQuestion", true) else { panic!("should wait") };
        core.cancel(&id);
        assert!(core.decisions().is_empty());
        assert_eq!(core.permissions(), json!([]));
    }

    #[test]
    fn a_badge_from_ilot_agent_reaches_the_session() {
        let (core, _) = core();
        let mut payload = fixture("UserPromptSubmit");
        payload["ilot_agent"] = json!({ "id": "qwen", "name": "Qwen Code", "color": "#3ccfb4" });
        core.handle(client::request("t", "claude-code", "UserPromptSubmit", payload, false));
        assert_eq!(core.sessions()[0]["agent"], json!({ "id": "qwen", "name": "Qwen Code", "color": "teal" }));
    }

    #[test]
    fn the_relay_s_environment_says_where_the_session_runs() {
        let (core, _) = core();
        let mut request = client::request("t", "claude-code", "UserPromptSubmit", fixture("UserPromptSubmit"), false);
        request.env = Some(RelayEnv { wt_session: Some("8c2a1f0e-6b1d-4c1e-9b0a-1f2e3d4c5b6a".into()), ..RelayEnv::default() });
        core.handle(request);
        assert_eq!(core.sessions()[0]["terminal"], "windows-terminal");
        let id = core.sessions()[0]["id"].as_str().unwrap().to_owned();
        assert_eq!(core.terminal_of(Some(&id)), Some((Some(Host::WindowsTerminal), "demo".to_owned())));
        assert_eq!(core.terminal_of(None).map(|(host, _)| host), Some(Some(Host::WindowsTerminal)), "the followed session");
        assert_eq!(core.terminal_of(Some("claude-code:nope")), None);
        // A value of the wrong shape is dropped, not trusted.
        let mut odd = client::request("t", "codex", "UserPromptSubmit", fixture_of("codex", "UserPromptSubmit"), false);
        odd.env = Some(RelayEnv { term_program: Some("vscode; calc.exe".into()), ..RelayEnv::default() });
        core.handle(odd);
        let codex = core.sessions().as_array().unwrap().iter().find(|s| s["source"] == "codex").cloned().unwrap();
        assert!(codex.get("terminal").is_none());
    }

    #[test]
    fn a_bob_event_gets_the_relay_s_folder() {
        let (core, _) = core();
        let mut request = client::request("t", "bob", "UserPromptSubmit", fixture_of("bob", "UserPromptSubmit"), false);
        request.env = Some(RelayEnv { cwd: Some("/home/user/projects/billing".into()), ..RelayEnv::default() });
        core.handle(request);
        assert_eq!(core.sessions()[0]["project"], "billing");
        assert_eq!(core.sessions()[0]["source"], "bob");
    }

    #[test]
    fn an_edit_keeps_its_diff_for_the_live_view() {
        let (core, recorder) = core();
        let edit = json!({ "session_id": "s1", "cwd": "/p/demo", "tool_name": "Edit", "tool_input": { "file_path": "/p/demo/api.py", "old_string": "a", "new_string": "b" } });
        core.handle(client::request("t", "claude-code", "PostToolUse", edit, false));
        assert_eq!(emitted(&recorder, EVENT_DIFF), [json!({ "id": "claude-code:s1" })]);
        let diff = core.session_diff("claude-code:s1").unwrap();
        assert_eq!((diff.file.as_str(), diff.added, diff.removed), ("api.py", 1, 1));
        assert!(!core.sessions().to_string().contains("\"lines\""), "the diff stays out of the list");
        assert!(core.session_diff("claude-code:other").is_none());
    }

    #[test]
    fn claude_hooks_under_cursor_are_not_told_twice() {
        let (core, _) = core();
        let native = fixture_of("cursor", "beforeSubmitPrompt");
        let mut imported = fixture("UserPromptSubmit");
        imported["session_id"] = native["conversation_id"].clone();
        // Before Cursor's own hooks spoke, Claude's say what Cursor does.
        core.handle(client::request("t", "cursor", "UserPromptSubmit", imported.clone(), false));
        assert_eq!(core.sessions()[0]["source"], "cursor");
        core.handle(client::request("t", "cursor", "beforeSubmitPrompt", native, false));
        let mut edit = fixture_of("cursor", "afterFileEdit");
        core.handle(client::request("t", "cursor", "afterFileEdit", edit.clone(), false));
        // Claude's hook for the same edit is not counted again.
        edit["tool_name"] = json!("Write");
        edit["session_id"] = imported["session_id"].clone();
        edit["tool_input"] = json!({ "file_path": "/home/user/projects/demo/api.py", "content": "x\ny\nz" });
        core.handle(client::request("t", "cursor", "PostToolUse", edit, false));
        assert_eq!(core.sessions()[0]["activity"]["added"], 2);
        assert_eq!(core.sessions().as_array().unwrap().len(), 1);
    }

    #[test]
    fn followed_agents_never_wait_for_an_answer() {
        let (core, recorder) = core();
        let cases = [("copilot", "Notification"), ("kiro", "Stop"), ("bob", "PostToolUse"), ("cursor", "stop")];
        for (source, event) in cases {
            let payload = fixture_of(source, event);
            assert!(matches!(core.handle(client::request("t", source, event, payload, true)), Outcome::Reply(_)), "{source}");
        }
        assert!(emitted(&recorder, EVENT_PERMISSION_ADDED).is_empty());
        assert_eq!(core.permissions(), json!([]));
    }

    #[test]
    fn the_week_report_counts_real_events_and_answers() {
        let (core, _) = core();
        for event in ["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "Stop"] {
            send(&core, event, false);
        }
        let Outcome::Wait { id, .. } = send(&core, "PermissionRequest", true) else { panic!("should wait") };
        assert!(core.answer(&id, Answer::Deny));
        let report = core.stats().report(crate::stats::Which::Current, now_ms()).unwrap();
        // The request comes from another session of the fixtures: two sessions.
        assert_eq!((report.total.sessions, report.total.denied, report.total.allowed), (2, 1, 0));
    }

    #[test]
    fn random_tokens_differ() {
        let (a, b) = (random_hex(32), random_hex(32));
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }
}
