//! The videos îlot shows in windows of its own, and « Détacher la vidéo »
//! (docs/09-DEMO-FINALE.md, section 5 bis). The rule: a video is always shown by a player
//! that has the right to play it, never by a copy of its picture, never around its DRM.
//!
//! - Two site windows: `youtube` (îlot's page holding the official embed, players.rs) and
//!   `crunchyroll` (the site itself, where the user signs in; its DRM decides whether the
//!   episode plays). Both live in a WebView2 profile of their own (the `players` data
//!   folder, apart from the island's), have no capability, can only show their site, open
//!   no other window, download nothing and get no device permission.
//! - Detaching turns that window into a small frameless player, always on top, with
//!   rounded corners, which keeps playing. Nothing can be added to a remote page, so its
//!   commands live in a small local window laid over its bottom edge (`<site>-bar`,
//!   capabilities/video-bar.json, video-bar.html). « Ramener » hides the player into the
//!   island (the sound goes on, the island shows it); « Fermer » closes it.
//! - A video playing elsewhere: Crunchyroll in a browser with îlot's extension asks the
//!   browser for its own picture-in-picture (native_host.rs); else the window that plays,
//!   a browser or the Crunchyroll app, is made small and kept on top (on_top.rs).

use crate::locale::{Text, tr};
use crate::media::{self, MediaHub, MediaSource, Track};
use crate::native_host::{self, Extension, PipAnswer, Refusal};
use crate::on_top::{self, Lifted};
use crate::players::{self, Area, Slot, VIDEO_LABEL};
use crate::settings::{SettingsStore, WindowPosition};
use crate::source_app;
use crate::window::{IslandWindow, WINDOW_LABEL};
use serde::Serialize;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::webview::{NewWindowResponse, PermissionKind, PermissionResponse};
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, Listener, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Size, Url, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, Window, WindowEvent,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, DeferWindowPos, EndDeferWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
};

pub const CRUNCHYROLL_LABEL: &str = "crunchyroll";
pub const YOUTUBE_BAR: &str = "youtube-bar";
pub const CRUNCHYROLL_BAR: &str = "crunchyroll-bar";
pub const EVENT_STATE: &str = "video:state";
/// Where the Crunchyroll window opens the first time.
const CRUNCHYROLL_HOME: &str = "https://www.crunchyroll.com/";
/// Height of the commands bar laid over the bottom edge of the small player (logical).
const BAR_HEIGHT: f64 = 64.0;
/// Full windows, logical sizes. YouTube embeds must stay at least 200 × 200 px
/// (https://developers.google.com/youtube/terms/required-minimum-functionality).
const YOUTUBE_SIZE: (f64, f64) = (480.0, 270.0);
const YOUTUBE_MIN: (f64, f64) = (356.0, 200.0);
const CRUNCHYROLL_SIZE: (f64, f64) = (1100.0, 700.0);
const CRUNCHYROLL_MIN: (f64, f64) = (640.0, 400.0);
/// The small player, 16:9: 352 × 198 from the demo; YouTube's keeps its 200 px minimum.
const COMPACT: (f64, f64) = (352.0, 198.0);
const COMPACT_YOUTUBE: (f64, f64) = (356.0, 200.0);
/// Gap between the small player and the screen edges (logical).
const MARGIN: f64 = 24.0;
/// The flight into the island and back out: short, on an ease-out.
const FLIGHT: Duration = Duration::from_millis(240);
const FLIGHT_STEPS: u32 = 14;
/// Size of the player as it melts into the island, as a share of its own.
const DROP_RATIO: f64 = 0.2;
/// Farthest a move may ask (logical pixels).
const FARTHEST: f64 = 100_000.0;

const INVALID_VIDEO: Text<'static> = Text::VideoInvalidLink;
const NO_VIDEO_WINDOW: Text<'static> = Text::VideoNoWindow;
const NOTHING_PLAYING: Text<'static> = Text::MediaNothingPlaying;
const NOT_A_BAR: Text<'static> = Text::VideoNotABar;
const INVALID_MOVE: Text<'static> = Text::MoveInvalid;

// ---------------------------------------------------------------------------------------
// Sites (pure, tested)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    YouTube,
    Crunchyroll,
}

impl Site {
    const ALL: [Site; 2] = [Site::YouTube, Site::Crunchyroll];

    fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Site::YouTube => VIDEO_LABEL,
            Site::Crunchyroll => CRUNCHYROLL_LABEL,
        }
    }

    pub fn bar(self) -> &'static str {
        match self {
            Site::YouTube => YOUTUBE_BAR,
            Site::Crunchyroll => CRUNCHYROLL_BAR,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Site::YouTube => "youtube",
            Site::Crunchyroll => "crunchyroll",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Site::YouTube => "YouTube · îlot",
            Site::Crunchyroll => "Crunchyroll · îlot",
        }
    }

    fn full_size(self) -> ((f64, f64), (f64, f64)) {
        match self {
            Site::YouTube => (YOUTUBE_SIZE, YOUTUBE_MIN),
            Site::Crunchyroll => (CRUNCHYROLL_SIZE, CRUNCHYROLL_MIN),
        }
    }

    pub fn compact_size(self) -> (f64, f64) {
        match self {
            Site::YouTube => COMPACT_YOUTUBE,
            Site::Crunchyroll => COMPACT,
        }
    }

    pub fn of_label(label: &str) -> Option<Site> {
        Site::ALL.into_iter().find(|site| site.label() == label)
    }

    pub fn of_bar(label: &str) -> Option<Site> {
        Site::ALL.into_iter().find(|site| site.bar() == label)
    }
}

/// The Crunchyroll window may only show Crunchyroll, over HTTPS, on its usual port.
pub fn is_crunchyroll_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.host_str().is_some_and(|host| host == "crunchyroll.com" || host.ends_with(".crunchyroll.com"))
}

/// Where a site window may go (its main frame: the site's own frames are its business).
pub fn allows(site: Site, url: &Url) -> bool {
    match site {
        Site::YouTube => players::is_player_url(url),
        Site::Crunchyroll => is_crunchyroll_url(url),
    }
}

/// Remote pages get no device: autoplay follows WebView2's own rule, the rest is refused.
pub fn permission_for(kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Autoplay => PermissionResponse::Default,
        _ => PermissionResponse::Deny,
    }
}

/// Sessions of îlot's own windows: Windows names them after îlot or after WebView2.
pub fn is_own_session(app_id: &str) -> bool {
    let id = app_id.to_lowercase();
    id.contains("ilot") || id.contains("msedgewebview2")
}

// ---------------------------------------------------------------------------------------
// Geometry (pure, tested): physical pixels

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The bar covers the bottom edge of the player, as wide as it.
pub fn bar_frame(player: Frame, bar_height: f64) -> Frame {
    Frame { x: player.x, y: player.y + player.height - bar_height, width: player.width, height: bar_height }
}

/// Where the player goes when its bar is dragged to `(x, y)`.
pub fn player_for_bar(x: f64, y: f64, player_height: f64, bar_height: f64) -> (f64, f64) {
    (x, y + bar_height - player_height)
}

/// The small player goes back where it was on this run if its top is still on a
/// screen, or else to the bottom right corner of the screen it is on.
pub fn compact_place(size: (i32, i32), last: Option<WindowPosition>, areas: &[Area], home: Area, margin: i32) -> WindowPosition {
    match last {
        Some(position) if players::is_reachable(position, size.0, areas) => position,
        _ => players::bottom_right(home, size, margin, 0),
    }
}

/// A small frame centered on the island, where the player melts.
pub fn drop_frame(island: (f64, f64), size: (f64, f64), ratio: f64) -> Frame {
    let (width, height) = (size.0 * ratio, size.1 * ratio);
    Frame { x: island.0 - width / 2.0, y: island.1 - height / 2.0, width, height }
}

/// The frames of a flight, on an ease-out (cubic): fast at first, settling at the end.
pub fn flight(from: Frame, to: Frame, steps: u32) -> Vec<Frame> {
    let steps = steps.max(1);
    (1..=steps)
        .map(|step| {
            if step == steps {
                return to;
            }
            let t = f64::from(step) / f64::from(steps);
            let eased = 1.0 - (1.0 - t).powi(3);
            let mix = |a: f64, b: f64| a + (b - a) * eased;
            Frame {
                x: mix(from.x, to.x),
                y: mix(from.y, to.y),
                width: mix(from.width, to.width),
                height: mix(from.height, to.height),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Which way to detach (pure, tested)

/// How a video comes off the island.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// îlot's own window turns into the small player.
    Window(Site),
    /// The browser floats the video itself (îlot's extension).
    PictureInPicture,
    /// The window that plays is made small and kept on top.
    OnTop,
    /// Not a video îlot can show: the music mini-player.
    MiniPlayer,
}

/// What îlot knows about the session to detach.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub source: Option<MediaSource>,
    /// The session belongs to one of îlot's own windows.
    pub own: bool,
    /// îlot's YouTube window plays it (the core knows its video).
    pub youtube_video: bool,
    /// Which site windows are open, in `Site::ALL` order.
    pub open: [bool; 2],
    pub browser: bool,
    /// A browser or a Store app: a window îlot can find and lift.
    pub windowed: bool,
    /// îlot's extension is connected and a Crunchyroll tab holds a video.
    pub extension: bool,
}

pub fn choose(facts: &Facts) -> Way {
    let site = match facts.source {
        Some(MediaSource::YouTube) => Some(Site::YouTube),
        Some(MediaSource::Crunchyroll) => Some(Site::Crunchyroll),
        _ => None,
    };
    if facts.own {
        // Never a browser for îlot's own windows, even though WebView2 looks like Edge.
        let mut open = Site::ALL.into_iter().filter(|site| facts.open[site.index()]);
        let only = match (open.next(), open.next()) {
            (Some(site), None) => Some(site),
            _ => None,
        };
        let own = if facts.youtube_video { Some(Site::YouTube) } else { site.or(only) };
        return match own {
            Some(site) if facts.open[site.index()] => Way::Window(site),
            _ => Way::MiniPlayer,
        };
    }
    match site {
        Some(Site::Crunchyroll) if facts.browser && facts.extension => Way::PictureInPicture,
        Some(_) if facts.windowed => Way::OnTop,
        _ => Way::MiniPlayer,
    }
}

/// What « Détacher la vidéo » did, for the interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Window,
    PictureInPicture,
    /// The browser waits for a key press (the extension's shortcut) before it floats.
    NeedsGesture,
    OnTop,
    MiniPlayer,
}

// ---------------------------------------------------------------------------------------
// Shared state and what the interface sees of it

enum Detached {
    Window(Site),
    PictureInPicture,
    OnTop(Lifted),
}

#[derive(Default)]
struct Videos {
    detached: Option<Detached>,
    /// Site windows turned into the small player.
    compact: [bool; 2],
    /// Where each site window was, and how large, before it became small.
    full: [Option<Frame>; 2],
    /// Where the small player was last on this run.
    compact_at: Option<WindowPosition>,
    /// Bumped by each flight: an older flight that ends does nothing.
    flight: u64,
    flying: bool,
    /// Media events last relayed to the bars, by event: never the same one twice.
    relayed: [Option<String>; 2],
}

#[derive(Default)]
pub struct VideoHub(Mutex<Videos>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DetachedKind {
    Window,
    PictureInPicture,
    OnTop,
}

/// Payload of `video:state` and of `video_state` (type `VideoState` in the interface).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoState {
    pub detached: Option<DetachedKind>,
    /// The site window that is out, for `window`.
    pub site: Option<Site>,
    /// îlot's browser extension is connected.
    pub extension: bool,
}

fn state_of(app: &AppHandle) -> VideoState {
    let (detached, site) = match &app.state::<VideoHub>().0.lock().unwrap().detached {
        None => (None, None),
        Some(Detached::Window(site)) => (Some(DetachedKind::Window), Some(*site)),
        Some(Detached::PictureInPicture) => (Some(DetachedKind::PictureInPicture), None),
        Some(Detached::OnTop(_)) => (Some(DetachedKind::OnTop), None),
    };
    VideoState { detached, site, extension: app.state::<Extension>().connected() }
}

/// Tells the island and the bars where the video is.
fn publish(app: &AppHandle) {
    let state = state_of(app);
    for label in [WINDOW_LABEL, YOUTUBE_BAR, CRUNCHYROLL_BAR] {
        let _ = app.emit_to(label, EVENT_STATE, &state);
    }
}

fn set_detached(app: &AppHandle, detached: Option<Detached>) {
    app.state::<VideoHub>().0.lock().unwrap().detached = detached;
}

fn is_compact(app: &AppHandle, site: Site) -> bool {
    app.state::<VideoHub>().0.lock().unwrap().compact[site.index()]
}

/// The extension came, left, or the browser floated or docked the video by itself (its
/// shortcut, its own button): the island follows.
pub fn extension_changed(app: &AppHandle, pip_changed: bool) {
    if pip_changed {
        let in_pip = app.state::<Extension>().in_pip();
        let hub = app.state::<VideoHub>();
        let mut hub = hub.0.lock().unwrap();
        match (&hub.detached, in_pip) {
            (Some(Detached::PictureInPicture), false) => hub.detached = None,
            (None, true) => hub.detached = Some(Detached::PictureInPicture),
            _ => {}
        }
    }
    publish(app);
}

// ---------------------------------------------------------------------------------------
// Windows

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn hwnd_of(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|hwnd| HWND(hwnd.0))
}

/// The site windows' own WebView2 profile, apart from the island's.
fn players_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_local_data_dir().map(|dir| dir.join("players")).map_err(error)
}

fn area(monitor: &tauri::Monitor) -> Area {
    let work = monitor.work_area();
    Area { x: work.position.x, y: work.position.y, width: work.size.width as i32, height: work.size.height as i32 }
}

/// Windows 11 rounds the corners of the small player and keeps its shadow
/// (https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners).
/// Windows 10 refuses the attribute: square corners, and no shadow, which a frameless
/// window would draw as a white line.
fn round_corners(window: &WebviewWindow) -> Result<(), String> {
    let Some(hwnd) = hwnd_of(window) else { return Ok(()) };
    let preference = DWMWCP_ROUND;
    // SAFETY: the attribute is a DWM_WINDOW_CORNER_PREFERENCE read from a local of that size.
    let rounded = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const preference).cast::<c_void>(),
            size_of_val(&preference) as u32,
        )
    };
    window.set_shadow(rounded.is_ok()).map_err(error)
}

/// Moves the player and its bar in one go, so that they never part.
fn move_together(player: HWND, bar: Option<HWND>, at: (i32, i32), bar_at: (i32, i32)) -> windows::core::Result<()> {
    let flags = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
    // SAFETY: both handles belong to îlot's live windows; the batch handle is consumed by
    // EndDeferWindowPos (or released by the system when a step fails).
    unsafe {
        let mut batch = BeginDeferWindowPos(2)?;
        batch = DeferWindowPos(batch, player, None, at.0, at.1, 0, 0, flags)?;
        if let Some(bar) = bar {
            batch = DeferWindowPos(batch, bar, None, bar_at.0, bar_at.1, 0, 0, flags)?;
        }
        EndDeferWindowPos(batch)
    }
}

fn frame_of(window: &WebviewWindow) -> Result<Frame, String> {
    let at = window.outer_position().map_err(error)?;
    let size = window.outer_size().map_err(error)?;
    Ok(Frame { x: f64::from(at.x), y: f64::from(at.y), width: f64::from(size.width), height: f64::from(size.height) })
}

fn set_frame(window: &WebviewWindow, frame: Frame) {
    let _ = window.set_size(PhysicalSize::new(frame.width.round() as u32, frame.height.round() as u32));
    let _ = window.set_position(PhysicalPosition::new(frame.x.round() as i32, frame.y.round() as i32));
}

/// Center of the folded island on the screen, when it is placed.
fn island_center(app: &AppHandle) -> Option<(f64, f64)> {
    let pill = *app.state::<IslandWindow>().shapes_on_screen().first()?;
    Some((f64::from(pill.left + pill.right) / 2.0, f64::from(pill.top + pill.bottom) / 2.0))
}

/// Opens a site window (hidden), or sends the existing one to `url`.
fn open_site(app: &AppHandle, site: Site, url: Url) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(site.label()) {
        window.navigate(url).map_err(error)?;
        return Ok(window);
    }
    let (size, min) = site.full_size();
    let builder = WebviewWindowBuilder::new(app, site.label(), WebviewUrl::External(url))
        .title(site.title())
        .inner_size(size.0, size.1)
        .min_inner_size(min.0, min.1)
        .always_on_top(site == Site::YouTube)
        .visible(false)
        // YouTube leaves nothing behind; Crunchyroll keeps the user signed in, in its own profile.
        .incognito(site == Site::YouTube)
        .data_directory(players_dir(app)?)
        .use_https_scheme(true)
        .on_navigation(move |url| allows(site, url))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .on_permission_request(|_, kind| permission_for(kind))
        .background_color(Color(0, 0, 0, 255));
    let window = match site {
        Site::YouTube => builder.build().map_err(error)?,
        Site::Crunchyroll => builder.center().build().map_err(error)?,
    };
    if site == Site::YouTube {
        let saved = app.state::<SettingsStore>().get().video_player;
        players::place(&window, saved, players::MINI_SIZE.0 + MARGIN).map_err(error)?;
    }
    Ok(window)
}

/// Shows a site window at its full size, in front.
pub fn show_full(app: &AppHandle, site: Site) -> Result<(), String> {
    let window = app.get_webview_window(site.label()).ok_or(NO_VIDEO_WINDOW)?;
    if is_compact(app, site) {
        leave_compact(app, site, &window)?;
    }
    {
        let hub = app.state::<VideoHub>();
        let mut hub = hub.0.lock().unwrap();
        if matches!(hub.detached, Some(Detached::Window(s)) if s == site) {
            hub.detached = None;
        }
    }
    window.unminimize().map_err(error)?;
    window.show().map_err(error)?;
    window.set_focus().map_err(error)?;
    publish(app);
    Ok(())
}

/// Back to a normal window, as large and where it was before.
fn leave_compact(app: &AppHandle, site: Site, window: &WebviewWindow) -> Result<(), String> {
    if let Some(bar) = app.get_webview_window(site.bar()) {
        bar.hide().map_err(error)?;
    }
    let full = {
        let hub = app.state::<VideoHub>();
        let mut hub = hub.0.lock().unwrap();
        hub.compact[site.index()] = false;
        hub.full[site.index()].take()
    };
    let (size, min) = site.full_size();
    window.set_decorations(true).map_err(error)?;
    window.set_shadow(true).map_err(error)?;
    window.set_resizable(true).map_err(error)?;
    window.set_skip_taskbar(false).map_err(error)?;
    window.set_always_on_top(site == Site::YouTube).map_err(error)?;
    window.set_min_size(Some(LogicalSize::new(min.0, min.1))).map_err(error)?;
    match full {
        Some(frame) => {
            window.set_size(PhysicalSize::new(frame.width as u32, frame.height as u32)).map_err(error)?;
            window.set_position(PhysicalPosition::new(frame.x as i32, frame.y as i32)).map_err(error)
        }
        None => window.set_size(LogicalSize::new(size.0, size.1)).map_err(error),
    }
}

/// Turns the site window into the small player (not shown yet): frameless, always on
/// top, out of the taskbar, rounded. Returns where it goes.
fn enter_compact(app: &AppHandle, site: Site, window: &WebviewWindow) -> Result<Frame, String> {
    let (width, height) = site.compact_size();
    let scale = window.scale_factor().map_err(error)?;
    let size = ((width * scale).round() as i32, (height * scale).round() as i32);
    let last = {
        let hub = app.state::<VideoHub>();
        let mut hub = hub.0.lock().unwrap();
        if !hub.compact[site.index()] {
            let at = window.outer_position().map_err(error)?;
            let inner = window.inner_size().map_err(error)?;
            hub.full[site.index()] =
                Some(Frame { x: f64::from(at.x), y: f64::from(at.y), width: f64::from(inner.width), height: f64::from(inner.height) });
        }
        hub.compact[site.index()] = true;
        hub.compact_at
    };
    window.set_min_size(None::<Size>).map_err(error)?;
    window.set_decorations(false).map_err(error)?;
    window.set_resizable(false).map_err(error)?;
    window.set_always_on_top(true).map_err(error)?;
    window.set_skip_taskbar(true).map_err(error)?;
    window.set_size(LogicalSize::new(width, height)).map_err(error)?;
    round_corners(window)?;
    let areas: Vec<Area> = window.available_monitors().map_err(error)?.iter().map(area).collect();
    let home = window
        .current_monitor()
        .map_err(error)?
        .or(window.primary_monitor().map_err(error)?)
        .map(|monitor| area(&monitor))
        .ok_or(NO_VIDEO_WINDOW)?;
    let at = compact_place(size, last, &areas, home, (MARGIN * scale).round() as i32);
    window.set_position(PhysicalPosition::new(at.x, at.y)).map_err(error)?;
    app.state::<VideoHub>().0.lock().unwrap().compact_at = Some(at);
    Ok(Frame { x: f64::from(at.x), y: f64::from(at.y), width: f64::from(size.0), height: f64::from(size.1) })
}

/// The commands bar of a small player: îlot's own page, owned by the player window (so
/// always above it, and gone with it).
fn bar_of(app: &AppHandle, site: Site, player: &WebviewWindow) -> Result<WebviewWindow, String> {
    if let Some(bar) = app.get_webview_window(site.bar()) {
        return Ok(bar);
    }
    let (width, _) = site.compact_size();
    WebviewWindowBuilder::new(app, site.bar(), WebviewUrl::App(format!("video-bar.html?site={}", site.key()).into()))
        .title(tr(Text::VideoBarTitle))
        .owner(player)
        .map_err(error)?
        .inner_size(width, BAR_HEIGHT)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .build()
        .map_err(error)
}

fn place_bar(bar: &WebviewWindow, player: Frame) -> Result<(), String> {
    let scale = bar.scale_factor().map_err(error)?;
    let frame = bar_frame(player, (BAR_HEIGHT * scale).round());
    bar.set_size(PhysicalSize::new(frame.width.round() as u32, frame.height.round() as u32)).map_err(error)?;
    bar.set_position(PhysicalPosition::new(frame.x.round() as i32, frame.y.round() as i32)).map_err(error)
}

/// Runs a flight on a thread of its own, then `land` (if no newer flight started).
fn fly(app: &AppHandle, window: WebviewWindow, frames: Vec<Frame>, land: impl FnOnce(&AppHandle) + Send + 'static) {
    let generation = {
        let hub = app.state::<VideoHub>();
        let mut hub = hub.0.lock().unwrap();
        hub.flight += 1;
        hub.flying = true;
        hub.flight
    };
    let pilot = app.clone();
    let step = FLIGHT / FLIGHT_STEPS;
    let spawned = std::thread::Builder::new().name("ilot-video-flight".into()).spawn(move || {
        for frame in frames {
            set_frame(&window, frame);
            std::thread::sleep(step);
        }
        let app = pilot;
        let current = {
            let hub = app.state::<VideoHub>();
            let mut hub = hub.0.lock().unwrap();
            let current = hub.flight == generation;
            if current {
                hub.flying = false;
            }
            current
        };
        if current {
            land(&app);
        }
    });
    if spawned.is_err() {
        app.state::<VideoHub>().0.lock().unwrap().flying = false;
    }
}

/// Shows the small player, flying out of the island when it comes from there.
fn show_compact(app: &AppHandle, site: Site, animate: bool) -> Result<(), String> {
    let window = app.get_webview_window(site.label()).ok_or(NO_VIDEO_WINDOW)?;
    let was_shown = window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false);
    let frame = enter_compact(app, site, &window)?;
    let bar = bar_of(app, site, &window)?;
    place_bar(&bar, frame)?;
    set_detached(app, Some(Detached::Window(site)));
    publish(app);
    match island_center(app).filter(|_| animate && !was_shown) {
        Some(island) => {
            let start = drop_frame(island, (frame.width, frame.height), DROP_RATIO);
            set_frame(&window, start);
            window.show().map_err(error)?;
            fly(app, window, flight(start, frame, FLIGHT_STEPS), move |_| {
                let _ = bar.show();
            });
        }
        None => {
            window.unminimize().map_err(error)?;
            window.show().map_err(error)?;
            bar.show().map_err(error)?;
        }
    }
    Ok(())
}

/// Hides the small player into the island: it keeps playing, the island shows it.
fn dock_window(app: &AppHandle, site: Site, animate: bool) {
    let Some(window) = app.get_webview_window(site.label()) else { return };
    if let Some(bar) = app.get_webview_window(site.bar()) {
        let _ = bar.hide();
    }
    let visible = window.is_visible().unwrap_or(false);
    let from = frame_of(&window).ok();
    match (island_center(app), from) {
        (Some(island), Some(from)) if animate && visible => {
            let to = drop_frame(island, (from.width, from.height), DROP_RATIO);
            let parked = window.clone();
            fly(app, window, flight(from, to, FLIGHT_STEPS), move |_| {
                let _ = parked.hide();
                set_frame(&parked, from);
            });
        }
        _ => {
            let _ = window.hide();
        }
    }
}

fn dock(app: &AppHandle, animate: bool) {
    let detached = app.state::<VideoHub>().0.lock().unwrap().detached.take();
    match detached {
        Some(Detached::Window(site)) => dock_window(app, site, animate),
        Some(Detached::PictureInPicture) => native_host::exit_pip(app),
        Some(Detached::OnTop(lifted)) => on_top::put_back(&lifted),
        None => {}
    }
    publish(app);
}

fn lift(app: &AppHandle, track: Option<&Track>) -> Result<Outcome, String> {
    let track = track.ok_or(NOTHING_PLAYING)?;
    let lifted = on_top::lift(&track.app_id, &track.title, &media::app_name(&track.app_id))?;
    set_detached(app, Some(Detached::OnTop(lifted)));
    publish(app);
    Ok(Outcome::OnTop)
}

fn facts(app: &AppHandle, app_id: &str, track: Option<&Track>) -> Facts {
    Facts {
        source: track.map(|t| t.source),
        own: is_own_session(app_id),
        youtube_video: track.is_some_and(|t| t.video_id.is_some()),
        open: Site::ALL.map(|site| app.get_webview_window(site.label()).is_some()),
        browser: media::is_browser(app_id),
        windowed: media::is_browser(app_id) || source_app::is_packaged(app_id),
        extension: app.state::<Extension>().ready(),
    }
}

// ---------------------------------------------------------------------------------------
// Commands

/// Plays a YouTube video in îlot's YouTube window (opened on first use).
#[tauri::command]
pub async fn play_youtube(video_id: String, start: Option<u32>, app: AppHandle) -> Result<(), String> {
    if !media::is_video_id(&video_id) {
        return Err(INVALID_VIDEO.into());
    }
    let url = Url::parse(&players::player_url(&video_id, start.unwrap_or(0).min(players::MAX_START))).map_err(error)?;
    app.state::<MediaHub>().set_video(Some(video_id));
    open_site(&app, Site::YouTube, url)?;
    if is_compact(&app, Site::YouTube) {
        dock_others(&app, Site::YouTube);
        show_compact(&app, Site::YouTube, false)
    } else {
        show_full(&app, Site::YouTube)
    }
}

/// Opens îlot's Crunchyroll window, where the user signs in and picks an episode.
#[tauri::command]
pub async fn open_crunchyroll(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window(CRUNCHYROLL_LABEL).is_none() {
        open_site(&app, Site::Crunchyroll, Url::parse(CRUNCHYROLL_HOME).map_err(error)?)?;
    }
    show_full(&app, Site::Crunchyroll)
}

/// One video out at a time: whatever else is out goes back first.
fn dock_others(app: &AppHandle, keep: Site) {
    let other = match &app.state::<VideoHub>().0.lock().unwrap().detached {
        None => false,
        Some(Detached::Window(site)) => *site != keep,
        Some(_) => true,
    };
    if other {
        dock(app, false);
    }
}

/// « Détacher la vidéo »: the session's video goes into a small player that has the
/// right to play it (îlot's window, the browser's own, or the app's window kept on top).
#[tauri::command]
pub async fn detach_video(app_id: String, animate: bool, app: AppHandle) -> Result<Outcome, String> {
    let track = media::get_media(app.state::<MediaHub>()).sessions.into_iter().find(|t| t.app_id == app_id);
    let way = choose(&facts(&app, &app_id, track.as_ref()));
    match way {
        Way::Window(site) => {
            dock_others(&app, site);
            show_compact(&app, site, animate)?;
            Ok(Outcome::Window)
        }
        Way::PictureInPicture => {
            dock(&app, false);
            match native_host::request_pip(&app).await {
                Ok(PipAnswer::Shown) => {
                    set_detached(&app, Some(Detached::PictureInPicture));
                    publish(&app);
                    Ok(Outcome::PictureInPicture)
                }
                Ok(PipAnswer::Refused(Refusal::Gesture)) => Ok(Outcome::NeedsGesture),
                // Turned off by the site, no video found, no answer: the window instead.
                _ => lift(&app, track.as_ref()),
            }
        }
        Way::OnTop => {
            dock(&app, false);
            lift(&app, track.as_ref())
        }
        Way::MiniPlayer => {
            players::open_mini_player(app).await?;
            Ok(Outcome::MiniPlayer)
        }
    }
}

/// « Ramener dans l'île »: the video goes back where it came from.
#[tauri::command]
pub async fn dock_video(animate: bool, app: AppHandle) {
    dock(&app, animate);
}

#[tauri::command]
pub fn video_state(app: AppHandle) -> VideoState {
    state_of(&app)
}

/// The bar is being dragged: its player follows, both in one move.
#[tauri::command]
pub fn video_bar_move(x: f64, y: f64, window: WebviewWindow) -> Result<(), String> {
    let site = Site::of_bar(window.label()).ok_or(NOT_A_BAR)?;
    if !x.is_finite() || !y.is_finite() || x.abs() > FARTHEST || y.abs() > FARTHEST {
        return Err(INVALID_MOVE.into());
    }
    let app = window.app_handle();
    let player = app.get_webview_window(site.label()).ok_or(NO_VIDEO_WINDOW)?;
    if !is_compact(app, site) {
        return Ok(());
    }
    let scale = window.scale_factor().map_err(error)?;
    let bar_height = f64::from(window.outer_size().map_err(error)?.height);
    let player_height = f64::from(player.outer_size().map_err(error)?.height);
    let (bar_x, bar_y) = ((x * scale).round(), (y * scale).round());
    let (left, top) = player_for_bar(bar_x, bar_y, player_height, bar_height);
    let at = (left as i32, top as i32);
    let (Some(player_hwnd), bar_hwnd) = (hwnd_of(&player), hwnd_of(&window)) else { return Err(NO_VIDEO_WINDOW.into()) };
    move_together(player_hwnd, bar_hwnd, at, (bar_x as i32, bar_y as i32)).map_err(error)?;
    app.state::<VideoHub>().0.lock().unwrap().compact_at = Some(WindowPosition { x: at.0, y: at.1 });
    Ok(())
}

/// « Fermer »: the bar closes its player (and itself with it).
#[tauri::command]
pub async fn close_video(window: WebviewWindow) -> Result<(), String> {
    let site = Site::of_bar(window.label()).ok_or(NOT_A_BAR)?;
    if let Some(player) = window.app_handle().get_webview_window(site.label()) {
        player.destroy().map_err(error)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Setup and window events

/// Relays the media events to the bars (media.rs speaks to the island and the
/// mini-player only): the bar shows the title and play-pause of its own session.
pub fn setup(app: &AppHandle) {
    for (slot, event) in [media::EVENT_CHANGED, media::EVENT_SESSIONS].into_iter().enumerate() {
        let handle = app.clone();
        app.listen_any(event, move |emitted| relay(&handle, slot, event, emitted.payload()));
    }
}

fn relay(app: &AppHandle, slot: usize, event: &str, payload: &str) {
    let bars: Vec<WebviewWindow> = Site::ALL.into_iter().filter_map(|site| app.get_webview_window(site.bar())).collect();
    if bars.is_empty() {
        return;
    }
    {
        let hub = app.state::<VideoHub>();
        let mut hub = hub.0.lock().unwrap();
        // The relay itself is heard again, and media.rs sends each event to two windows.
        if hub.relayed[slot].as_deref() == Some(payload) {
            return;
        }
        hub.relayed[slot] = Some(payload.to_owned());
    }
    for bar in bars {
        let _ = app.emit_str_to(bar.label(), event, payload.to_owned());
    }
}

/// Moves of a site window: the bar follows a small player; a full YouTube window is
/// remembered (settings.json). A closed window is forgotten.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let Some(site) = Site::of_label(window.label()) else { return };
    let app = window.app_handle();
    match event {
        WindowEvent::Moved(moved) => {
            let position = WindowPosition { x: moved.x, y: moved.y };
            if players::is_minimized(position) {
                return;
            }
            let (compact, flying) = {
                let hub = app.state::<VideoHub>();
                let mut hub = hub.0.lock().unwrap();
                let compact = hub.compact[site.index()];
                if compact && !hub.flying {
                    hub.compact_at = Some(position);
                }
                (compact, hub.flying)
            };
            if compact {
                // Moved by anything but its own bar (keyboard, Windows): the bar follows.
                if let (Some(bar), Ok(size), false) = (app.get_webview_window(site.bar()), window.outer_size(), flying)
                    && let Ok(bar_size) = bar.outer_size()
                {
                    let top = moved.y + size.height as i32 - bar_size.height as i32;
                    if bar.outer_position().is_ok_and(|at| at.x != moved.x || at.y != top) {
                        let _ = bar.set_position(PhysicalPosition::new(moved.x, top));
                    }
                }
            } else if site == Site::YouTube {
                players::save_position(app, Slot::Video, position);
            }
        }
        WindowEvent::Destroyed => {
            {
                let hub = app.state::<VideoHub>();
                let mut hub = hub.0.lock().unwrap();
                hub.compact[site.index()] = false;
                hub.full[site.index()] = None;
                if matches!(hub.detached, Some(Detached::Window(s)) if s == site) {
                    hub.detached = None;
                }
            }
            if site == Site::YouTube {
                app.state::<MediaHub>().set_video(None);
            }
            publish(app);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn the_crunchyroll_window_stays_on_crunchyroll() {
        for ok in [
            "https://www.crunchyroll.com/fr/watch/GRDV0019R/episode-7",
            "https://crunchyroll.com/",
            "https://sso.crunchyroll.com/fr/login",
            "https://static.crunchyroll.com/vilos-v2/web/vilos/player.html",
        ] {
            assert!(allows(Site::Crunchyroll, &url(ok)), "{ok}");
        }
        for bad in [
            "http://www.crunchyroll.com/",
            "https://www.crunchyroll.com:8443/",
            "https://crunchyroll.com.evil.example/",
            "https://evilcrunchyroll.com/",
            "https://crunchyroll.com@evil.example/",
            "https://accounts.google.com/o/oauth2/auth",
            "https://ilot-player.localhost/embed/dQw4w9WgXcQ",
            "http://tauri.localhost/index.html",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
        ] {
            assert!(!allows(Site::Crunchyroll, &url(bad)), "{bad}");
        }
    }

    #[test]
    fn the_youtube_window_shows_only_its_player_page() {
        assert!(allows(Site::YouTube, &url("https://ilot-player.localhost/embed/dQw4w9WgXcQ?start=0")));
        assert!(!allows(Site::YouTube, &url("https://www.youtube.com/watch?v=dQw4w9WgXcQ")));
        assert!(!allows(Site::YouTube, &url("https://www.crunchyroll.com/")));
    }

    #[test]
    fn remote_pages_get_no_device() {
        for kind in [
            PermissionKind::Microphone,
            PermissionKind::Camera,
            PermissionKind::Geolocation,
            PermissionKind::Notifications,
            PermissionKind::ClipboardRead,
            PermissionKind::FileSystemAccess,
            PermissionKind::WindowManagement,
            PermissionKind::Other,
        ] {
            assert!(matches!(permission_for(kind), PermissionResponse::Deny), "{kind:?}");
        }
        assert!(matches!(permission_for(PermissionKind::Autoplay), PermissionResponse::Default));
    }

    #[test]
    fn labels_name_their_site() {
        assert_eq!(Site::of_label("youtube"), Some(Site::YouTube));
        assert_eq!(Site::of_label("crunchyroll"), Some(Site::Crunchyroll));
        assert_eq!(Site::of_label("main"), None);
        assert_eq!(Site::of_bar("crunchyroll-bar"), Some(Site::Crunchyroll));
        assert_eq!(Site::of_bar("youtube"), None);
        assert_eq!(serde_json::to_string(&Site::YouTube).unwrap(), "\"youtube\"");
        assert_eq!(Site::Crunchyroll.compact_size(), (352.0, 198.0));
        assert_eq!(Site::YouTube.compact_size(), (356.0, 200.0), "YouTube's 200 px minimum");
    }

    #[test]
    fn own_sessions_are_recognized() {
        assert!(is_own_session("ilot.exe"));
        assert!(is_own_session("C:\\Users\\x\\AppData\\Local\\îlot\\ilot.exe"));
        assert!(is_own_session("msedgewebview2.exe"));
        assert!(!is_own_session("MSEdge"));
        assert!(!is_own_session("15EF7777.Crunchyroll_mgdgtskya6f22!App"));
    }

    const PLAYER: Frame = Frame { x: 1544.0, y: 818.0, width: 352.0, height: 198.0 };

    #[test]
    fn the_bar_covers_the_bottom_of_the_player() {
        assert_eq!(bar_frame(PLAYER, 64.0), Frame { x: 1544.0, y: 952.0, width: 352.0, height: 64.0 });
        assert_eq!(player_for_bar(1544.0, 952.0, 198.0, 64.0), (1544.0, 818.0));
        assert_eq!(player_for_bar(100.0, 300.0, 297.0, 96.0), (100.0, 99.0), "at 150 %");
    }

    const SCREEN: Area = Area { x: 0, y: 0, width: 1920, height: 1040 };
    const LEFT: Area = Area { x: -1366, y: 0, width: 1366, height: 728 };

    #[test]
    fn the_small_player_goes_back_where_it_was() {
        let last = Some(WindowPosition { x: 200, y: 300 });
        assert_eq!(compact_place((352, 198), last, &[SCREEN], SCREEN, 24), WindowPosition { x: 200, y: 300 });
        assert_eq!(compact_place((352, 198), None, &[SCREEN], SCREEN, 24), WindowPosition { x: 1544, y: 818 });
        let gone = Some(WindowPosition { x: -900, y: 300 });
        assert_eq!(compact_place((352, 198), gone, &[SCREEN], SCREEN, 24), WindowPosition { x: 1544, y: 818 });
        assert_eq!(compact_place((528, 297), gone, &[SCREEN, LEFT], LEFT, 36), WindowPosition { x: -900, y: 300 });
    }

    #[test]
    fn the_flight_eases_out_and_lands_on_target() {
        let to = drop_frame((960.0, 20.0), (352.0, 198.0), DROP_RATIO);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(close(to.width, 70.4) && close(to.height, 39.6));
        assert!(close(to.x + to.width / 2.0, 960.0) && close(to.y + to.height / 2.0, 20.0));
        let frames = flight(PLAYER, to, FLIGHT_STEPS);
        assert_eq!(frames.len(), FLIGHT_STEPS as usize);
        assert_eq!(*frames.last().unwrap(), to);
        // Ease-out: the first step covers more ground than the last.
        let first = PLAYER.y - frames[0].y;
        let last = frames[frames.len() - 2].y - frames[frames.len() - 1].y;
        assert!(first > 5.0 * last, "{first} vs {last}");
        assert_eq!(flight(PLAYER, to, 0), vec![to]);
    }

    fn facts(source: MediaSource) -> Facts {
        Facts { source: Some(source), ..Facts::default() }
    }

    #[test]
    fn own_windows_turn_into_the_small_player() {
        let youtube = Facts { own: true, youtube_video: true, open: [true, false], ..facts(MediaSource::YouTube) };
        assert_eq!(choose(&youtube), Way::Window(Site::YouTube));
        let crunchyroll = Facts { own: true, open: [false, true], browser: true, windowed: true, ..facts(MediaSource::Crunchyroll) };
        assert_eq!(choose(&crunchyroll), Way::Window(Site::Crunchyroll), "WebView2 is not a browser here");
        let unknown = Facts { own: true, open: [false, true], ..facts(MediaSource::Other) };
        assert_eq!(choose(&unknown), Way::Window(Site::Crunchyroll), "the only open window");
        let closed = Facts { own: true, youtube_video: true, browser: true, windowed: true, ..facts(MediaSource::YouTube) };
        assert_eq!(choose(&closed), Way::MiniPlayer, "never the user's browser");
        let both = Facts { own: true, open: [true, true], ..facts(MediaSource::Other) };
        assert_eq!(choose(&both), Way::MiniPlayer, "two windows, no way to tell");
    }

    #[test]
    fn crunchyroll_in_a_browser_floats_with_the_extension_or_stays_on_top() {
        let browser = Facts { browser: true, windowed: true, ..facts(MediaSource::Crunchyroll) };
        assert_eq!(choose(&Facts { extension: true, ..browser }), Way::PictureInPicture);
        assert_eq!(choose(&browser), Way::OnTop);
        let store_app = Facts { windowed: true, extension: true, ..facts(MediaSource::Crunchyroll) };
        assert_eq!(choose(&store_app), Way::OnTop, "the extension lives in browsers only");
        let youtube = Facts { browser: true, windowed: true, extension: true, ..facts(MediaSource::YouTube) };
        assert_eq!(choose(&youtube), Way::OnTop);
    }

    #[test]
    fn music_and_unknown_apps_keep_the_mini_player() {
        assert_eq!(choose(&Facts { windowed: true, ..facts(MediaSource::Spotify) }), Way::MiniPlayer);
        assert_eq!(choose(&facts(MediaSource::YouTube)), Way::MiniPlayer, "no window to lift");
        assert_eq!(choose(&Facts::default()), Way::MiniPlayer);
    }

    #[test]
    fn the_interface_reads_the_outcomes_and_the_state() {
        assert_eq!(serde_json::to_string(&Outcome::NeedsGesture).unwrap(), "\"needsGesture\"");
        assert_eq!(serde_json::to_string(&Outcome::PictureInPicture).unwrap(), "\"pictureInPicture\"");
        let state = VideoState { detached: Some(DetachedKind::Window), site: Some(Site::Crunchyroll), extension: true };
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            serde_json::json!({ "detached": "window", "site": "crunchyroll", "extension": true })
        );
        let none = VideoState { detached: None, site: None, extension: false };
        assert_eq!(serde_json::to_value(&none).unwrap(), serde_json::json!({ "detached": null, "site": null, "extension": false }));
    }

    /// The real capabilities, compiled by build.rs, decide who may call what.
    mod access {
        use crate::media::MediaHub;
        use crate::settings::SettingsStore;
        use tauri::ipc::{CallbackFn, InvokeBody};
        use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
        use tauri::webview::InvokeRequest;
        use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

        const LOCAL: &str = "http://tauri.localhost";
        const CRUNCHYROLL_PAGE: &str = "https://www.crunchyroll.com/fr/watch/GRDV0019R/episode-7";

        fn app() -> App<MockRuntime> {
            mock_builder()
                .manage(MediaHub::default())
                .manage(SettingsStore::open(None))
                .invoke_handler(tauri::generate_handler![crate::media::get_media, crate::get_settings])
                .build(tauri::generate_context!())
                .unwrap()
        }

        fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
            WebviewWindowBuilder::new(app, label, WebviewUrl::default()).build().unwrap()
        }

        /// True when the access control refuses the call. A command it lets through may
        /// still fail later (not registered in this mock app), which is not a refusal.
        fn refused(window: &WebviewWindow<MockRuntime>, cmd: &str, url: &str) -> bool {
            let request = InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: url.parse().unwrap(),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            get_ipc_response(window, request).is_err_and(|error| error.to_string().contains("not allowed"))
        }

        #[test]
        fn the_crunchyroll_window_reaches_no_command() {
            let app = app();
            let site = window(&app, super::CRUNCHYROLL_LABEL);
            for cmd in ["get_media", "get_settings", "media_command", "video_state", "dock_video", "plugin:event|listen"] {
                assert!(refused(&site, cmd, CRUNCHYROLL_PAGE), "{cmd} from Crunchyroll");
                assert!(refused(&site, cmd, LOCAL), "{cmd} even from a local page");
            }
        }

        #[test]
        fn a_bar_reaches_its_player_and_nothing_else() {
            let app = app();
            for label in [super::YOUTUBE_BAR, super::CRUNCHYROLL_BAR] {
                let bar = window(&app, label);
                for cmd in ["get_media", "get_settings", "media_command", "video_state", "video_bar_move", "dock_video", "close_video", "buddy_home"] {
                    assert!(!refused(&bar, cmd, LOCAL), "{label}: {cmd}");
                }
                for cmd in ["save_settings", "answer_permission", "detach_video", "open_crunchyroll", "play_youtube", "set_connected", "copy_text"] {
                    assert!(refused(&bar, cmd, LOCAL), "{label}: {cmd}");
                }
                assert!(refused(&bar, "get_media", CRUNCHYROLL_PAGE), "remote pages get nothing");
            }
        }

        #[test]
        fn the_island_detaches_and_docks_but_never_moves_a_bar() {
            let app = app();
            let island = window(&app, crate::window::WINDOW_LABEL);
            for cmd in ["detach_video", "dock_video", "video_state", "open_crunchyroll", "play_youtube"] {
                assert!(!refused(&island, cmd, LOCAL), "{cmd}");
            }
            for cmd in ["video_bar_move", "close_video"] {
                assert!(refused(&island, cmd, LOCAL), "{cmd}");
            }
        }

        #[test]
        fn the_mini_player_gains_nothing() {
            let app = app();
            let mini = window(&app, crate::players::MINI_LABEL);
            for cmd in ["detach_video", "video_bar_move", "close_video", "open_crunchyroll", "dock_video"] {
                assert!(refused(&mini, cmd, LOCAL), "{cmd}");
            }
        }
    }
}
