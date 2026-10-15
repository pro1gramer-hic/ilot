//! Permission requests waiting for the user. Each one holds the channel back to the
//! relay that is waiting; it leaves only on the user's answer, on expiry, when the agent
//! gives up, or refused by « Tout suspendre ». Nothing here ever allows on its own.

use crate::connectors::PermissionView;
use crate::connectors::questions::Question;
use crate::events::Source;
use crate::risk::Risk;
use ilot_bridge::protocol::Reply;
use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// The relay waits 110 s (the hook's timeout is 120 s); îlot gives up at the same time.
pub const PERMISSION_TIMEOUT_MS: u64 = 110_000;
/// The interface stops taking answers this long before the end (`ANSWER_MARGIN_MS` in
/// `src/lib/stores/permissions.svelte.ts`).
pub const ANSWER_MARGIN: Duration = Duration::from_secs(5);

pub struct Pending {
    pub id: String,
    pub source: Source,
    pub session_key: String,
    pub project: String,
    pub event: String,
    pub payload: Value,
    pub view: PermissionView,
    /// Where it runs, as shown (`~/dev/ilot`).
    pub folder: String,
    pub received_at: u64,
    /// How long the relay waits for it (`ilot_bridge::protocol::wait_for`).
    pub timeout_ms: u64,
    /// For the journal: how long it waited, whatever the clock does.
    pub waiting_since: Instant,
    pub reply: oneshot::Sender<Reply>,
}

/// What the interface receives (same shape as `PermissionRequest` in TypeScript).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDto {
    pub id: String,
    pub source: Source,
    pub project: String,
    pub title: String,
    pub detail: String,
    pub content: Option<String>,
    pub extras: Vec<String>,
    pub reason: Option<String>,
    pub truncated: bool,
    pub can_always: bool,
    pub risk: Risk,
    pub warnings: Vec<String>,
    pub folder: String,
    pub received_at: u64,
    pub timeout_ms: u64,
    /// The agent's questions, when it asks instead of requesting a permission.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<Question>,
}

impl Pending {
    pub fn dto(&self) -> PermissionDto {
        PermissionDto {
            id: self.id.clone(),
            source: self.source,
            project: self.project.clone(),
            title: self.view.title.clone(),
            detail: self.view.detail.clone(),
            content: self.view.content.clone(),
            extras: self.view.extras.clone(),
            reason: self.view.reason.clone(),
            truncated: self.view.truncated,
            can_always: self.view.can_always,
            risk: self.view.risk,
            warnings: self.view.warnings.clone(),
            folder: self.folder.clone(),
            received_at: self.received_at,
            timeout_ms: self.timeout_ms,
            questions: self.view.questions.clone(),
        }
    }
}

#[derive(Default)]
pub struct PermissionQueue {
    pending: Vec<Pending>,
    /// « Tout suspendre »: nothing waits, every request is refused as it comes.
    suspended: bool,
}

impl PermissionQueue {
    /// Queues a request; while suspended, gives it back to be refused.
    pub fn add(&mut self, pending: Pending) -> Result<(), Box<Pending>> {
        if self.suspended {
            return Err(Box::new(pending));
        }
        self.pending.push(pending);
        Ok(())
    }

    /// Turns « Tout suspendre » on or off. Turning it on hands back every waiting request,
    /// to be refused: none stays answerable while suspended.
    pub fn set_suspended(&mut self, on: bool) -> Vec<Pending> {
        self.suspended = on;
        if on { std::mem::take(&mut self.pending) } else { Vec::new() }
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended
    }

    pub fn get(&self, id: &str) -> Option<&Pending> {
        self.pending.iter().find(|p| p.id == id)
    }

    /// Takes a request out of the queue (answered, expired or abandoned).
    pub fn take(&mut self, id: &str) -> Option<Pending> {
        let index = self.pending.iter().position(|p| p.id == id)?;
        Some(self.pending.remove(index))
    }

    pub fn list(&self) -> Vec<PermissionDto> {
        self.pending.iter().map(Pending::dto).collect()
    }
}

/// The home folder of the agent that asks, read from its transcript path
/// (`/home/user/.claude/projects/…`, `C:\Users\me\.codex\sessions\…`), else the Windows
/// profile for a Windows working folder.
pub fn agent_home(transcript: Option<&str>, cwd: &str, windows_profile: Option<&str>) -> Option<String> {
    let from_transcript = transcript.and_then(|path| {
        [".claude", ".codex", ".gemini"].iter().find_map(|dir| {
            let at = path.find(&format!("/{dir}/")).or_else(|| path.find(&format!("\\{dir}\\")))?;
            Some(path[..at].to_owned()).filter(|home| !home.is_empty())
        })
    });
    from_transcript.or_else(|| windows_profile.filter(|_| is_windows_path(cwd)).map(str::to_owned))
}

fn is_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/')
}

/// The working folder as the island shows it: `~/dev/ilot` under the agent's home,
/// otherwise unchanged.
pub fn display_folder(cwd: &str, home: Option<&str>) -> String {
    let Some(home) = home.map(|h| h.trim_end_matches(['/', '\\'])).filter(|h| !h.is_empty()) else {
        return cwd.to_owned();
    };
    let same = |prefix: &str| if is_windows_path(home) { prefix.eq_ignore_ascii_case(home) } else { prefix == home };
    match cwd.get(..home.len()).filter(|prefix| same(prefix)).map(|_| &cwd[home.len()..]) {
        Some("") => "~".to_owned(),
        Some(rest) if rest.starts_with(['/', '\\']) => format!("~{rest}"),
        _ => cwd.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(id: &str) -> (Pending, oneshot::Receiver<Reply>) {
        let (reply, receiver) = oneshot::channel();
        let view = PermissionView {
            title: "Claude veut exécuter une commande".into(),
            detail: "npm run build".into(),
            content: None,
            extras: Vec::new(),
            reason: None,
            truncated: false,
            can_always: true,
            risk: Risk::High,
            warnings: vec![crate::locale::tr(crate::risk::DELETE)],
            questions: Vec::new(),
        };
        let pending = Pending {
            id: id.into(),
            source: Source::ClaudeCode,
            session_key: "claude-code:s1".into(),
            project: "epilot".into(),
            event: "PermissionRequest".into(),
            payload: Value::Null,
            view,
            folder: "~/dev/epilot".into(),
            received_at: 42,
            timeout_ms: PERMISSION_TIMEOUT_MS,
            waiting_since: Instant::now(),
            reply,
        };
        (pending, receiver)
    }

    #[test]
    fn requests_leave_the_queue_once() {
        let mut queue = PermissionQueue::default();
        let (first, _r1) = pending("a");
        let (second, _r2) = pending("b");
        assert!(queue.add(first).is_ok());
        assert!(queue.add(second).is_ok());
        assert_eq!(queue.list().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert!(queue.take("a").is_some());
        assert!(queue.take("a").is_none());
        assert_eq!(queue.list().len(), 1);
    }

    #[test]
    fn the_interface_gets_the_view_and_the_deadline() {
        let (p, _r) = pending("a");
        let dto = p.dto();
        assert_eq!(dto.detail, "npm run build");
        assert_eq!(dto.timeout_ms, PERMISSION_TIMEOUT_MS);
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["canAlways"], true);
        assert_eq!(value["risk"], "high");
        assert_eq!(value["warnings"], serde_json::json!(["Supprime des fichiers de façon définitive."]));
        assert_eq!(value["folder"], "~/dev/epilot");
    }

    #[test]
    fn suspending_empties_the_queue_and_refuses_what_comes() {
        let mut queue = PermissionQueue::default();
        let (first, _r1) = pending("a");
        assert!(queue.add(first).is_ok());
        let refused: Vec<String> = queue.set_suspended(true).into_iter().map(|p| p.id).collect();
        assert_eq!(refused, ["a"]);
        assert!(queue.is_suspended());
        assert!(queue.list().is_empty());
        let (second, _r2) = pending("b");
        assert_eq!(queue.add(second).unwrap_err().id, "b");
        assert!(queue.set_suspended(false).is_empty());
        let (third, _r3) = pending("c");
        assert!(queue.add(third).is_ok());
        assert_eq!(queue.list().len(), 1);
    }

    #[test]
    fn the_folder_is_shown_from_the_home() {
        assert_eq!(display_folder("/home/user/dev/ilot", Some("/home/user")), "~/dev/ilot");
        assert_eq!(display_folder("/home/user", Some("/home/user/")), "~");
        assert_eq!(display_folder("/home/username/x", Some("/home/user")), "/home/username/x");
        assert_eq!(display_folder("/srv/app", Some("/home/user")), "/srv/app");
        assert_eq!(display_folder("/home/user/dev", None), "/home/user/dev");
        assert_eq!(display_folder(r"C:\Users\HP 240\dev\ilot", Some(r"c:\users\hp 240")), r"~\dev\ilot");
        assert_eq!(display_folder("/home/usér/a", Some("/home/us")), "/home/usér/a");
    }

    #[test]
    fn the_home_comes_from_the_agent_transcript() {
        let claude = "/home/user/.claude/projects/-home-user-projects-demo/789440dc.jsonl";
        assert_eq!(agent_home(Some(claude), "/home/user/projects/demo", None).as_deref(), Some("/home/user"));
        let codex = "/home/user/.codex/sessions/2026/10/11/rollout.jsonl";
        assert_eq!(agent_home(Some(codex), "/x", Some(r"C:\Users\me")).as_deref(), Some("/home/user"));
        let windows = r"C:\Users\me\.claude\projects\x.jsonl";
        assert_eq!(agent_home(Some(windows), r"C:\Users\me\dev", None).as_deref(), Some(r"C:\Users\me"));
        assert_eq!(agent_home(None, r"C:\Users\me\dev", Some(r"C:\Users\me")).as_deref(), Some(r"C:\Users\me"));
        assert_eq!(agent_home(None, "/home/user/dev", Some(r"C:\Users\me")), None, "no guess for a Linux folder");
    }

    #[test]
    fn the_answer_reaches_the_waiting_relay() {
        let (p, mut receiver) = pending("a");
        p.reply.send(Reply { output: Some("ok".into()) }).unwrap();
        assert_eq!(receiver.try_recv().unwrap().output.as_deref(), Some("ok"));
    }
}
