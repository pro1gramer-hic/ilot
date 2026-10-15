//! Mochi out of the island (docs/09-DEMO-FINALE.md, section 6): a small transparent
//! window, `buddy`, that never takes the keyboard nor the clicks. Its page
//! (src/Buddy.svelte) decides where Mochi goes; the core moves the window, lists the
//! places worth a visit and reports the cursor.
//!
//! Coordinates are Mochi's center in logical pixels of the virtual screen, at the scale of
//! the island's monitor when Mochi went out: one scale for the whole outing, so that a
//! point means the same place on every monitor.
//!
//! « Sur le bureau » (prompt 12, lot 1): Mochi stays where he is dropped, can be picked up
//! again, and comes back there at the next launch (`settings.desk`). In every mode the
//! window lets clicks through except on Mochi himself, so that a right click on him opens
//! his wardrobe in the island.

use crate::locale::{Text, tr};
use crate::placement::{self, Point};
use crate::players::Area;
use crate::settings::{DeskPlace, SettingsStore};
use crate::window::{IslandWindow, Rect, SHELL_CLASSES, WINDOW_LABEL, class_name, primary_button_down, rect};
use serde::Serialize;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, WINDOW_EX_STYLE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};
use windows::core::BOOL;

pub const BUDDY_LABEL: &str = "buddy";
const EVENT_CURSOR: &str = "buddy:cursor";
const EVENT_RECALL: &str = "buddy:recall";
const EVENT_RETURNED: &str = "buddy:returned";
/// A right click on Mochi out of the island opens his wardrobe there.
const EVENT_WARDROBE: &str = "island:wardrobe";
/// « Se balade », « Te suit » and « Sur le bureau »; « Reste dans l'île » never opens the window.
const MODES: [&str; 3] = ["balade", "suit", "bureau"];
const DESK: &str = "bureau";
/// Mochi's body in his window (logical pixels): a disc this wide, a little below the
/// window's center (his 72 px drawing stands on its feet in a 96 px square).
const HIT_RADIUS: f64 = 27.0;
const HIT_BELOW: f64 = 4.0;
/// Logical size of the window; Mochi (72 px) sits in its middle.
const SIZE: f64 = 96.0;
/// Mochi's center lands this far from the pill's left end, at mid-height.
const SLOT_INSET: f64 = 24.0;
/// Farthest a point may be asked (logical pixels), well beyond any desktop.
const FARTHEST: f64 = 1_000_000.0;
const INVALID_POINT: Text<'static> = Text::PositionInvalid;
/// A still cursor is reported again this often, so that a page that just loaded hears
/// where it is (and whether the button still holds Mochi).
const CURSOR_KEEPALIVE: Duration = Duration::from_millis(200);

// Places (logical pixels, multiplied by the screen's scale), as in `pickSpot` of
// reference/ilot-demo-finale.html.
/// Mochi's center keeps this far from the screen edges.
const MARGIN: f64 = 30.0;
/// At the bottom of the screen.
const ABOVE_BOTTOM: f64 = 40.0;
/// On a window's top edge: his feet on the edge.
const ABOVE_EDGE: f64 = 22.0;
/// A window's edge is only worth it this far below the top of the screen.
const EDGE_BELOW_TOP: f64 = 80.0;
/// Narrower windows are skipped.
const MIN_WINDOW_WIDTH: f64 = 160.0;
/// Only the first windows (from the top of the pile) are visited.
const MAX_WINDOWS: usize = 6;
/// Beside the cursor: this far to the side, a little below.
const CURSOR_SIDE: f64 = 96.0;
const CURSOR_BELOW: f64 = 24.0;
/// Near the cursor, never higher than this from the top (the island lives there).
const CURSOR_TOP: f64 = 110.0;
/// Against a side edge of the screen.
const SIDE: f64 = 34.0;
/// Distance kept from the island's shapes: Mochi never covers what it shows.
const KEEP_OUT: f64 = 56.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpotKind {
    /// Bottom of the screen: looks around.
    Bas,
    /// Top edge of a window: dances.
    Fenetre,
    /// Beside the cursor: waves.
    Curseur,
    /// Against a side edge: peeks.
    Bord,
    /// Middle of the screen: naps.
    Milieu,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Spot {
    pub kind: SpotKind,
    pub x: f64,
    pub y: f64,
}

/// Places worth a visit on a work area (physical pixels): along the bottom, on the top
/// edge of the visible windows (`windows` from the top of the pile down; an edge covered
/// by a window above is skipped), beside the cursor, against the sides and in the
/// middle. Nothing within reach of the island's shapes (`keep_out`).
pub fn spots(work: Rect, scale: f64, windows: &[Rect], cursor: Option<Point>, keep_out: &[Rect]) -> Vec<Spot> {
    let (left, top) = (f64::from(work.left), f64::from(work.top));
    let (right, bottom) = (f64::from(work.right), f64::from(work.bottom));
    let (width, height) = (right - left, bottom - top);
    let mut all = Vec::new();
    let mut add = |kind, x, y| all.push(Spot { kind, x, y });
    for share in [0.12, 0.27, 0.42, 0.58, 0.73, 0.88] {
        add(SpotKind::Bas, left + width * share, bottom - ABOVE_BOTTOM * scale);
    }
    let mut visited = 0;
    for (i, window) in windows.iter().enumerate() {
        let (w_left, w_top, w_right) = (f64::from(window.left), f64::from(window.top), f64::from(window.right));
        if visited == MAX_WINDOWS
            || w_right - w_left < MIN_WINDOW_WIDTH * scale
            || w_top < top + EDGE_BELOW_TOP * scale
            || w_top > bottom - ABOVE_BOTTOM * scale
        {
            continue;
        }
        let mut seen = false;
        for share in [0.25, 0.5, 0.75] {
            let x = w_left + (w_right - w_left) * share;
            if windows[..i].iter().any(|above| above.contains(x, w_top + 1.0)) {
                continue;
            }
            add(SpotKind::Fenetre, x, w_top - ABOVE_EDGE * scale);
            seen = true;
        }
        visited += usize::from(seen);
    }
    if let Some(cursor) = cursor {
        for side in [-1.0, 1.0] {
            let x = (cursor.x + side * CURSOR_SIDE * scale).clamp(left + MARGIN * scale, right - MARGIN * scale);
            let y = (cursor.y + CURSOR_BELOW * scale).clamp(top + CURSOR_TOP * scale, bottom - ABOVE_BOTTOM * scale);
            add(SpotKind::Curseur, x, y);
        }
    }
    for x in [left + SIDE * scale, right - SIDE * scale] {
        for share in [0.3, 0.5, 0.7] {
            add(SpotKind::Bord, x, top + height * share);
        }
    }
    for x_share in [0.3, 0.5, 0.7] {
        for y_share in [0.4, 0.6] {
            add(SpotKind::Milieu, left + width * x_share, top + height * y_share);
        }
    }
    let margin = MARGIN * scale;
    let reach = KEEP_OUT * scale;
    all.retain(|s| {
        s.x >= left + margin
            && s.x <= right - margin
            && s.y >= top + margin
            && s.y <= bottom - margin
            && !keep_out.iter().any(|r| {
                s.x > f64::from(r.left) - reach
                    && s.x < f64::from(r.right) + reach
                    && s.y > f64::from(r.top) - reach
                    && s.y < f64::from(r.bottom) + reach
            })
    });
    all
}

/// The window's rectangle on screen (DWM's extended frame, without the invisible
/// resize borders) when it is a window the user sees: visible, not minimized, not
/// cloaked (another virtual desktop, a suspended app), not a tool window or a
/// click-through overlay (WS_EX_TRANSPARENT), titled, not the desktop or the taskbar,
/// not îlot's own.
/// https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute
/// https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
///
/// # Safety
/// `hwnd` comes from EnumWindows; every out-parameter is a properly sized local.
unsafe fn seen_rect(hwnd: HWND, skip_process: Option<u32>) -> Option<Rect> {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() || GetWindowTextLengthW(hwnd) == 0 {
            return None;
        }
        let style = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
        if style.contains(WS_EX_TOOLWINDOW) || style.contains(WS_EX_TRANSPARENT) {
            return None;
        }
        let mut process = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&raw mut process));
        if skip_process == Some(process) {
            return None;
        }
        let mut cloaked = 0u32;
        let read = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, (&raw mut cloaked).cast::<c_void>(), size_of::<u32>() as u32);
        if read.is_err() || cloaked != 0 || SHELL_CLASSES.contains(&class_name(hwnd).as_str()) {
            return None;
        }
        let mut bounds = RECT::default();
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&raw mut bounds).cast::<c_void>(),
            size_of::<RECT>() as u32,
        )
        .ok()?;
        (bounds.right > bounds.left && bounds.bottom > bounds.top).then_some(rect(bounds))
    }
}

struct Found {
    skip_process: Option<u32>,
    list: Vec<(isize, Rect)>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam is the `Found` that `visible_windows` lends to EnumWindows for the
    // length of the call, and the handle comes from the system.
    unsafe {
        let found = &mut *(lparam.0 as *mut Found);
        if let Some(rect) = seen_rect(hwnd, found.skip_process) {
            found.list.push((hwnd.0 as isize, rect));
        }
    }
    BOOL(1)
}

/// The windows the user sees, with their handle, from the top of the pile down:
/// EnumWindows walks the system's single z-order list that way (its page does not say
/// so; `the_newest_window_comes_first_like_the_pile` checks it). Should the order ever
/// differ, Mochi would only skip or keep an edge wrongly.
/// https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumwindows
/// https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#z-order
pub(crate) fn visible_windows(skip_process: Option<u32>) -> Vec<(isize, Rect)> {
    let mut found = Found { skip_process, list: Vec::new() };
    // SAFETY: the callback only uses `found` during the call.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM((&raw mut found) as isize)) };
    found.list
}

/// The cursor as Mochi's page receives it, with the primary button (held under the
/// cursor until it is released).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Cursor {
    pub x: f64,
    pub y: f64,
    pub down: bool,
}

#[derive(Default)]
pub struct Buddy {
    /// Scale of the coordinates while Mochi is out; nothing while he is home.
    space: Mutex<Option<f64>>,
    last_cursor: Mutex<Option<(Cursor, Instant)>>,
    /// Out « Sur le bureau »: he stays where he is put.
    desk: AtomicBool,
    /// Held under the cursor (pulled out of the island, or picked up on the desktop): his
    /// page follows the cursor reports until the button is released.
    held: AtomicBool,
    /// Where he is on the screen (physical pixels) and his window's scale, for the clicks.
    body: Mutex<Option<(Point, f64)>>,
    /// The window takes the clicks now (the cursor is on him).
    solid: AtomicBool,
}

impl Buddy {
    pub(crate) fn space(&self) -> Option<f64> {
        *self.space.lock().unwrap()
    }

    fn open(&self, space: f64, desk: bool, held: bool) {
        *self.space.lock().unwrap() = Some(space);
        *self.last_cursor.lock().unwrap() = None;
        *self.body.lock().unwrap() = None;
        self.desk.store(desk, Ordering::SeqCst);
        self.held.store(held, Ordering::SeqCst);
        self.solid.store(false, Ordering::SeqCst);
    }

    fn on_desk(&self) -> bool {
        self.desk.load(Ordering::SeqCst)
    }

    fn hold(&self, on: bool) {
        self.held.store(on, Ordering::SeqCst);
    }

    /// The page needs the cursor: always while he roams or follows, only while he is
    /// held when he rests on the desktop (no reports at rest).
    fn wants_cursor(&self) -> bool {
        !self.on_desk() || self.held.load(Ordering::SeqCst)
    }

    fn close(&self) {
        *self.space.lock().unwrap() = None;
    }

    /// True when the cursor changed since the last report, or when that report is old.
    fn worth_reporting(&self, now: Cursor, at: Instant) -> bool {
        let mut last = self.last_cursor.lock().unwrap();
        let due = last.is_none_or(|(cursor, when)| cursor != now || at.duration_since(when) >= CURSOR_KEEPALIVE);
        if due {
            *last = Some((now, at));
        }
        due
    }
}

/// Top-left corner (physical) of the window centered on Mochi at `(x, y)`.
pub fn origin(space: f64, window_scale: f64, x: f64, y: f64) -> (i32, i32) {
    let half = SIZE / 2.0 * window_scale;
    ((x * space - half).round() as i32, (y * space - half).round() as i32)
}

fn checked(x: f64, y: f64) -> Result<(f64, f64), String> {
    let fine = |v: f64| v.is_finite() && v.abs() <= FARTHEST;
    if fine(x) && fine(y) { Ok((x, y)) } else { Err(INVALID_POINT.into()) }
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn place(window: &WebviewWindow, buddy: &Buddy, space: f64, x: f64, y: f64) -> Result<(), String> {
    let scale = window.scale_factor().map_err(error)?;
    let (left, top) = origin(space, scale, x, y);
    *buddy.body.lock().unwrap() = Some((Point::new(x * space, y * space), scale));
    window.set_position(PhysicalPosition::new(left, top)).map_err(error)
}

/// Distance (logical pixels of his window) from the cursor to Mochi's body, 0 on it.
pub fn distance_to_body(body: Point, scale: f64, cursor: Point) -> f64 {
    let dx = (cursor.x - body.x) / scale;
    let dy = (cursor.y - body.y) / scale - HIT_BELOW;
    (dx.hypot(dy) - HIT_RADIUS).max(0.0)
}

/// Makes Mochi's window take the clicks while the cursor is on him (a right click opens
/// his wardrobe; on the desktop, a press picks him up), and let them through elsewhere.
/// Returns how far the cursor is from him while he is out, so that the cursor loop
/// watches it closely only when it comes near.
pub fn hit_test(app: &AppHandle, cursor: Option<Point>) -> Option<f64> {
    let buddy = app.state::<Buddy>();
    buddy.space()?;
    let (body, scale) = (*buddy.body.lock().unwrap())?;
    let distance = distance_to_body(body, scale, cursor?);
    let solid = distance == 0.0;
    if buddy.solid.swap(solid, Ordering::SeqCst) != solid
        && let Some(window) = app.get_webview_window(BUDDY_LABEL)
    {
        let _ = window.set_ignore_cursor_events(!solid);
    }
    Some(distance)
}

/// A full-screen app came or went: on the desktop Mochi hides and comes back where he
/// was; out for a walk, he goes home as before.
pub fn fullscreen(app: &AppHandle, on: bool) {
    let buddy = app.state::<Buddy>();
    if buddy.space().is_none() {
        return;
    }
    if !buddy.on_desk() {
        if on {
            recall(app);
        }
        return;
    }
    if let Some(window) = app.get_webview_window(BUDDY_LABEL) {
        let _ = if on { window.hide() } else { window.show() };
    }
}

/// Where Mochi rests, as saved: the screen under his center and his share of its work area.
pub fn desk_place(screens: &[(String, Area, Area)], center: Point) -> Option<DeskPlace> {
    let (name, _, work) = screens.iter().find(|(_, bounds, _)| {
        center.x >= f64::from(bounds.x)
            && center.x < f64::from(bounds.x + bounds.width)
            && center.y >= f64::from(bounds.y)
            && center.y < f64::from(bounds.y + bounds.height)
    })?;
    let share = placement::to_share(*work, center);
    Some(DeskPlace { monitor: name.clone(), x: share.x.clamp(0.0, 1.0), y: share.y.clamp(0.0, 1.0) })
}

/// Back from the saved place: on the same screen if it is still there, kept off its edges.
pub fn desk_point(screens: &[(String, Area, Area)], place: &DeskPlace, scale: f64) -> Option<Point> {
    let (_, _, work) = screens.iter().find(|(name, ..)| *name == place.monitor)?;
    let p = placement::from_share(*work, Point::new(place.x, place.y))?;
    let margin = MARGIN * scale;
    let (left, top) = (f64::from(work.x) + margin, f64::from(work.y) + margin);
    let (right, bottom) = (f64::from(work.x + work.width) - margin, f64::from(work.y + work.height) - margin);
    if left > right || top > bottom {
        return None;
    }
    Some(Point::new(p.x.clamp(left, right), p.y.clamp(top, bottom)))
}

/// Every monitor: its name, its whole area and its work area (physical pixels).
fn monitors(app: &AppHandle) -> Vec<(String, Area, Area)> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let work = m.work_area();
            (
                m.name().cloned().unwrap_or_default(),
                Area { x: m.position().x, y: m.position().y, width: m.size().width as i32, height: m.size().height as i32 },
                Area { x: work.position.x, y: work.position.y, width: work.size.width as i32, height: work.size.height as i32 },
            )
        })
        .collect()
}

/// Reports the cursor to Mochi's page while he is out (about 60 times per second, when
/// it moves). Returns true while he is out.
pub fn report_cursor(app: &AppHandle, cursor: Option<Point>) -> bool {
    let buddy = app.state::<Buddy>();
    let Some(space) = buddy.space() else { return false };
    if !buddy.wants_cursor() {
        return false;
    }
    if let Some(c) = cursor {
        let now = Cursor { x: c.x / space, y: c.y / space, down: primary_button_down() };
        if buddy.worth_reporting(now, Instant::now()) {
            let _ = app.emit_to(BUDDY_LABEL, EVENT_CURSOR, now);
        }
    }
    true
}

/// Asks Mochi to come home (an alert, a click on the island, a full-screen app).
pub fn recall(app: &AppHandle) {
    if app.state::<Buddy>().space().is_some() {
        let _ = app.emit_to(BUDDY_LABEL, EVENT_RECALL, ());
    }
}

/// Once the window is gone, Mochi is home: the island shows him again.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() == BUDDY_LABEL && matches!(event, WindowEvent::Destroyed) {
        window.state::<Buddy>().close();
        let _ = window.emit_to(WINDOW_LABEL, EVENT_RETURNED, ());
    }
}

/// Opens Mochi's window centered on a screen point, in the given mode (or starts him
/// again from there if he is already out).
#[tauri::command]
pub async fn buddy_show(x: f64, y: f64, mode: String, app: AppHandle) -> Result<(), String> {
    if !MODES.contains(&mode.as_str()) {
        return Err(tr(Text::ModeUnknown { mode: &mode }));
    }
    let (x, y) = checked(x, y)?;
    // Pulled out of the island: held under the cursor until the page sees the button up.
    open_at(&app, x, y, &mode, true)
}

/// Opens (or moves and restarts) Mochi's window on `(x, y)`; `restore` puts him back where
/// he was left, without the jump out of the island.
fn open_at(app: &AppHandle, x: f64, y: f64, mode: &str, held: bool) -> Result<(), String> {
    let space = app.state::<IslandWindow>().screen().map_or(1.0, |s| s.scale);
    let buddy = app.state::<Buddy>();
    buddy.open(space, mode == DESK, held);
    let restore = if held { "" } else { "&restore=1" };
    let page = format!("buddy.html?x={x:.1}&y={y:.1}&mode={mode}{restore}");
    if let Some(window) = app.get_webview_window(BUDDY_LABEL) {
        let url = window.url().map_err(error)?.join(&page).map_err(error)?;
        window.set_ignore_cursor_events(true).map_err(error)?;
        place(&window, &buddy, space, x, y)?;
        return window.navigate(url).map_err(error);
    }
    let mut config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == BUDDY_LABEL)
        .cloned()
        .ok_or(Text::MochiWindowMissing)?;
    config.url = WebviewUrl::App(page.into());
    let window = WebviewWindowBuilder::from_config(app, &config).map_err(error)?.build().map_err(error)?;
    window.set_ignore_cursor_events(true).map_err(error)?;
    place(&window, &buddy, space, x, y)?;
    window.show().map_err(error)
}

/// Moves Mochi's window so that he is centered on the point.
#[tauri::command]
pub async fn buddy_move(x: f64, y: f64, app: AppHandle) -> Result<(), String> {
    let (x, y) = checked(x, y)?;
    let buddy = app.state::<Buddy>();
    let (Some(window), Some(space)) = (app.get_webview_window(BUDDY_LABEL), buddy.space()) else {
        return Ok(());
    };
    place(&window, &buddy, space, x, y)
}

/// « Sur le bureau »: the user picked Mochi up; his page follows the cursor until release.
#[tauri::command]
pub fn buddy_grab(app: AppHandle) {
    app.state::<Buddy>().hold(true);
}

/// « Sur le bureau »: Mochi was put down at `(x, y)`; the place is saved for the next launch.
#[tauri::command]
pub fn buddy_settle(x: f64, y: f64, app: AppHandle) -> Result<(), String> {
    let (x, y) = checked(x, y)?;
    let buddy = app.state::<Buddy>();
    buddy.hold(false);
    let (Some(space), true) = (buddy.space(), buddy.on_desk()) else { return Ok(()) };
    let place = desk_place(&monitors(&app), Point::new(x * space, y * space));
    app.state::<SettingsStore>().update(|settings| settings.desk = place).map(|_| ()).map_err(error)
}

/// At launch: Mochi was left on the desktop last time (and « Sur le bureau » is still the
/// mode), so he comes back there. True when he is out again.
#[tauri::command]
pub fn buddy_restore(app: AppHandle) -> Result<bool, String> {
    let settings = app.state::<SettingsStore>().get();
    let (Some(place), true) = (settings.desk, settings.modes.mochi == DESK) else { return Ok(false) };
    let Some(space) = app.state::<IslandWindow>().screen().map(|s| s.scale) else { return Ok(false) };
    let Some(point) = desk_point(&monitors(&app), &place, space) else { return Ok(false) };
    open_at(&app, point.x / space, point.y / space, DESK, false)?;
    Ok(true)
}

/// A right click on Mochi out of the island: his wardrobe opens in the island.
#[tauri::command]
pub fn open_wardrobe(app: AppHandle) {
    let _ = app.emit_to(WINDOW_LABEL, EVENT_WARDROBE, ());
}

/// Places worth a visit on the island's screen.
#[tauri::command]
pub async fn buddy_spots(app: AppHandle) -> Vec<Spot> {
    let island = app.state::<IslandWindow>();
    let Some(screen) = island.screen() else { return Vec::new() };
    let space = app.state::<Buddy>().space().unwrap_or(screen.scale);
    let work = Rect {
        left: screen.work.x,
        top: screen.work.y,
        right: screen.work.x + screen.work.width,
        bottom: screen.work.y + screen.work.height,
    };
    let windows: Vec<Rect> = visible_windows(Some(std::process::id())).into_iter().map(|(_, r)| r).collect();
    let cursor = app.cursor_position().ok().map(|c| Point::new(c.x, c.y));
    spots(work, screen.scale, &windows, cursor, &island.shapes_on_screen())
        .into_iter()
        .map(|s| Spot { x: s.x / space, y: s.y / space, ..s })
        .collect()
}

/// Asks Mochi to come home now.
#[tauri::command]
pub fn buddy_recall(app: AppHandle) {
    recall(&app);
}

/// Screen point where Mochi lands back in the island.
#[tauri::command]
pub fn buddy_home(app: AppHandle) -> Result<Point, String> {
    let island = app.state::<IslandWindow>();
    let slot = island.mochi_slot(SLOT_INSET).ok_or(Text::IslandNotPlaced)?;
    let space = app.state::<Buddy>().space().or_else(|| island.screen().map(|s| s.scale)).unwrap_or(1.0);
    Ok(Point::new(slot.x / space, slot.y / space))
}

/// Closes Mochi's window once he is home; the island then gets `buddy:returned`. Home,
/// he no longer has a place on the desktop.
#[tauri::command]
pub fn buddy_hide(app: AppHandle) {
    if app.state::<SettingsStore>().get().desk.is_some()
        && let Err(e) = app.state::<SettingsStore>().update(|settings| settings.desk = None)
    {
        eprintln!("îlot: Mochi's place not cleared: {e}");
    }
    if let Some(window) = app.get_webview_window(BUDDY_LABEL) {
        let _ = window.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1040 };

    fn of(list: &[Spot], kind: SpotKind) -> Vec<(f64, f64)> {
        list.iter().filter(|s| s.kind == kind).map(|s| (s.x, s.y)).collect()
    }

    #[test]
    fn every_kind_of_place_on_an_empty_desktop() {
        let list = spots(WORK, 1.0, &[], Some(Point::new(800.0, 500.0)), &[]);
        for kind in [SpotKind::Bas, SpotKind::Curseur, SpotKind::Bord, SpotKind::Milieu] {
            assert!(!of(&list, kind).is_empty(), "{kind:?}");
        }
        assert!(of(&list, SpotKind::Fenetre).is_empty());
        assert!(of(&list, SpotKind::Bas).iter().all(|&(_, y)| y == 1000.0));
        assert_eq!(of(&list, SpotKind::Curseur), vec![(704.0, 524.0), (896.0, 524.0)]);
        assert!(of(&list, SpotKind::Bord).iter().all(|&(x, _)| x == 34.0 || x == 1886.0));
    }

    #[test]
    fn mochi_stands_on_the_top_edge_of_a_window() {
        let editor = Rect { left: 200, top: 300, right: 1000, bottom: 900 };
        let list = spots(WORK, 1.0, &[editor], None, &[]);
        assert_eq!(of(&list, SpotKind::Fenetre), vec![(400.0, 278.0), (600.0, 278.0), (800.0, 278.0)]);
    }

    #[test]
    fn a_covered_edge_is_skipped() {
        let front = Rect { left: 500, top: 100, right: 1400, bottom: 800 };
        let behind = Rect { left: 200, top: 300, right: 1000, bottom: 900 };
        let list = spots(WORK, 1.0, &[front, behind], None, &[]);
        let edges = of(&list, SpotKind::Fenetre);
        assert!(edges.contains(&(400.0, 278.0)), "the uncovered part of the edge stays");
        assert!(!edges.contains(&(600.0, 278.0)) && !edges.contains(&(800.0, 278.0)));
        assert!(edges.contains(&(725.0, 78.0)));
    }

    #[test]
    fn maximized_narrow_and_off_screen_windows_give_no_edge() {
        let maximized = Rect { left: 0, top: 0, right: 1920, bottom: 1040 };
        let narrow = Rect { left: 300, top: 400, right: 420, bottom: 600 };
        let elsewhere = Rect { left: 2200, top: 400, right: 3000, bottom: 900 };
        let list = spots(WORK, 1.0, &[narrow, elsewhere, maximized], None, &[]);
        assert!(of(&list, SpotKind::Fenetre).is_empty());
    }

    #[test]
    fn only_the_first_windows_are_visited() {
        let windows: Vec<Rect> = (0..10).map(|i| Rect { left: 100 + i * 150, top: 200 + i * 60, right: 300 + i * 150, bottom: 260 + i * 60 }).collect();
        let list = spots(WORK, 1.0, &windows, None, &[]);
        let tops: std::collections::BTreeSet<i64> = of(&list, SpotKind::Fenetre).iter().map(|&(_, y)| y as i64).collect();
        assert_eq!(tops.len(), MAX_WINDOWS);
    }

    #[test]
    fn mochi_never_lands_on_the_island() {
        let island = Rect { left: 470, top: 12, right: 1450, bottom: 700 };
        let list = spots(WORK, 1.0, &[Rect { left: 600, top: 400, right: 1300, bottom: 900 }], Some(Point::new(900.0, 300.0)), &[island]);
        for s in &list {
            let inside = s.x > 470.0 - 56.0 && s.x < 1450.0 + 56.0 && s.y > 12.0 - 56.0 && s.y < 700.0 + 56.0;
            assert!(!inside, "{s:?}");
        }
        assert!(of(&list, SpotKind::Curseur).is_empty());
    }

    #[test]
    fn places_follow_the_scale_and_the_screen_origin() {
        let work = Rect { left: -2560, top: 0, right: 0, bottom: 1400 };
        let list = spots(work, 1.5, &[], Some(Point::new(-1000.0, 10.0)), &[]);
        assert!(of(&list, SpotKind::Bas).iter().all(|&(_, y)| y == 1340.0));
        assert_eq!(of(&list, SpotKind::Curseur), vec![(-1144.0, 165.0), (-856.0, 165.0)]);
        assert!(of(&list, SpotKind::Bord).iter().all(|&(x, _)| x == -2509.0 || x == -51.0));
        assert!(list.iter().all(|s| s.x > -2560.0 && s.x < 0.0));
    }

    #[test]
    fn places_are_sent_with_french_kinds() {
        let json = serde_json::to_value(Spot { kind: SpotKind::Fenetre, x: 1.0, y: 2.0 }).unwrap();
        assert_eq!(json, serde_json::json!({ "kind": "fenetre", "x": 1.0, "y": 2.0 }));
        let cursor = serde_json::to_value(Cursor { x: 3.0, y: 4.0, down: true }).unwrap();
        assert_eq!(cursor, serde_json::json!({ "x": 3.0, "y": 4.0, "down": true }));
    }

    #[test]
    fn the_window_is_centered_on_mochi() {
        assert_eq!(origin(1.0, 1.0, 500.0, 300.0), (452, 252));
        assert_eq!(origin(1.5, 1.5, 500.0, 300.0), (678, 378));
        assert_eq!(origin(1.5, 1.0, 500.0, 300.0), (702, 402), "another monitor's scale only changes the size");
    }

    #[test]
    fn absurd_points_are_refused() {
        assert!(checked(10.0, -20.0).is_ok());
        assert_eq!(checked(f64::NAN, 0.0).unwrap_err(), INVALID_POINT);
        assert!(checked(0.0, f64::INFINITY).is_err());
        assert!(checked(2e6, 0.0).is_err());
    }

    #[test]
    fn the_cursor_is_reported_when_it_changes_and_now_and_then() {
        let buddy = Buddy::default();
        assert_eq!(buddy.space(), None);
        buddy.open(1.25, false, true);
        assert_eq!(buddy.space(), Some(1.25));
        let c = Cursor { x: 1.0, y: 2.0, down: false };
        let t = Instant::now();
        assert!(buddy.worth_reporting(c, t));
        assert!(!buddy.worth_reporting(c, t + Duration::from_millis(16)));
        assert!(buddy.worth_reporting(Cursor { down: true, ..c }, t + Duration::from_millis(32)));
        let still = Cursor { down: true, ..c };
        assert!(!buddy.worth_reporting(still, t + Duration::from_millis(100)));
        assert!(buddy.worth_reporting(still, t + Duration::from_millis(240)), "a still cursor is told again");
        buddy.close();
        assert_eq!(buddy.space(), None);
    }

    #[test]
    fn on_the_desktop_the_cursor_is_only_reported_while_he_is_held() {
        let buddy = Buddy::default();
        buddy.open(1.0, false, false);
        assert!(buddy.wants_cursor(), "out for a walk, he watches the cursor");
        buddy.open(1.0, true, true);
        assert!(buddy.wants_cursor(), "just pulled out of the island");
        buddy.hold(false);
        assert!(!buddy.wants_cursor(), "put down: nothing to report");
        buddy.hold(true);
        assert!(buddy.wants_cursor(), "picked up again");
    }

    #[test]
    fn clicks_land_on_his_body_only() {
        let body = Point::new(500.0, 300.0);
        assert_eq!(distance_to_body(body, 1.0, Point::new(500.0, 304.0)), 0.0);
        assert_eq!(distance_to_body(body, 1.0, Point::new(520.0, 310.0)), 0.0);
        assert!(distance_to_body(body, 1.0, Point::new(500.0 + 45.0, 300.0 - 45.0)) > 0.0, "the window's corner");
        assert_eq!(distance_to_body(body, 1.0, Point::new(500.0, 304.0 + 127.0)), 100.0);
        assert_eq!(distance_to_body(body, 2.0, Point::new(500.0 + 50.0, 308.0)), 0.0, "twice as large at 200 %");
    }

    #[test]
    fn his_place_on_the_desktop_survives_a_new_resolution() {
        let laptop = ("DISPLAY1".to_owned(), Area { x: 0, y: 0, width: 1920, height: 1080 }, Area { x: 0, y: 0, width: 1920, height: 1032 });
        let right = (
            "DISPLAY2".to_owned(),
            Area { x: 1920, y: 0, width: 2560, height: 1440 },
            Area { x: 1920, y: 0, width: 2560, height: 1368 },
        );
        let screens = vec![laptop.clone(), right.clone()];
        let place = desk_place(&screens, Point::new(1920.0 + 1280.0, 684.0)).unwrap();
        assert_eq!(place, DeskPlace { monitor: "DISPLAY2".into(), x: 0.5, y: 0.5 });
        assert!(desk_place(&screens, Point::new(-5.0, 10.0)).is_none(), "off every screen");
        // The right screen now runs at 1920 × 1080: same share, same spot on it.
        let smaller = ("DISPLAY2".to_owned(), Area { x: 1920, y: 0, width: 1920, height: 1080 }, Area { x: 1920, y: 0, width: 1920, height: 1032 });
        assert_eq!(desk_point(&[laptop.clone(), smaller], &place, 1.0), Some(Point::new(1920.0 + 960.0, 516.0)));
        // Unplugged: he stays home.
        assert_eq!(desk_point(std::slice::from_ref(&laptop), &place, 1.0), None);
        // A corner is kept off the edges.
        let corner = DeskPlace { monitor: "DISPLAY1".into(), x: 1.0, y: 0.0 };
        assert_eq!(desk_point(&[laptop], &corner, 1.0), Some(Point::new(1890.0, 30.0)));
    }

    /// Windows created by the test itself, far off screen and never activated.
    mod win32 {
        use super::super::*;
        use windows::Win32::Graphics::Dwm::{DWMWA_CLOAK, DwmSetWindowAttribute};
        use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext};
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SW_HIDE, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, ShowWindow, WINDOW_STYLE,
            WS_EX_NOACTIVATE, WS_POPUP,
        };
        use windows::core::{PCWSTR, w};

        struct TestWindow(HWND);

        impl TestWindow {
            /// A popup window of the system « STATIC » class: no class to register.
            fn new(title: PCWSTR, extra: WINDOW_EX_STYLE, x: i32, y: i32) -> Self {
                // SAFETY: creates and shows a window owned by this test thread.
                unsafe {
                    let hwnd = CreateWindowExW(
                        WS_EX_NOACTIVATE | extra,
                        w!("STATIC"),
                        title,
                        WINDOW_STYLE(WS_POPUP.0),
                        x,
                        y,
                        400,
                        300,
                        None,
                        None,
                        None,
                        None,
                    )
                    .unwrap();
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    Self(hwnd)
                }
            }

            fn rect(&self) -> Option<Rect> {
                visible_windows(None).into_iter().find(|(h, _)| *h == self.0.0 as isize).map(|(_, r)| r)
            }

            fn position(&self, list: &[(isize, Rect)]) -> Option<usize> {
                list.iter().position(|(h, _)| *h == self.0.0 as isize)
            }
        }

        impl Drop for TestWindow {
            fn drop(&mut self) {
                // SAFETY: the window belongs to this thread.
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }

        fn physical_pixels() {
            // SAFETY: changes this test thread's DPI awareness only.
            unsafe {
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            }
        }

        #[test]
        fn a_visible_window_is_listed_with_its_frame() {
            physical_pixels();
            let window = TestWindow::new(w!("îlot test visible"), WINDOW_EX_STYLE(0), -24_000, -24_000);
            assert_eq!(window.rect(), Some(Rect { left: -24_000, top: -24_000, right: -23_600, bottom: -23_700 }));
            let mine = visible_windows(Some(std::process::id()));
            assert_eq!(window.position(&mine), None, "îlot skips its own windows");
        }

        #[test]
        fn hidden_minimized_cloaked_tool_and_untitled_windows_are_skipped() {
            physical_pixels();
            let hidden = TestWindow::new(w!("îlot test hidden"), WINDOW_EX_STYLE(0), -24_500, -24_000);
            // SAFETY: our own windows.
            unsafe {
                let _ = ShowWindow(hidden.0, SW_HIDE);
            }
            assert_eq!(hidden.rect(), None);
            let minimized = TestWindow::new(w!("îlot test minimized"), WINDOW_EX_STYLE(0), -25_000, -24_000);
            // SAFETY: our own window.
            unsafe {
                let _ = ShowWindow(minimized.0, SW_SHOWMINNOACTIVE);
            }
            assert_eq!(minimized.rect(), None);
            let cloaked = TestWindow::new(w!("îlot test cloaked"), WINDOW_EX_STYLE(0), -25_500, -24_000);
            let on = BOOL(1);
            // SAFETY: DWMWA_CLOAK reads one BOOL; an app may cloak its own windows.
            unsafe {
                DwmSetWindowAttribute(cloaked.0, DWMWA_CLOAK, (&raw const on).cast::<c_void>(), size_of::<BOOL>() as u32)
                    .unwrap();
            }
            assert_eq!(cloaked.rect(), None);
            let tool = TestWindow::new(w!("îlot test tool"), WS_EX_TOOLWINDOW, -26_000, -24_000);
            assert_eq!(tool.rect(), None);
            let overlay = TestWindow::new(w!("îlot test overlay"), WS_EX_TRANSPARENT, -26_500, -24_000);
            assert_eq!(overlay.rect(), None);
            let untitled = TestWindow::new(w!(""), WINDOW_EX_STYLE(0), -27_000, -24_000);
            assert_eq!(untitled.rect(), None);
        }

        #[test]
        fn the_newest_window_comes_first_like_the_pile() {
            physical_pixels();
            let older = TestWindow::new(w!("îlot test older"), WINDOW_EX_STYLE(0), -24_000, -26_000);
            let newer = TestWindow::new(w!("îlot test newer"), WINDOW_EX_STYLE(0), -23_900, -25_900);
            let list = visible_windows(None);
            assert!(newer.position(&list).unwrap() < older.position(&list).unwrap());
        }
    }

    /// The real capabilities, compiled by build.rs, decide who may call what.
    mod access {
        use tauri::ipc::{CallbackFn, InvokeBody};
        use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
        use tauri::webview::InvokeRequest;
        use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

        const LOCAL: &str = "http://tauri.localhost";

        fn app() -> App<MockRuntime> {
            mock_builder().build(tauri::generate_context!()).unwrap()
        }

        fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
            WebviewWindowBuilder::new(app, label, WebviewUrl::default()).build().unwrap()
        }

        /// False when the access control refuses the call; any other answer (here the
        /// command is not registered in the mock app) means the call got through.
        fn allowed(window: &WebviewWindow<MockRuntime>, cmd: &str) -> bool {
            let request = InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: LOCAL.parse().unwrap(),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            get_ipc_response(window, request).is_err_and(|e| !e.to_string().contains("not allowed"))
        }

        #[test]
        fn mochi_s_window_reaches_its_own_commands_only() {
            let app = app();
            let buddy = window(&app, super::super::BUDDY_LABEL);
            for cmd in [
                "buddy_move",
                "buddy_spots",
                "buddy_home",
                "buddy_hide",
                "get_settings",
                "buddy_grab",
                "buddy_settle",
                "open_wardrobe",
            ] {
                assert!(allowed(&buddy, cmd), "{cmd}");
            }
            for cmd in [
                "buddy_show",
                "save_settings",
                "answer_permission",
                "island_drag_start",
                "set_island_shape",
                "buddy_restore",
                "play_sound",
                "island_to_screen",
                "island_memory",
            ] {
                assert!(!allowed(&buddy, cmd), "{cmd}");
            }
        }

        #[test]
        fn the_island_moves_itself_and_sends_mochi_out() {
            let app = app();
            let main = window(&app, crate::window::WINDOW_LABEL);
            for cmd in [
                "island_drag_start",
                "island_move_by",
                "island_recenter",
                "island_placement",
                "buddy_show",
                "buddy_recall",
                "buddy_home",
                "buddy_restore",
                "list_screens",
                "island_to_screen",
                "play_sound",
                "sound_overrides",
                "sound_file",
                "open_sounds_folder",
                "island_memory",
            ] {
                assert!(allowed(&main, cmd), "{cmd}");
            }
            for cmd in ["buddy_move", "buddy_spots", "buddy_hide", "buddy_grab", "buddy_settle", "open_wardrobe"] {
                assert!(!allowed(&main, cmd), "{cmd}");
            }
        }
    }

    #[test]
    fn a_bad_point_is_refused_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(checked(f64::NAN, 0.0).unwrap_err(), "Invalid position");
    }
}
