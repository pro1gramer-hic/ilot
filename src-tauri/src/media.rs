//! What plays on Windows, read through the System Media Transport Controls (SMTC):
//! Spotify, browsers (YouTube, Crunchyroll…) and any app shown in the Windows media
//! flyout. The `globalMediaControl` capability only concerns packaged apps, so an
//! unpackaged Win32 app such as îlot may use these APIs (see docs/JOURNAL.md, phase 5).
//!
//! No polling: a dedicated thread waits for SMTC events, reads the sessions and pushes
//! `media:changed` (the current session) and `media:sessions` (every session and the
//! history of this run, kept in memory only) to the island and the mini-player.

use crate::artwork;
use crate::locale::{Text, tr};
use crate::players::MINI_LABEL;
use crate::source_app;
use crate::window::WINDOW_LABEL;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Media::MediaPlaybackAutoRepeatMode;

pub const EVENT_CHANGED: &str = "media:changed";
pub const EVENT_SESSIONS: &str = "media:sessions";

/// Media shown in "Joué récemment" and in the mini-player thumbnails.
const RECENT_MAX: usize = 8;
/// SMTC events come in bursts (title, then cover, then timeline): wait for calm.
const SETTLE: Duration = Duration::from_millis(120);
/// A position that drifts less than this from the interface's own estimate is not resent.
const DRIFT_SECONDS: f64 = 1.5;
/// Ids of the YouTube web app installed from Edge or Chrome; Chrome shortens it in its
/// SMTC app id ("Chrome._crx_agimnkijcamfeangaknmldooml", seen on a real PC).
const YOUTUBE_WEB_APP: [&str; 2] = ["agimnkijcaahngcdmfeangaknmldooml", "agimnkijcamfeangaknmldooml"];
/// What sites append to their titles; the island shows the title alone.
const TITLE_SUFFIXES: [&str; 6] = [
    " - regardez sur crunchyroll",
    " - watch on crunchyroll",
    " | crunchyroll",
    " - crunchyroll",
    " - youtube music",
    " - youtube",
];

const NO_ACCESS: Text<'static> = Text::MediaNoAccess;
const NOTHING_PLAYING: Text<'static> = Text::MediaNothingPlaying;
const REFUSED: Text<'static> = Text::MediaRefused;

// ---------------------------------------------------------------------------------------
// Model shared with the interface (src/lib/media.ts)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum MediaSource {
    Spotify,
    YouTube,
    Crunchyroll,
    #[serde(rename = "other")]
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Controls {
    pub play_pause: bool,
    pub next: bool,
    pub previous: bool,
    /// The session lets others change its shuffle (`IsShuffleEnabled`) and repeat
    /// (`IsRepeatEnabled`): nothing says Spotify does, the buttons show only if so.
    pub shuffle: bool,
    pub repeat: bool,
}

/// Repeat mode of a session (`MediaPlaybackAutoRepeatMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    None,
    Track,
    List,
}

impl RepeatMode {
    fn from_windows(mode: MediaPlaybackAutoRepeatMode) -> Option<Self> {
        match mode {
            MediaPlaybackAutoRepeatMode::None => Some(RepeatMode::None),
            MediaPlaybackAutoRepeatMode::Track => Some(RepeatMode::Track),
            MediaPlaybackAutoRepeatMode::List => Some(RepeatMode::List),
            _ => None,
        }
    }

    fn to_windows(self) -> MediaPlaybackAutoRepeatMode {
        match self {
            RepeatMode::None => MediaPlaybackAutoRepeatMode::None,
            RepeatMode::Track => MediaPlaybackAutoRepeatMode::Track,
            RepeatMode::List => MediaPlaybackAutoRepeatMode::List,
        }
    }
}

/// One media session as the interface shows it (type `Track` in src/lib/media.ts).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Stable key: the same app playing the same title.
    pub id: String,
    pub title: String,
    pub artist: String,
    pub source: MediaSource,
    /// Name of the app that plays ("Spotify", "Microsoft Edge"…).
    pub app: String,
    /// SMTC app id (AUMID), used to send commands to this session.
    pub app_id: String,
    pub playing: bool,
    /// Seconds, at `updated_at`.
    pub position: f64,
    /// Seconds; 0 when unknown or live.
    pub duration: f64,
    /// Epoch milliseconds when `position` was true; the interface moves it forward.
    pub updated_at: i64,
    pub controls: Controls,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<String>,
    /// YouTube video playing in îlot's own player window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    /// Shuffle on or off, when the session says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shuffle: Option<bool>,
    /// Repeat mode, when the session says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatMode>,
}

/// Commands accepted by `media_command` (contract shared with the interface).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaAction {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
}

/// Everything the interface needs when it starts (`get_media`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub current: Option<Track>,
    pub sessions: Vec<Track>,
    pub recent: Vec<Track>,
    /// Set when Windows refuses access to the media sessions.
    pub error: Option<String>,
}

/// Payload of `media:sessions`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionsPayload<'a> {
    sessions: &'a [Track],
    recent: &'a [Track],
    error: &'a Option<String>,
}

// ---------------------------------------------------------------------------------------
// Pure rules (tested below)

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Browsers expose the tab that plays, not the site: the site is guessed from texts.
pub fn is_browser(app_id: &str) -> bool {
    let id = app_id.to_lowercase();
    ["msedge", "chrome", "firefox", "308046b0af4a39cb", "brave", "opera", "vivaldi"]
        .iter()
        .any(|name| id.contains(name))
}

/// Which source plays, from the SMTC app id and texts that may name the site: the
/// media title and artist, and the titles of the browser windows showing that media
/// ("Ma vidéo - YouTube - Microsoft Edge").
pub fn identify_source(app_id: &str, texts: &[&str]) -> MediaSource {
    let id = app_id.to_lowercase();
    if id.contains("spotify") {
        return MediaSource::Spotify;
    }
    if id.contains("crunchyroll") {
        return MediaSource::Crunchyroll;
    }
    if id.contains("youtube") || YOUTUBE_WEB_APP.iter().any(|app| id.contains(app)) {
        return MediaSource::YouTube;
    }
    let mentions = |site: &str| texts.iter().any(|text| contains_ignore_case(text, site));
    if mentions("crunchyroll") {
        MediaSource::Crunchyroll
    } else if mentions("youtube") {
        MediaSource::YouTube
    } else {
        MediaSource::Other
    }
}

/// Readable name of the app behind an SMTC app id.
pub fn app_name(app_id: &str) -> String {
    let id = app_id.to_lowercase();
    const KNOWN: [(&str, &str); 9] = [
        ("spotify", "Spotify"),
        ("msedge", "Microsoft Edge"),
        ("chrome", "Google Chrome"),
        ("firefox", "Firefox"),
        ("308046b0af4a39cb", "Firefox"),
        ("brave", "Brave"),
        ("opera", "Opera"),
        ("vivaldi", "Vivaldi"),
        ("vlc", "VLC"),
    ];
    if id.contains("zunemusic") {
        return tr(Text::MediaPlayerApp);
    }
    if let Some((_, name)) = KNOWN.iter().find(|(key, _)| id.contains(key)) {
        return (*name).to_owned();
    }
    let name = match app_id.split_once('!') {
        // Packaged app: "Publisher.Name_hash!Entry".
        Some((package, _)) => {
            let family = package.split('_').next().unwrap_or(package);
            family.rsplit('.').next().unwrap_or(family)
        }
        // Desktop app: "C:\…\app.exe" or "app.exe".
        None => {
            let file = app_id.rsplit(['\\', '/']).next().unwrap_or(app_id);
            let len = file.len();
            if len > 4 && file[len - 4..].eq_ignore_ascii_case(".exe") { &file[..len - 4] } else { file }
        }
    };
    if name.trim().is_empty() { tr(Text::OtherApp) } else { name.to_owned() }
}

/// Title without the site name that some apps append ("… - Regardez sur Crunchyroll").
pub fn clean_title(title: &str) -> String {
    let trimmed = title.trim();
    let lower = trimmed.to_lowercase();
    TITLE_SUFFIXES
        .iter()
        .find(|suffix| lower.ends_with(*suffix) && lower.len() > suffix.len())
        // Lowercasing keeps byte lengths for these ASCII suffixes.
        .map_or(trimmed, |suffix| trimmed[..trimmed.len() - suffix.len()].trim_end())
        .to_owned()
}

/// Window titles that show the given media, used to recognize the site in a browser.
pub fn titles_showing<'a>(window_titles: &'a [String], media_title: &str) -> Vec<&'a str> {
    if media_title.trim().is_empty() {
        return Vec::new();
    }
    window_titles
        .iter()
        .filter(|title| contains_ignore_case(title, media_title.trim()))
        .map(String::as_str)
        .collect()
}

/// A YouTube video id: 11 characters among letters, digits, `-` and `_`.
pub fn is_video_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The position to ask for, in ticks of 100 ns on the session's timeline, to play from
/// `seconds` after its start; kept within the media when its end is known.
pub fn seek_ticks(seconds: f64, start: i64, end: i64) -> Option<i64> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let target = start.saturating_add((seconds * 10_000_000.0).round() as i64);
    Some(if end > start { target.min(end) } else { target })
}

/// Ticks of 100 ns (Windows `TimeSpan`) to seconds.
pub fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / 10_000_000.0
}

/// Windows `DateTime` (100 ns since 1601) to Unix milliseconds; `None` when unset.
pub fn universal_time_to_unix_ms(universal: i64) -> Option<i64> {
    const UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;
    (universal > UNIX_EPOCH_TICKS).then(|| (universal - UNIX_EPOCH_TICKS) / 10_000)
}

/// Where playback is at `now_ms`, from the position Windows read at `read_at_ms`.
pub fn position_at(
    position: f64,
    read_at_ms: Option<i64>,
    now_ms: i64,
    playing: bool,
    rate: f64,
    duration: f64,
) -> f64 {
    let elapsed = match (playing, read_at_ms) {
        (true, Some(read_at)) => (now_ms - read_at).max(0) as f64 / 1000.0 * rate.max(0.0),
        _ => 0.0,
    };
    let position = (position + elapsed).max(0.0);
    if duration > 0.0 { position.min(duration) } else { position }
}

/// What one SMTC session says, as read from Windows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionInfo {
    pub app_id: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub rate: f64,
    /// Timeline, in 100 ns ticks.
    pub start: i64,
    pub end: i64,
    pub position: i64,
    /// Windows `DateTime` of the last timeline update.
    pub last_updated: i64,
    pub controls: Controls,
    pub artwork: Option<String>,
    pub shuffle: Option<bool>,
    pub repeat: Option<RepeatMode>,
}

/// Converts what Windows says about a session into the track shown by the interface.
pub fn to_track(info: &SessionInfo, source: MediaSource, now_ms: i64, video_id: Option<String>) -> Track {
    let duration = ticks_to_seconds(info.end - info.start).max(0.0);
    let position = position_at(
        ticks_to_seconds(info.position - info.start),
        universal_time_to_unix_ms(info.last_updated),
        now_ms,
        info.playing,
        info.rate,
        duration,
    );
    let app = if video_id.is_some() { "îlot".to_owned() } else { app_name(&info.app_id) };
    Track {
        id: format!("{}\u{1f}{}\u{1f}{}", info.app_id, info.title, info.artist),
        title: clean_title(&info.title),
        artist: info.artist.clone(),
        source,
        app,
        app_id: info.app_id.clone(),
        playing: info.playing,
        position,
        duration,
        updated_at: now_ms,
        controls: info.controls,
        artwork: info.artwork.clone(),
        video_id,
        shuffle: info.shuffle,
        repeat: info.repeat,
    }
}

/// The session Windows calls current, or else the first one that plays.
pub fn pick_current(tracks: &[Track], system_current: Option<&str>) -> Option<Track> {
    system_current
        .and_then(|id| tracks.iter().find(|t| t.app_id == id))
        .or_else(|| tracks.iter().find(|t| t.playing))
        .cloned()
}

/// Adds a playing track to the history of this run: newest first, no duplicates.
pub fn remember(recent: &mut Vec<Track>, track: &Track) {
    if !track.playing || track.title.trim().is_empty() {
        return;
    }
    recent.retain(|t| t.id != track.id);
    recent.insert(0, track.clone());
    recent.truncate(RECENT_MAX);
}

/// True when the interface must hear about the change: anything but the time moved, or
/// the time no longer matches what the interface computes on its own (seek, stall).
pub fn needs_emit(previous: Option<&Track>, next: Option<&Track>) -> bool {
    match (previous, next) {
        (None, None) => false,
        (Some(a), Some(b)) => {
            let untimed = |t: &Track| Track { position: 0.0, updated_at: 0, ..t.clone() };
            if untimed(a) != untimed(b) {
                return true;
            }
            let expected = position_at(a.position, Some(a.updated_at), b.updated_at, a.playing, 1.0, a.duration);
            (expected - b.position).abs() > DRIFT_SECONDS
        }
        _ => true,
    }
}

fn lists_differ(previous: &[Track], next: &[Track]) -> bool {
    previous.len() != next.len() || previous.iter().zip(next).any(|(a, b)| needs_emit(Some(a), Some(b)))
}

// ---------------------------------------------------------------------------------------
// Shared state and Tauri commands

#[derive(Default)]
pub struct MediaHub {
    snapshot: Mutex<MediaSnapshot>,
    manager: Mutex<Option<SessionManager>>,
    /// Wakes the media thread when îlot's own YouTube window changes.
    wake: Mutex<Option<Sender<Signal>>>,
    /// Video loaded in îlot's own YouTube window.
    video: Mutex<Option<String>>,
}

impl MediaHub {
    fn manager(&self) -> Option<SessionManager> {
        self.manager.lock().unwrap().clone()
    }

    /// Remembers the video shown by îlot's own player window, so its session is
    /// recognized as YouTube and can be played again from the history.
    pub fn set_video(&self, video_id: Option<String>) {
        *self.video.lock().unwrap() = video_id;
        if let Some(wake) = self.wake.lock().unwrap().as_ref() {
            let _ = wake.send(Signal::Sessions);
        }
    }

    fn video(&self) -> Option<String> {
        self.video.lock().unwrap().clone()
    }
}

#[tauri::command]
pub fn get_media(hub: State<'_, MediaHub>) -> MediaSnapshot {
    hub.snapshot.lock().unwrap().clone()
}

/// Sends a command to a session: the one with this app id, or else the current one.
#[tauri::command]
pub async fn media_command(
    action: MediaAction,
    app_id: Option<String>,
    hub: State<'_, MediaHub>,
) -> Result<(), String> {
    let manager = hub.manager().ok_or(NO_ACCESS)?;
    let session = find_session(&manager, app_id.as_deref()).ok_or(NOTHING_PLAYING)?;
    let request = match action {
        MediaAction::Play => session.TryPlayAsync(),
        MediaAction::Pause => session.TryPauseAsync(),
        MediaAction::Toggle => session.TryTogglePlayPauseAsync(),
        MediaAction::Next => session.TrySkipNextAsync(),
        MediaAction::Previous => session.TrySkipPreviousAsync(),
    };
    match request.and_then(|operation| operation.join()) {
        Ok(true) => Ok(()),
        Ok(false) | Err(_) => Err(REFUSED.into()),
    }
}

/// Moves this app's session to `seconds` from the start of the media (the clickable
/// wave). `TryChangePlaybackPositionAsync` takes ticks of 100 ns
/// (https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssession.trychangeplaybackpositionasync);
/// an app that does not allow seeking answers no.
#[tauri::command]
pub async fn media_seek(app_id: String, seconds: f64, hub: State<'_, MediaHub>) -> Result<(), String> {
    let manager = hub.manager().ok_or(NO_ACCESS)?;
    let session = find_session(&manager, Some(&app_id)).ok_or(NOTHING_PLAYING)?;
    let timeline = session.GetTimelineProperties().map_err(|_| REFUSED)?;
    let start = timeline.StartTime().map_or(0, |t| t.Duration);
    let end = timeline.EndTime().map_or(0, |t| t.Duration);
    let target = seek_ticks(seconds, start, end).ok_or(REFUSED)?;
    match session.TryChangePlaybackPositionAsync(target).and_then(|operation| operation.join()) {
        Ok(true) => Ok(()),
        Ok(false) | Err(_) => Err(REFUSED.into()),
    }
}

/// Shuffle and repeat of this app's session
/// (https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssession,
/// `TryChangeShuffleActiveAsync`, `TryChangeAutoRepeatModeAsync`). An app that does not
/// let others change them answers no.
#[tauri::command]
pub async fn media_mode(
    app_id: String,
    shuffle: Option<bool>,
    repeat: Option<RepeatMode>,
    hub: State<'_, MediaHub>,
) -> Result<(), String> {
    let manager = hub.manager().ok_or(NO_ACCESS)?;
    let session = find_session(&manager, Some(&app_id)).ok_or(NOTHING_PLAYING)?;
    if let Some(on) = shuffle
        && !matches!(session.TryChangeShuffleActiveAsync(on).and_then(|operation| operation.join()), Ok(true))
    {
        return Err(REFUSED.into());
    }
    if let Some(mode) = repeat
        && !matches!(session.TryChangeAutoRepeatModeAsync(mode.to_windows()).and_then(|operation| operation.join()), Ok(true))
    {
        return Err(REFUSED.into());
    }
    Ok(())
}

/// Brings the app or browser that plays this session to the foreground.
#[tauri::command]
pub async fn open_media_source(app_id: String, app: AppHandle) -> Result<(), String> {
    let hub = app.state::<MediaHub>();
    let snapshot = hub.snapshot.lock().unwrap().clone();
    let track = snapshot.sessions.iter().find(|t| t.app_id == app_id).ok_or(NOTHING_PLAYING)?;
    if track.video_id.is_some() {
        return crate::players::show_video_window(&app);
    }
    // Crunchyroll plays in îlot's own window: show it rather than look for a browser.
    if crate::video::is_own_session(&track.app_id) {
        return crate::video::show_full(&app, crate::video::Site::Crunchyroll);
    }
    source_app::bring_to_front(&track.app_id, &track.title)
}

fn find_session(manager: &SessionManager, app_id: Option<&str>) -> Option<Session> {
    match app_id {
        None => manager.GetCurrentSession().ok(),
        Some(id) => manager
            .GetSessions()
            .ok()?
            .into_iter()
            .find(|session| session.SourceAppUserModelId().is_ok_and(|aumid| aumid == id)),
    }
}

// ---------------------------------------------------------------------------------------
// The media thread: SMTC events in, interface events out

#[derive(Clone, Debug, PartialEq, Eq)]
enum Signal {
    /// Sessions appeared, disappeared, or Windows picked another current one.
    Sessions,
    /// Title, artist or cover of this app's session changed.
    Properties(String),
    /// Playback status or timeline changed.
    Playback,
}

#[derive(Default)]
struct Pending {
    sessions: bool,
    properties: Vec<String>,
}

impl Pending {
    fn add(&mut self, signal: Signal) {
        match signal {
            Signal::Sessions => self.sessions = true,
            Signal::Properties(app_id) => self.properties.push(app_id),
            Signal::Playback => {}
        }
    }
}

/// What does not change between two play/pause events: read again only on
/// `MediaPropertiesChanged`, because decoding the cover costs a few milliseconds.
#[derive(Clone)]
struct Properties {
    title: String,
    artist: String,
    artwork: Option<String>,
    source: MediaSource,
}

struct Watched {
    app_id: String,
    session: Session,
    tokens: [i64; 3],
    properties: Option<Properties>,
}

impl Watched {
    fn subscribe(session: Session, app_id: String, tx: &Sender<Signal>) -> windows::core::Result<Self> {
        let tokens = [
            session.MediaPropertiesChanged(&notify(tx, Signal::Properties(app_id.clone())))?,
            session.PlaybackInfoChanged(&notify(tx, Signal::Playback))?,
            session.TimelinePropertiesChanged(&notify(tx, Signal::Playback))?,
        ];
        Ok(Self { app_id, session, tokens, properties: None })
    }

    fn unsubscribe(&self) {
        let [properties, playback, timeline] = self.tokens;
        let _ = self.session.RemoveMediaPropertiesChanged(properties);
        let _ = self.session.RemovePlaybackInfoChanged(playback);
        let _ = self.session.RemoveTimelinePropertiesChanged(timeline);
    }
}

/// Event handler that only wakes the media thread: the work happens there.
fn notify<S, A>(tx: &Sender<Signal>, signal: Signal) -> TypedEventHandler<S, A>
where
    S: windows::core::RuntimeType + 'static,
    A: windows::core::RuntimeType + 'static,
{
    let tx = tx.clone();
    TypedEventHandler::new(move |_, _| {
        let _ = tx.send(signal.clone());
        Ok(())
    })
}

/// App ids under which îlot's own player window may appear in the SMTC.
fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Starts reading the media sessions in the background.
pub fn start(app: &AppHandle) -> tauri::Result<()> {
    let app = app.clone();
    std::thread::Builder::new().name("ilot-media".into()).spawn(move || {
        let (tx, rx) = mpsc::channel();
        *app.state::<MediaHub>().wake.lock().unwrap() = Some(tx.clone());
        if let Err(error) = run(&app, &tx, &rx) {
            eprintln!("îlot: media sessions unavailable: {error}");
            let hub = app.state::<MediaHub>();
            let mut snapshot = hub.snapshot.lock().unwrap();
            *snapshot = MediaSnapshot { error: Some(NO_ACCESS.into()), ..MediaSnapshot::default() };
            emit_all(&app, EVENT_CHANGED, &None::<Track>);
            emit_all(&app, EVENT_SESSIONS, &payload(&snapshot));
        }
    })?;
    Ok(())
}

fn payload(snapshot: &MediaSnapshot) -> SessionsPayload<'_> {
    SessionsPayload { sessions: &snapshot.sessions, recent: &snapshot.recent, error: &snapshot.error }
}

/// Media events go to îlot's own windows only, never to the YouTube player window.
fn emit_all<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: &S) {
    for label in [WINDOW_LABEL, MINI_LABEL] {
        let _ = app.emit_to(label, event, payload.clone());
    }
}

fn run(app: &AppHandle, tx: &Sender<Signal>, rx: &Receiver<Signal>) -> windows::core::Result<()> {
    let manager = SessionManager::RequestAsync()?.join()?;
    manager.SessionsChanged(&notify(tx, Signal::Sessions))?;
    manager.CurrentSessionChanged(&notify(tx, Signal::Sessions))?;
    *app.state::<MediaHub>().manager.lock().unwrap() = Some(manager.clone());

    let mut watched: Vec<Watched> = Vec::new();
    let mut pending = Pending { sessions: true, properties: Vec::new() };
    loop {
        if pending.sessions {
            resubscribe(&manager, &mut watched, tx);
        }
        for w in &mut watched {
            if pending.properties.contains(&w.app_id) {
                w.properties = None;
            }
        }
        publish(app, &manager, &mut watched);

        let Ok(first) = rx.recv() else { return Ok(()) };
        pending = Pending::default();
        pending.add(first);
        while let Ok(next) = rx.recv_timeout(SETTLE) {
            pending.add(next);
        }
    }
}

/// Follows exactly the sessions Windows lists now; known ones keep their cached properties.
fn resubscribe(manager: &SessionManager, watched: &mut Vec<Watched>, tx: &Sender<Signal>) {
    let live: Vec<Session> = manager.GetSessions().map(|v| v.into_iter().collect()).unwrap_or_default();
    let mut next = Vec::with_capacity(live.len());
    for session in live {
        let Ok(app_id) = session.SourceAppUserModelId().map(|id| id.to_string_lossy()) else { continue };
        match watched.iter().position(|w| w.app_id == app_id && w.session == session) {
            Some(index) => next.push(watched.remove(index)),
            None => {
                if let Ok(w) = Watched::subscribe(session, app_id, tx) {
                    next.push(w);
                }
            }
        }
    }
    for gone in watched.drain(..) {
        gone.unsubscribe();
    }
    *watched = next;
}

fn read_properties(session: &Session, app_id: &str, video: Option<&str>) -> Properties {
    let read = session.TryGetMediaPropertiesAsync().and_then(|operation| operation.join());
    let (title, artist, artwork) = match &read {
        Ok(p) => (
            p.Title().map(|t| t.to_string_lossy()).unwrap_or_default(),
            p.Artist().map(|a| a.to_string_lossy()).unwrap_or_default(),
            p.Thumbnail().and_then(|thumbnail| artwork::read(&thumbnail)).ok(),
        ),
        Err(_) => Default::default(),
    };
    let source = guess_source(app_id, &title, &artist, video);
    Properties { title, artist, artwork, source }
}

fn guess_source(app_id: &str, title: &str, artist: &str, video: Option<&str>) -> MediaSource {
    if video.is_some() && crate::video::is_own_session(app_id) {
        return MediaSource::YouTube;
    }
    let windows = if is_browser(app_id) {
        source_app::window_titles(&source_app::process_names(app_id))
    } else {
        Vec::new()
    };
    let mut texts = vec![title, artist];
    texts.extend(titles_showing(&windows, title));
    identify_source(app_id, &texts)
}

fn read_info(w: &Watched, properties: &Properties) -> SessionInfo {
    let mut info = SessionInfo {
        app_id: w.app_id.clone(),
        title: properties.title.clone(),
        artist: properties.artist.clone(),
        rate: 1.0,
        artwork: properties.artwork.clone(),
        ..SessionInfo::default()
    };
    if let Ok(playback) = w.session.GetPlaybackInfo() {
        info.playing = playback.PlaybackStatus().is_ok_and(|s| s == PlaybackStatus::Playing);
        info.rate = playback.PlaybackRate().and_then(|r| r.Value()).unwrap_or(1.0);
        if let Ok(c) = playback.Controls() {
            let enabled = |flag: windows::core::Result<bool>| flag.unwrap_or(false);
            info.controls = Controls {
                play_pause: enabled(c.IsPlayPauseToggleEnabled()) || enabled(c.IsPlayEnabled()) || enabled(c.IsPauseEnabled()),
                next: enabled(c.IsNextEnabled()),
                previous: enabled(c.IsPreviousEnabled()),
                shuffle: enabled(c.IsShuffleEnabled()),
                repeat: enabled(c.IsRepeatEnabled()),
            };
        }
        info.shuffle = playback.IsShuffleActive().and_then(|r| r.Value()).ok();
        info.repeat = playback.AutoRepeatMode().and_then(|r| r.Value()).ok().and_then(RepeatMode::from_windows);
    }
    if let Ok(timeline) = w.session.GetTimelineProperties() {
        info.start = timeline.StartTime().map_or(0, |t| t.Duration);
        info.end = timeline.EndTime().map_or(0, |t| t.Duration);
        info.position = timeline.Position().map_or(0, |t| t.Duration);
        info.last_updated = timeline.LastUpdatedTime().map_or(0, |t| t.UniversalTime);
    }
    info
}

/// Reads every session, updates the shared snapshot and tells the windows what changed.
fn publish(app: &AppHandle, manager: &SessionManager, watched: &mut [Watched]) {
    let hub = app.state::<MediaHub>();
    let video = hub.video();
    let now = now_ms();
    let mut tracks = Vec::with_capacity(watched.len());
    for w in watched.iter_mut() {
        let properties = match w.properties.as_mut() {
            None => w.properties.insert(read_properties(&w.session, &w.app_id, video.as_deref())),
            Some(p) => {
                if p.source == MediaSource::Other && is_browser(&w.app_id) {
                    // The tab title may change after the media: look at the windows again.
                    p.source = guess_source(&w.app_id, &p.title, &p.artist, video.as_deref());
                }
                p
            }
        }
        .clone();
        let own = video.clone().filter(|_| crate::video::is_own_session(&w.app_id));
        tracks.push(to_track(&read_info(w, &properties), properties.source, now, own));
    }
    let system_current = manager.GetCurrentSession().and_then(|s| s.SourceAppUserModelId()).ok();
    let current = pick_current(&tracks, system_current.map(|id| id.to_string_lossy()).as_deref());

    let mut snapshot = hub.snapshot.lock().unwrap();
    let mut recent = snapshot.recent.clone();
    for track in &tracks {
        remember(&mut recent, track);
    }
    let current_changed = needs_emit(snapshot.current.as_ref(), current.as_ref());
    let lists_changed = lists_differ(&snapshot.sessions, &tracks) || recent != snapshot.recent;
    *snapshot = MediaSnapshot { current, sessions: tracks, recent, error: None };
    if current_changed {
        emit_all(app, EVENT_CHANGED, &snapshot.current);
    }
    if lists_changed {
        emit_all(app, EVENT_SESSIONS, &payload(&snapshot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOTIFY: &str = "Spotify.exe";
    const SPOTIFY_STORE: &str = "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify";
    const EDGE: &str = "MSEdge";
    const CHROME: &str = "Chrome";
    const FIREFOX: &str = "308046B0AF4A39CB";

    #[test]
    fn spotify_is_recognized_by_its_app_id() {
        assert_eq!(identify_source(SPOTIFY, &["Blinding Lights"]), MediaSource::Spotify);
        assert_eq!(identify_source(SPOTIFY_STORE, &[]), MediaSource::Spotify);
    }

    #[test]
    fn browser_sites_are_recognized_from_titles() {
        assert_eq!(
            identify_source(EDGE, &["Lofi Girl", "Pluie de nuit - YouTube - Microsoft\u{200b} Edge"]),
            MediaSource::YouTube
        );
        assert_eq!(
            identify_source(CHROME, &["Regarder Frieren Épisode 7 | Crunchyroll"]),
            MediaSource::Crunchyroll
        );
        assert_eq!(identify_source(FIREFOX, &["Ma vidéo — YouTube Music"]), MediaSource::YouTube);
    }

    #[test]
    fn web_apps_and_unknown_apps() {
        assert_eq!(identify_source("MSEdge._crx_agimnkijcaahngcdmfeangaknmldooml", &[]), MediaSource::YouTube);
        assert_eq!(identify_source(EDGE, &["Un podcast", "Radio France"]), MediaSource::Other);
        assert_eq!(identify_source("VideoLAN.VLC", &["film.mkv"]), MediaSource::Other);
    }

    #[test]
    fn apps_seen_on_a_real_pc() {
        let crunchyroll = "15EF7777.Crunchyroll_mgdgtskya6f22!App";
        let title = "Presque mariés, loin d'être amoureux. Plus qu'un fantasme, mais pas la réalité - Regardez sur Crunchyroll";
        assert_eq!(identify_source(crunchyroll, &[title]), MediaSource::Crunchyroll);
        assert_eq!(app_name(crunchyroll), "Crunchyroll");
        let youtube_app = "Chrome._crx_agimnkijcamfeangaknmldooml";
        assert_eq!(identify_source(youtube_app, &["Résumé Foireux", "CrazyBomb World"]), MediaSource::YouTube);
        assert_eq!(app_name(youtube_app), "Google Chrome");
    }

    #[test]
    fn site_names_are_cut_from_titles() {
        assert_eq!(
            clean_title("Presque mariés. Plus qu'un fantasme - Regardez sur Crunchyroll"),
            "Presque mariés. Plus qu'un fantasme"
        );
        assert_eq!(clean_title("Pluie de nuit · lo-fi - YouTube"), "Pluie de nuit · lo-fi");
        assert_eq!(clean_title("Frieren Épisode 7 | Crunchyroll "), "Frieren Épisode 7");
        assert_eq!(clean_title("Paris la nuit"), "Paris la nuit");
        assert_eq!(clean_title(" - YouTube"), "- YouTube");
    }

    #[test]
    fn app_names_are_readable() {
        assert_eq!(app_name(SPOTIFY), "Spotify");
        assert_eq!(app_name(SPOTIFY_STORE), "Spotify");
        assert_eq!(app_name(EDGE), "Microsoft Edge");
        assert_eq!(app_name(CHROME), "Google Chrome");
        assert_eq!(app_name(FIREFOX), "Firefox");
        assert_eq!(app_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "Lecteur multimédia");
        assert_eq!(app_name("Deezer.Deezer_abc123!Deezer.App"), "Deezer");
        assert_eq!(app_name(r"C:\Program Files\Foobar\foobar2000.exe"), "foobar2000");
        assert_eq!(app_name(""), "Autre app");
    }

    #[test]
    fn browsers_are_detected() {
        assert!(is_browser(EDGE) && is_browser(CHROME) && is_browser(FIREFOX));
        assert!(!is_browser(SPOTIFY));
    }

    #[test]
    fn only_window_titles_showing_the_media_count() {
        let windows = vec![
            "Pluie de nuit · lo-fi - YouTube - Microsoft Edge".to_owned(),
            "Boîte de réception - Outlook".to_owned(),
        ];
        assert_eq!(titles_showing(&windows, "pluie de nuit · lo-fi"), vec![windows[0].as_str()]);
        assert!(titles_showing(&windows, "Autre chose").is_empty());
        assert!(titles_showing(&windows, "  ").is_empty());
    }

    #[test]
    fn video_ids_are_strict() {
        assert!(is_video_id("dQw4w9WgXcQ"));
        assert!(is_video_id("a-b_c-d_e-f"));
        assert!(!is_video_id("dQw4w9WgXc"));
        assert!(!is_video_id("dQw4w9WgXcQ&"));
        assert!(!is_video_id("<script>xx>"));
    }

    #[test]
    fn seeking_asks_for_ticks_on_the_timeline() {
        assert_eq!(seek_ticks(90.5, 0, 2_000_000_000), Some(905_000_000));
        assert_eq!(seek_ticks(10.0, 50_000_000, 0), Some(150_000_000), "from the start of the timeline");
        assert_eq!(seek_ticks(500.0, 0, 1_870_000_000), Some(1_870_000_000), "not past the end");
        assert_eq!(seek_ticks(30.0, 0, 0), Some(300_000_000), "end unknown (live)");
        assert_eq!(seek_ticks(-1.0, 0, 10), None);
        assert_eq!(seek_ticks(f64::NAN, 0, 10), None);
        assert_eq!(seek_ticks(f64::INFINITY, 0, 10), None);
    }

    #[test]
    fn windows_times_convert() {
        assert_eq!(ticks_to_seconds(1_870_000_000), 187.0);
        // 2026-10-11T00:00:00Z
        assert_eq!(universal_time_to_unix_ms(134_361_504_000_000_000), Some(1_791_676_800_000));
        assert_eq!(universal_time_to_unix_ms(0), None);
    }

    #[test]
    fn position_moves_forward_only_while_playing() {
        assert_eq!(position_at(30.0, Some(1_000), 6_000, true, 1.0, 120.0), 35.0);
        assert_eq!(position_at(30.0, Some(1_000), 6_000, false, 1.0, 120.0), 30.0);
        assert_eq!(position_at(30.0, Some(1_000), 6_000, true, 2.0, 120.0), 40.0);
        assert_eq!(position_at(30.0, None, 6_000, true, 1.0, 120.0), 30.0);
    }

    #[test]
    fn position_stays_within_the_track() {
        assert_eq!(position_at(118.0, Some(0), 10_000, true, 1.0, 120.0), 120.0);
        assert_eq!(position_at(-3.0, Some(0), 0, true, 1.0, 120.0), 0.0);
        assert_eq!(position_at(30.0, Some(9_000), 1_000, true, 1.0, 120.0), 30.0);
        // Live stream: no end.
        assert_eq!(position_at(5_000.0, Some(0), 10_000, true, 1.0, 0.0), 5_010.0);
    }

    fn spotify_info() -> SessionInfo {
        SessionInfo {
            app_id: SPOTIFY.to_owned(),
            title: "Lo-fi beats pour coder".to_owned(),
            artist: "Lofi Girl".to_owned(),
            playing: true,
            rate: 1.0,
            start: 0,
            end: 1_870_000_000,
            position: 300_000_000,
            last_updated: 134_361_504_000_000_000,
            controls: Controls { play_pause: true, next: true, previous: true, ..Controls::default() },
            artwork: Some("data:image/jpeg;base64,/9j/".to_owned()),
            ..SessionInfo::default()
        }
    }

    #[test]
    fn shuffle_and_repeat_reach_the_interface_only_when_known() {
        let plain = serde_json::to_value(to_track(&spotify_info(), MediaSource::Spotify, 0, None)).unwrap();
        assert!(plain.get("shuffle").is_none() && plain.get("repeat").is_none());
        assert_eq!(plain["controls"]["shuffle"], false);
        let info = SessionInfo {
            shuffle: Some(true),
            repeat: Some(RepeatMode::List),
            controls: Controls { shuffle: true, repeat: true, ..spotify_info().controls },
            ..spotify_info()
        };
        let json = serde_json::to_value(to_track(&info, MediaSource::Spotify, 0, None)).unwrap();
        assert_eq!((json["shuffle"].clone(), json["repeat"].clone()), (true.into(), "list".into()));
        assert_eq!(json["controls"]["repeat"], true);
        assert_eq!(serde_json::from_str::<RepeatMode>("\"track\"").unwrap(), RepeatMode::Track);
        for mode in [RepeatMode::None, RepeatMode::Track, RepeatMode::List] {
            assert_eq!(RepeatMode::from_windows(mode.to_windows()), Some(mode));
        }
        assert_eq!(RepeatMode::from_windows(MediaPlaybackAutoRepeatMode(7)), None);
    }

    #[test]
    fn smtc_info_becomes_a_track() {
        let now = 1_791_676_800_000 + 2_000;
        let track = to_track(&spotify_info(), MediaSource::Spotify, now, None);
        assert_eq!(track.title, "Lo-fi beats pour coder");
        assert_eq!(track.app, "Spotify");
        assert_eq!(track.app_id, SPOTIFY);
        assert_eq!(track.duration, 187.0);
        assert_eq!(track.position, 32.0);
        assert_eq!(track.updated_at, now);
        assert!(track.playing && track.controls.next);
        assert_eq!(track.id, "Spotify.exe\u{1f}Lo-fi beats pour coder\u{1f}Lofi Girl");
    }

    #[test]
    fn timeline_with_an_offset_start_and_no_update_time() {
        let info = SessionInfo {
            start: 100_000_000,
            end: 400_000_000,
            position: 150_000_000,
            last_updated: 0,
            ..spotify_info()
        };
        let track = to_track(&info, MediaSource::Spotify, 99, None);
        assert_eq!((track.position, track.duration), (5.0, 30.0));
    }

    #[test]
    fn own_player_track_carries_its_video() {
        let info = SessionInfo { app_id: "ilot.exe".to_owned(), ..spotify_info() };
        let track = to_track(&info, MediaSource::YouTube, 0, Some("dQw4w9WgXcQ".to_owned()));
        assert_eq!(track.app, "îlot");
        assert_eq!(track.video_id.as_deref(), Some("dQw4w9WgXcQ"));
        assert!(crate::video::is_own_session("ilot.exe") && !crate::video::is_own_session(SPOTIFY));
    }

    #[test]
    fn track_serializes_for_the_interface() {
        let track = to_track(&spotify_info(), MediaSource::Spotify, 0, None);
        let json = serde_json::to_value(&track).unwrap();
        assert_eq!(json["source"], "Spotify");
        assert_eq!(json["appId"], SPOTIFY);
        assert_eq!(json["controls"]["playPause"], true);
        assert!(json.get("videoId").is_none());
        let other = to_track(&spotify_info(), MediaSource::Other, 0, None);
        assert_eq!(serde_json::to_value(&other).unwrap()["source"], "other");
    }

    #[test]
    fn actions_follow_the_contract() {
        for (text, action) in [
            ("play", MediaAction::Play),
            ("pause", MediaAction::Pause),
            ("toggle", MediaAction::Toggle),
            ("next", MediaAction::Next),
            ("previous", MediaAction::Previous),
        ] {
            assert_eq!(serde_json::from_str::<MediaAction>(&format!("\"{text}\"")).unwrap(), action);
        }
        assert!(serde_json::from_str::<MediaAction>("\"stop\"").is_err());
    }

    fn track(app_id: &str, title: &str, playing: bool) -> Track {
        let info = SessionInfo { app_id: app_id.to_owned(), title: title.to_owned(), playing, ..spotify_info() };
        to_track(&info, identify_source(app_id, &[title]), 0, None)
    }

    #[test]
    fn current_is_the_system_choice_then_what_plays() {
        let tracks = vec![track(EDGE, "Vidéo - YouTube", false), track(SPOTIFY, "Chanson", true)];
        assert_eq!(pick_current(&tracks, Some(EDGE)).unwrap().app_id, EDGE);
        assert_eq!(pick_current(&tracks, Some("Gone")).unwrap().app_id, SPOTIFY);
        assert_eq!(pick_current(&tracks, None).unwrap().app_id, SPOTIFY);
        assert!(pick_current(&[], None).is_none());
    }

    #[test]
    fn history_keeps_played_tracks_newest_first() {
        let mut recent = Vec::new();
        remember(&mut recent, &track(SPOTIFY, "Un", true));
        remember(&mut recent, &track(SPOTIFY, "Deux", true));
        remember(&mut recent, &track(SPOTIFY, "Un", true));
        remember(&mut recent, &track(SPOTIFY, "En pause", false));
        remember(&mut recent, &track(SPOTIFY, " ", true));
        let titles: Vec<_> = recent.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["Un", "Deux"]);
        for n in 0..20 {
            remember(&mut recent, &track(SPOTIFY, &format!("Titre {n}"), true));
        }
        assert_eq!(recent.len(), RECENT_MAX);
        assert_eq!(recent[0].title, "Titre 19");
    }

    #[test]
    fn small_time_moves_are_not_resent() {
        let a = track(SPOTIFY, "Chanson", true);
        let on_time = Track { position: a.position + 10.0, updated_at: a.updated_at + 10_000, ..a.clone() };
        let seeked = Track { position: a.position + 60.0, updated_at: a.updated_at + 10_000, ..a.clone() };
        let paused = Track { playing: false, ..a.clone() };
        assert!(!needs_emit(Some(&a), Some(&on_time)));
        assert!(needs_emit(Some(&a), Some(&seeked)));
        assert!(needs_emit(Some(&a), Some(&paused)));
        assert!(needs_emit(None, Some(&a)) && needs_emit(Some(&a), None));
        assert!(!needs_emit(None, None));
        assert!(lists_differ(std::slice::from_ref(&a), &[]));
        assert!(!lists_differ(std::slice::from_ref(&a), &[on_time]));
    }

    /// Reads the real sessions of this PC without playing or changing anything.
    /// `cargo test -p ilot -- --ignored --nocapture media::tests::reads_real_sessions`
    #[test]
    #[ignore = "needs Windows media sessions"]
    fn reads_real_sessions() {
        let manager = SessionManager::RequestAsync().unwrap().join().expect("SMTC access");
        let sessions: Vec<Session> = manager.GetSessions().unwrap().into_iter().collect();
        println!("{} session(s)", sessions.len());
        for session in sessions {
            let app_id = session.SourceAppUserModelId().unwrap().to_string_lossy();
            let w = Watched { app_id: app_id.clone(), session, tokens: [0; 3], properties: None };
            let properties = read_properties(&w.session, &app_id, None);
            let info = read_info(&w, &properties);
            let track = to_track(&info, properties.source, now_ms(), None);
            let artwork = track.artwork.as_ref().map_or(0, String::len);
            println!(
                "{app_id}: {:?} « {} » de « {} » ({}, en lecture : {}) {:.0}/{:.0} s, {:?}, pochette {artwork} octets",
                track.source,
                track.title,
                track.artist,
                track.app,
                track.playing,
                track.position,
                track.duration,
                track.controls
            );
        }
    }

    #[test]
    fn app_names_follow_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(app_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "Media Player");
        assert_eq!(app_name(""), "Other app");
        assert_eq!(app_name(SPOTIFY), "Spotify");
    }
}
