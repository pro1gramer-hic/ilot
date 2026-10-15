//! Counters of the Monday report (« Bilan du lundi », prompt 12, lot 5): for each week, the
//! time agents worked, their sessions, the files they touched, the lines they added and
//! removed, the commands they ran, the permissions allowed and refused, by day, by AI and
//! by project. Only numbers, AI ids, dates and project names (the last folder of the
//! working directory) reach `%LOCALAPPDATA%\ilot\stats.json`: never a command, a prompt,
//! a path, a file name or a session id (those stay in memory while îlot runs). Written
//! atomically, at most once a minute; erased by the user from the report.

use crate::atomic::write_atomic;
use crate::decisions::DecisionKind;
use crate::events::{Action, AgentEvent, EventKind, Source};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Weeks kept: the current one and four before it.
pub const MAX_WEEKS: usize = 5;
/// Between two events of a turn, at most this much counts as work: a long answer being
/// written counts, an agent that stopped without saying so does not count for hours.
pub const GAP_CAP_MS: u64 = 15 * 60 * 1000;
/// Projects counted per week; past it, the least busy one leaves.
pub const MAX_PROJECTS: usize = 32;
/// Characters kept of a project name.
const PROJECT_CHARS: usize = 64;
/// A session silent this long leaves the memory (a new event counts it again).
const FORGET_SESSION_MS: u64 = 3 * 60 * 60 * 1000;
const DAY_MS: i64 = 86_400_000;
/// From 1601-01-01 (FILETIME) to 1970-01-01, in milliseconds.
const FILETIME_EPOCH_MS: u64 = 11_644_473_600_000;

/// One day's counters.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Day {
    /// Time agents spent in a turn, gaps over 5 min left out.
    pub active_ms: u64,
    pub sessions: u32,
    /// Files edited or written (each one once per session).
    pub files: u32,
    pub added: u64,
    pub removed: u64,
    pub commands: u32,
    /// « Autoriser » and « Toujours ».
    pub allowed: u32,
    /// « Refuser » and « Tout suspendre ».
    pub denied: u32,
}

impl Day {
    fn add(&mut self, other: &Day) {
        self.active_ms += other.active_ms;
        self.sessions += other.sessions;
        self.files += other.files;
        self.added += other.added;
        self.removed += other.removed;
        self.commands += other.commands;
        self.allowed += other.allowed;
        self.denied += other.denied;
    }
}

/// The longest session of the week, by working time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Longest {
    pub ms: u64,
    pub source: Source,
    pub project: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Week {
    /// Its Monday, a local date ("2026-10-05").
    pub monday: String,
    /// Monday first.
    pub days: [Day; 7],
    /// Working time per AI id ("claude-code").
    pub agents: BTreeMap<String, u64>,
    /// Working time per project name.
    pub projects: BTreeMap<String, u64>,
    pub longest: Option<Longest>,
}

/// What `stats.json` holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatsFile {
    pub weeks: Vec<Week>,
    /// The Monday whose report was last offered on its own.
    pub offered: Option<String>,
}

/// A session as îlot follows it while it runs (memory only).
struct Live {
    in_turn: bool,
    last_at: u64,
    /// Working time this week.
    worked_ms: u64,
    monday: i64,
    files: HashSet<String>,
}

#[derive(Default)]
struct Inner {
    file: StatsFile,
    live: HashMap<String, Live>,
    dirty: bool,
}

/// One AI or one project and its working time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub name: String,
    pub ms: u64,
}

/// A week as the report shows it (type `WeekReport` in src/lib/report.ts).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub monday: String,
    pub days: Vec<Day>,
    pub total: Day,
    /// Busiest first.
    pub agents: Vec<Share>,
    /// Busiest first, five at most.
    pub projects: Vec<Share>,
    /// Index of the busiest day (Monday 0), none when no agent worked.
    pub busiest_day: Option<usize>,
    pub longest: Option<Longest>,
}

/// Which week the report shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Which {
    Last,
    Current,
}

pub struct Stats {
    path: Option<PathBuf>,
    /// Local day (days since 1970-01-01) of an instant in epoch milliseconds.
    local_day: fn(u64) -> i64,
    inner: Mutex<Inner>,
}

pub fn stats_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("ilot").join("stats.json"))
}

/// Day of the week, Monday 0 (1970-01-01 was a Thursday).
pub fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// Days since 1970-01-01 in UTC (tests).
pub fn utc_day(ms: u64) -> i64 {
    (ms as i64).div_euclid(DAY_MS)
}

/// Days since 1970-01-01 in the user's time zone, as Windows gives it now.
pub fn local_day(ms: u64) -> i64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
    let ticks = (ms + FILETIME_EPOCH_MS) * 10_000;
    let utc = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut local = FILETIME::default();
    // SAFETY: both pointers are valid FILETIME structures for the call.
    if unsafe { FileTimeToLocalFileTime(&utc, &mut local) }.is_err() {
        return utc_day(ms);
    }
    let local_ticks = (u64::from(local.dwHighDateTime) << 32) | u64::from(local.dwLowDateTime);
    ((local_ticks / 10_000) as i64 - FILETIME_EPOCH_MS as i64).div_euclid(DAY_MS)
}

/// "2026-10-05" for a day number (proleptic Gregorian calendar).
pub fn date_of(day: i64) -> String {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The day number of "2026-10-05".
pub fn day_of(date: &str) -> Option<i64> {
    let mut parts = date.splitn(3, '-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    Some(era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468)
}

fn source_id(source: Source) -> String {
    serde_json::to_value(source).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// Busiest first; equal times by name, so the order never flickers.
fn ranked(map: &BTreeMap<String, u64>) -> Vec<Share> {
    let mut list: Vec<Share> = map.iter().map(|(name, ms)| Share { name: name.clone(), ms: *ms }).collect();
    list.sort_by(|a, b| b.ms.cmp(&a.ms).then_with(|| a.name.cmp(&b.name)));
    list
}

impl Inner {
    /// The week that starts on `monday`, made when it is the newest; none for a week
    /// older than every week kept.
    fn week(&mut self, monday: i64) -> Option<&mut Week> {
        let date = date_of(monday);
        if let Some(index) = self.file.weeks.iter().position(|w| w.monday == date) {
            return self.file.weeks.get_mut(index);
        }
        let newest = self.file.weeks.iter().filter_map(|w| day_of(&w.monday)).max();
        if newest.is_some_and(|newest| newest > monday) {
            return None;
        }
        self.file.weeks.push(Week { monday: date, ..Week::default() });
        let extra = self.file.weeks.len().saturating_sub(MAX_WEEKS);
        self.file.weeks.drain(..extra);
        self.file.weeks.last_mut()
    }
}

impl Stats {
    /// Reads the file (missing or unreadable: no counters yet).
    pub fn open(path: Option<PathBuf>, local_day: fn(u64) -> i64) -> Self {
        let file = path.as_deref().map(read).unwrap_or_default();
        Self { path, local_day, inner: Mutex::new(Inner { file, ..Inner::default() }) }
    }

    /// Counters in memory only (tests of the core).
    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::open(None, utc_day)
    }

    /// Counts what an agent event says: working time, a new session, lines, a command, a
    /// file. Only numbers and the project name leave this function.
    pub fn event(&self, event: &AgentEvent) {
        let day = (self.local_day)(event.at);
        let monday = day - weekday(day);
        let key = crate::sessions::session_key(event.source, &event.session_id);
        let mut inner = self.inner.lock().unwrap();
        let new_session = !inner.live.contains_key(&key);
        if new_session && event.kind == EventKind::SessionEnd {
            return;
        }
        let live = inner.live.entry(key.clone()).or_insert(Live {
            in_turn: false,
            last_at: event.at,
            worked_ms: 0,
            monday,
            files: HashSet::new(),
        });
        if live.monday != monday {
            live.monday = monday;
            live.worked_ms = 0;
        }
        let worked = if live.in_turn { event.at.saturating_sub(live.last_at).min(GAP_CAP_MS) } else { 0 };
        live.worked_ms += worked;
        live.last_at = live.last_at.max(event.at);
        match event.kind {
            EventKind::Working | EventKind::Tool => live.in_turn = true,
            EventKind::TurnDone | EventKind::Interrupted | EventKind::Error | EventKind::SessionEnd => live.in_turn = false,
            _ => {}
        }
        let activity = event.activity.as_ref();
        let command = activity.is_some_and(|a| a.action == Action::Run);
        let file = activity
            .filter(|a| matches!(a.action, Action::Edit | Action::Write))
            .and_then(|a| a.target.clone())
            .is_some_and(|target| live.files.insert(target));
        let session_ms = live.worked_ms;
        if event.kind == EventKind::SessionEnd {
            inner.live.remove(&key);
        }
        let (added, removed) = activity.map_or((0, 0), |a| (u64::from(a.added), u64::from(a.removed)));
        if !(new_session || worked > 0 || command || file || added > 0 || removed > 0) {
            return;
        }
        let project: String = event.project.chars().take(PROJECT_CHARS).collect();
        let Some(week) = inner.week(monday) else { return };
        let today = &mut week.days[weekday(day) as usize];
        today.sessions += u32::from(new_session);
        today.active_ms += worked;
        today.commands += u32::from(command);
        today.files += u32::from(file);
        today.added += added;
        today.removed += removed;
        if worked > 0 {
            *week.agents.entry(source_id(event.source)).or_default() += worked;
            *week.projects.entry(project.clone()).or_default() += worked;
            if week.projects.len() > MAX_PROJECTS
                && let Some(least) = ranked(&week.projects).last().map(|s| s.name.clone())
            {
                week.projects.remove(&least);
            }
        }
        if session_ms > week.longest.as_ref().map_or(0, |l| l.ms) {
            week.longest = Some(Longest { ms: session_ms, source: event.source, project });
        }
        inner.dirty = true;
    }

    /// Counts the user's answer to a permission (the terminal and expired ones are not
    /// answers from îlot).
    pub fn decision(&self, kind: DecisionKind, at: u64) {
        let allowed = matches!(kind, DecisionKind::Allow | DecisionKind::Always);
        let denied = matches!(kind, DecisionKind::Deny | DecisionKind::Suspended);
        if !allowed && !denied {
            return;
        }
        let day = (self.local_day)(at);
        let mut inner = self.inner.lock().unwrap();
        let Some(week) = inner.week(day - weekday(day)) else { return };
        let today = &mut week.days[weekday(day) as usize];
        today.allowed += u32::from(allowed);
        today.denied += u32::from(denied);
        inner.dirty = true;
    }

    /// Writes the counters if they changed, and forgets the sessions gone quiet.
    pub fn flush(&self, now: u64) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.live.retain(|_, live| now.saturating_sub(live.last_at) < FORGET_SESSION_MS);
        if !inner.dirty {
            return Ok(());
        }
        if let Some(path) = &self.path {
            write(path, &inner.file)?;
        }
        inner.dirty = false;
        Ok(())
    }

    /// The report of last week or of this week so far; none when nothing was counted.
    pub fn report(&self, which: Which, now: u64) -> Option<Report> {
        let today = (self.local_day)(now);
        let monday = today - weekday(today) - if which == Which::Last { 7 } else { 0 };
        let date = date_of(monday);
        let inner = self.inner.lock().unwrap();
        let week = inner.file.weeks.iter().find(|w| w.monday == date)?;
        Some(report_of(week))
    }

    /// True once per Monday, when last week has a report to offer: the island then says
    /// it is ready.
    pub fn take_monday(&self, now: u64) -> bool {
        let today = (self.local_day)(now);
        if weekday(today) != 0 {
            return false;
        }
        let date = date_of(today);
        let last = date_of(today - 7);
        let mut inner = self.inner.lock().unwrap();
        let has_last = inner.file.weeks.iter().any(|w| w.monday == last && w.days.iter().any(|d| d != &Day::default()));
        if !has_last || inner.file.offered.as_deref() == Some(date.as_str()) {
            return false;
        }
        inner.file.offered = Some(date);
        inner.dirty = true;
        true
    }

    /// « Effacer mes statistiques »: the file goes, and so do the counters in memory.
    pub fn clear(&self) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.file = StatsFile::default();
        for live in inner.live.values_mut() {
            live.worked_ms = 0;
        }
        inner.dirty = false;
        match &self.path {
            Some(path) => crate::uninstall::remove_file_and_empty_folder(path),
            None => Ok(()),
        }
    }
}

fn report_of(week: &Week) -> Report {
    let mut total = Day::default();
    for day in &week.days {
        total.add(day);
    }
    let busiest_day = week
        .days
        .iter()
        .enumerate()
        .filter(|(_, d)| d.active_ms > 0)
        .max_by(|(a, x), (b, y)| x.active_ms.cmp(&y.active_ms).then_with(|| b.cmp(a)))
        .map(|(index, _)| index);
    Report {
        monday: week.monday.clone(),
        days: week.days.to_vec(),
        total,
        agents: ranked(&week.agents),
        projects: ranked(&week.projects).into_iter().take(5).collect(),
        busiest_day,
        longest: week.longest.clone(),
    }
}

fn read(path: &Path) -> StatsFile {
    std::fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

fn write(path: &Path, file: &StatsFile) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(file).map_err(io::Error::other)?;
    write_atomic(path, &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Activity;

    const MIN: u64 = 60_000;
    /// Monday 5 October 2026, 09:00 UTC.
    const MONDAY_9H: u64 = 1_791_190_800_000;

    fn event(kind: EventKind, session: &str, at: u64) -> AgentEvent {
        AgentEvent::new(Source::ClaudeCode, session, "/home/hic/projects/epilot", kind, "", at)
    }

    fn tool(session: &str, at: u64, verb: &str, target: &str, added: u32, removed: u32) -> AgentEvent {
        let action = match verb {
            "lance" => Action::Run,
            "édite" => Action::Edit,
            "écrit" => Action::Write,
            _ => Action::Read,
        };
        AgentEvent {
            activity: Some(Activity { action, verb: verb.into(), target: Some(target.into()), added, removed }),
            ..event(EventKind::Tool, session, at)
        }
    }

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ilot-stats-{name}-{}", std::process::id())).join("stats.json")
    }

    #[test]
    fn calendar_dates_round_trip() {
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(day_of("1970-01-01"), Some(0));
        assert_eq!(date_of(utc_day(MONDAY_9H)), "2026-10-05");
        assert_eq!(weekday(utc_day(MONDAY_9H)), 0);
        assert_eq!(day_of("2024-02-29").map(date_of).as_deref(), Some("2024-02-29"));
        assert_eq!(day_of("2026-13-01"), None);
        assert_eq!(day_of("rien"), None);
    }

    #[test]
    fn a_week_of_fake_events_adds_up() {
        let stats = Stats::in_memory();
        let at = |minutes: u64| MONDAY_9H + minutes * MIN;
        stats.event(&event(EventKind::SessionStart, "a", at(0)));
        stats.event(&event(EventKind::Working, "a", at(0)));
        stats.event(&tool("a", at(2), "lit", "api.py", 0, 0));
        stats.event(&tool("a", at(4), "édite", "api.py", 12, 3));
        stats.event(&tool("a", at(5), "édite", "api.py", 2, 1));
        stats.event(&tool("a", at(6), "écrit", "test_api.py", 40, 0));
        stats.event(&tool("a", at(7), "lance", "pytest -q", 0, 0));
        stats.event(&event(EventKind::TurnDone, "a", at(8)));
        // Idle between turns: not counted.
        stats.event(&event(EventKind::Working, "a", at(60)));
        // A 40-minute silence inside a turn counts 15 minutes at most.
        stats.event(&tool("a", at(100), "lance", "cargo test", 0, 0));
        stats.event(&event(EventKind::TurnDone, "a", at(101)));
        // Tuesday, Codex on another project.
        let tuesday = at(24 * 60);
        let codex = AgentEvent::new(Source::Codex, "b", "/home/hic/site", EventKind::Working, "", tuesday);
        stats.event(&codex);
        stats.event(&AgentEvent { kind: EventKind::TurnDone, at: tuesday + 12 * MIN, ..codex.clone() });
        stats.decision(DecisionKind::Allow, at(3));
        stats.decision(DecisionKind::Always, at(3));
        stats.decision(DecisionKind::Deny, at(4));
        stats.decision(DecisionKind::Terminal, at(4));

        let report = stats.report(Which::Current, tuesday).unwrap();
        assert_eq!(report.monday, "2026-10-05");
        let monday = &report.days[0];
        assert_eq!(monday.active_ms, (8 + 15 + 1) * MIN);
        assert_eq!((monday.sessions, monday.files, monday.commands), (1, 2, 2));
        assert_eq!((monday.added, monday.removed), (54, 4));
        assert_eq!((monday.allowed, monday.denied), (2, 1));
        assert_eq!(report.days[1].active_ms, 12 * MIN);
        assert_eq!(report.total.sessions, 2);
        assert_eq!(report.busiest_day, Some(0), "Monday: 24 min against 12");
        assert_eq!(report.agents, [Share { name: "claude-code".into(), ms: 24 * MIN }, Share { name: "codex".into(), ms: 12 * MIN }]);
        assert_eq!(report.projects[1], Share { name: "site".into(), ms: 12 * MIN });
        let longest = report.longest.unwrap();
        assert_eq!((longest.ms, longest.source, longest.project.as_str()), (24 * MIN, Source::ClaudeCode, "epilot"));
        assert_eq!(stats.report(Which::Last, tuesday), None);
    }

    #[test]
    fn a_new_week_starts_new_counters_and_keeps_five() {
        let stats = Stats::in_memory();
        for week in 0..7u64 {
            let at = MONDAY_9H + week * 7 * 24 * 60 * MIN;
            stats.event(&event(EventKind::Working, &format!("s{week}"), at));
            stats.event(&event(EventKind::TurnDone, &format!("s{week}"), at + (week + 1) * MIN));
        }
        let now = MONDAY_9H + 6 * 7 * 24 * 60 * MIN + 2 * 60 * MIN;
        assert_eq!(stats.report(Which::Current, now).unwrap().total.active_ms, 7 * MIN);
        assert_eq!(stats.report(Which::Last, now).unwrap().total.active_ms, 6 * MIN);
        let inner = stats.inner.lock().unwrap();
        assert_eq!(inner.file.weeks.len(), MAX_WEEKS);
        assert_eq!(inner.file.weeks[0].monday, "2026-10-19", "the two oldest weeks left");
        drop(inner);
        // A late event for a week no longer kept is dropped, not counted in another one.
        stats.decision(DecisionKind::Allow, MONDAY_9H);
        assert_eq!(stats.report(Which::Current, now).unwrap().total.allowed, 0);
    }

    #[test]
    fn a_session_running_into_the_next_week_counts_from_zero_there() {
        let stats = Stats::in_memory();
        let sunday_night = MONDAY_9H + 6 * 24 * 60 * MIN + 14 * 60 * MIN;
        stats.event(&event(EventKind::Working, "a", sunday_night - 4 * MIN));
        stats.event(&event(EventKind::TurnDone, "a", sunday_night));
        let monday = sunday_night + 2 * 60 * MIN;
        stats.event(&event(EventKind::Working, "a", monday));
        stats.event(&event(EventKind::TurnDone, "a", monday + 3 * MIN));
        let next = stats.report(Which::Current, monday).unwrap();
        assert_eq!(next.total.sessions, 0, "the session started last week");
        assert_eq!(next.longest.unwrap().ms, 3 * MIN);
        assert_eq!(stats.report(Which::Last, monday).unwrap().total.active_ms, 4 * MIN);
    }

    #[test]
    fn the_file_holds_counters_and_project_names_only() {
        let path = temp("content");
        let stats = Stats::open(Some(path.clone()), utc_day);
        let secret = AgentEvent {
            detail: Some("Refactor the payment module with my token sk-ant-secret".into()),
            title: "réponds au client Dupont".into(),
            ..AgentEvent::new(Source::ClaudeCode, "5c1e7-session", "/home/hic/clients/dupont-sa", EventKind::Working, "", MONDAY_9H)
        };
        stats.event(&secret);
        stats.event(&AgentEvent {
            kind: EventKind::Tool,
            at: MONDAY_9H + MIN,
            activity: Some(Activity { action: Action::Run, verb: "lance".into(), target: Some("curl -H 'x-api-key: hunter2' https://x".into()), added: 0, removed: 0 }),
            ..secret.clone()
        });
        stats.event(&AgentEvent {
            kind: EventKind::Tool,
            at: MONDAY_9H + 2 * MIN,
            activity: Some(Activity { action: Action::Edit, verb: "édite".into(), target: Some("contrat-confidentiel.md".into()), added: 3, removed: 1 }),
            ..secret.clone()
        });
        stats.decision(DecisionKind::Allow, MONDAY_9H + 3 * MIN);
        stats.flush(MONDAY_9H + 4 * MIN).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        for leak in ["sk-ant", "Refactor", "Dupont", "5c1e7", "/home", "clients", "curl", "hunter2", "contrat", ".md"] {
            assert!(!written.contains(leak), "{leak} must not be written: {written}");
        }
        assert!(written.contains("dupont-sa"), "the project name is kept");
        // Every text in the file, values and map keys alike, is a date, an AI id or a
        // project name; everything else is a number.
        let value: serde_json::Value = serde_json::from_str(&written).unwrap();
        let mut texts = Vec::new();
        texts_in(&value, &mut texts);
        assert!(texts.iter().all(|s| ["2026-10-05", "claude-code", "dupont-sa"].contains(&s.as_str())), "{texts:?}");
        assert_eq!(Stats::open(Some(path.clone()), utc_day).report(Which::Current, MONDAY_9H).unwrap().total.files, 1);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// The file's own field names; any other key is data (an AI id or a project name).
    const FIELDS: [&str; 18] = [
        "weeks", "offered", "monday", "days", "agents", "projects", "longest", "activeMs", "sessions", "files", "added",
        "removed", "commands", "allowed", "denied", "ms", "source", "project",
    ];

    fn texts_in(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(items) => items.iter().for_each(|v| texts_in(v, out)),
            serde_json::Value::Object(map) => {
                for (key, v) in map {
                    if !FIELDS.contains(&key.as_str()) {
                        out.push(key.clone());
                    }
                    texts_in(v, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn the_monday_report_is_offered_once_and_cleared_on_demand() {
        let path = temp("monday");
        let stats = Stats::open(Some(path.clone()), utc_day);
        stats.event(&event(EventKind::Working, "a", MONDAY_9H));
        stats.event(&event(EventKind::TurnDone, "a", MONDAY_9H + MIN));
        assert!(!stats.take_monday(MONDAY_9H + 2 * MIN), "nothing to offer for the week before");
        let next_monday = MONDAY_9H + 7 * 24 * 60 * MIN;
        assert!(!stats.take_monday(next_monday - 24 * 60 * MIN), "Sunday is not Monday");
        assert!(stats.take_monday(next_monday));
        assert!(!stats.take_monday(next_monday + 60 * MIN), "once per Monday");
        stats.flush(next_monday).unwrap();
        assert!(path.exists());
        stats.clear().unwrap();
        assert!(!path.exists(), "the file is gone");
        assert_eq!(stats.report(Which::Last, next_monday), None);
        assert!(!path.parent().unwrap().exists(), "and its empty folder");
    }

    #[test]
    fn missing_or_broken_files_give_no_counters() {
        assert_eq!(read(Path::new("Z:/nowhere/stats.json")), StatsFile::default());
        let path = temp("broken");
        write_atomic(&path, b"{ not json").unwrap();
        assert_eq!(read(&path), StatsFile::default());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn local_days_follow_windows() {
        // Whatever the time zone, midday UTC stays on the same date somewhere within a day.
        let day = local_day(MONDAY_9H + 3 * 60 * MIN);
        assert!((utc_day(MONDAY_9H) - 1..=utc_day(MONDAY_9H) + 1).contains(&day));
    }
}
