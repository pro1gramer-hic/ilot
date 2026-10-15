//! The island window: one transparent, always-on-top window that moves with the island
//! (docs/10-PORTAGE-FINALE.md). The interface draws the pill around an anchor point in
//! the window; the core puts the window where the island rests (placement.rs), carries
//! it while the user drags the pill, and lets clicks through everywhere except on the
//! shapes the interface sends: the island, the open panel, the music bubble.

use crate::buddy;
use crate::locale::Text;
use crate::placement::{self, CARRY, Point, Screen, Spring};
use crate::players::Area;
use crate::screens::{self, Display, Follow, ScreenView};
use crate::settings::{DisplaySettings, IslandPosition, SettingsStore};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, State, WebviewWindow};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetClassNameW, GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId,
    IsWindow, SM_SWAPBUTTON,
    SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetForegroundWindow, SystemParametersInfoW,
};
use windows::core::BOOL;

pub const WINDOW_LABEL: &str = "main";

const EVENT_POINTER: &str = "island:pointer";
const EVENT_PLACED: &str = "island:placed";
const EVENT_DRAG: &str = "island:drag";
/// The hidden island is called back: the cursor touched the edge it rests against.
/// Folded pill and music bubble are 40 px high; anything taller is an open island.
const FOLDED_HEIGHT: f64 = 48.0;
/// The island, the panel card and the music bubble (one spare).
const MAX_SHAPES: usize = 4;
/// The cursor is near the island within this distance (logical pixels).
const NEAR: f64 = 160.0;
/// One frame at 60 images per second: window moves and cursor reports near the island.
const FRAME: Duration = Duration::from_millis(16);
/// While a drag lasts the window follows the cursor at twice the frame rate.
const DRAG_TICK: Duration = Duration::from_millis(8);
/// Screens, resolution and full-screen apps are checked once per second.
const SLOW_CHECKS: Duration = Duration::from_secs(1);
/// After the interface loads, its listeners are in place well within this delay.
const ANNOUNCE_AFTER: Duration = Duration::from_millis(300);
/// The release speed is measured over the moves of this last stretch (seconds).
const SPEED_WINDOW_S: f64 = 0.1;
/// Longest step of the spring between two frames (a stalled frame never jumps far).
const MAX_STEP_S: f64 = 0.05;
/// Largest move asked at once (logical pixels): a keyboard step is 24.
const MAX_MOVE: f64 = 10_000.0;
/// Widest folded pill used for magnets; a wider first shape is an opened island.
const MAX_PILL_HALF_WIDTH: f64 = 160.0;
/// Desktop and taskbar windows cover the screen but are neither apps nor full screen.
pub const SHELL_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];
/// « Position » mode where the island always snaps to a magnet.
const MAGNETIC: &str = "aimant";

/// A clickable outline in logical pixels, relative to the window's top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
pub struct IslandShape {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

impl IslandShape {
    /// True when the point is on the rounded shape. With `top_strip`, the strip between
    /// the window's top edge and the shape counts too: when the island rests against the
    /// top of the screen, touching that edge counts as hovering.
    pub fn contains(&self, px: f64, py: f64, top_strip: bool) -> bool {
        if self.width <= 0.0 || self.height <= 0.0 {
            return false;
        }
        let right = self.x + self.width;
        let bottom = self.y + self.height;
        if px < self.x || px > right || py > bottom {
            return false;
        }
        if py < self.y {
            return top_strip && py >= 0.0;
        }
        let r = self.radius.min(self.width / 2.0).min(self.height / 2.0).max(0.0);
        let dx = px - px.clamp(self.x + r, right - r);
        let dy = py - py.clamp(self.y + r, bottom - r);
        dx * dx + dy * dy <= r * r
    }

    /// Distance from the point to the shape's box (0 on it).
    pub fn distance(&self, px: f64, py: f64) -> f64 {
        let dx = (self.x - px).max(px - (self.x + self.width)).max(0.0);
        let dy = (self.y - py).max(py - (self.y + self.height)).max(0.0);
        dx.hypot(dy)
    }
}

/// The cursor as the interface sees it, in the window's logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Pointer {
    pub x: f64,
    pub y: f64,
    pub inside: bool,
    pub near: bool,
}

/// Near the island every move is sent; elsewhere only entering or leaving.
pub fn worth_sending(previous: Option<Pointer>, now: Pointer) -> bool {
    match previous {
        None => true,
        Some(p) => p.inside != now.inside || p.near != now.near || (now.near && (p.x != now.x || p.y != now.y)),
    }
}

/// How long the cursor thread sleeps: one frame near the island, longer far from it, so
/// that a distant cursor costs almost nothing.
pub fn pause_for(distance: f64) -> Duration {
    if distance <= NEAR {
        FRAME
    } else if distance <= 3.0 * NEAR {
        Duration::from_millis(50)
    } else {
        Duration::from_millis(100)
    }
}

/// Where the pill sits in its window and which way the island opens (`island:placed`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub opens_down: bool,
}

#[derive(Clone, Copy, Serialize)]
struct Dragging {
    dragging: bool,
}

/// The island on its screen, in physical pixels.
#[derive(Clone, Debug)]
struct Placed {
    screen: Screen,
    /// Center of the pill now, where it is going, and its speed (pixels per second).
    center: Point,
    target: Point,
    speed: Point,
    /// Anchor last sent to the interface (logical) and the way the island opens.
    anchor: Point,
    opens_down: bool,
    /// Top-left corner of the window now, and the window's scale factor.
    window: (i32, i32),
    scale: f64,
}

/// What the cursor thread needs to read the cursor in window coordinates.
#[derive(Clone, Copy)]
struct View {
    window: (i32, i32),
    scale: f64,
    top_strip: bool,
}

#[derive(Default)]
pub struct IslandWindow {
    /// Making the window focusable or not rewrites its styles, and with them the one that
    /// lets clicks through: the cursor thread applies it again on its next turn.
    styles_changed: AtomicBool,
    shapes: Mutex<Vec<IslandShape>>,
    placed: Mutex<Option<Placed>>,
    /// Bumped by every drag or move: an older motion stops at its next frame and the
    /// newer one carries on from where the pill is, at its speed.
    motion: AtomicU64,
    moving: AtomicBool,
    /// When to send the placement again to an interface that just loaded.
    announce_at: Mutex<Option<Instant>>,
    /// Window that had the keyboard focus before îlot took it (raw HWND value).
    previous_foreground: Mutex<Option<isize>>,
    /// The monitors, read once per second (prompt 12: « Suit la souris »).
    displays: Mutex<Vec<Display>>,
}

impl IslandWindow {
    pub fn set_shapes(&self, shapes: Vec<IslandShape>) {
        *self.shapes.lock().unwrap() = shapes.into_iter().take(MAX_SHAPES).collect();
    }

    fn shapes(&self) -> Vec<IslandShape> {
        self.shapes.lock().unwrap().clone()
    }

    /// The island is folded (or hidden): nothing open would be carried to another screen.
    fn folded(&self) -> bool {
        folded(&self.shapes.lock().unwrap())
    }

    fn displays(&self) -> Vec<Display> {
        self.displays.lock().unwrap().clone()
    }

    /// Half width of the folded pill, for magnets and edges.
    fn half_width(&self) -> f64 {
        self.shapes
            .lock()
            .unwrap()
            .first()
            .map(|s| s.width / 2.0)
            .filter(|w| *w > 0.0 && *w <= MAX_PILL_HALF_WIDTH)
            .unwrap_or(placement::PILL_HALF_WIDTH)
    }

    fn placed(&self) -> Option<Placed> {
        self.placed.lock().unwrap().clone()
    }

    fn view(&self) -> Option<View> {
        self.placed.lock().unwrap().as_ref().map(|p| View {
            window: p.window,
            scale: p.scale,
            top_strip: p.window.1 <= p.screen.work.y && placement::against_top(p.anchor.y),
        })
    }

    /// The placement sent to the interface (top center until the window is placed).
    pub fn placement(&self) -> Placement {
        self.placed.lock().unwrap().as_ref().map_or(
            Placement { anchor_x: placement::WINDOW_WIDTH / 2.0, anchor_y: 32.0, opens_down: true },
            |p| Placement { anchor_x: p.anchor.x, anchor_y: p.anchor.y, opens_down: p.opens_down },
        )
    }

    /// The screen the island rests on.
    pub fn screen(&self) -> Option<Screen> {
        self.placed.lock().unwrap().as_ref().map(|p| p.screen.clone())
    }

    /// The clickable shapes on the screen, in physical pixels.
    pub fn shapes_on_screen(&self) -> Vec<Rect> {
        let Some(view) = self.view() else { return Vec::new() };
        let (x, y) = (f64::from(view.window.0), f64::from(view.window.1));
        self.shapes()
            .iter()
            .map(|s| Rect {
                left: (x + s.x * view.scale).floor() as i32,
                top: (y + s.y * view.scale).floor() as i32,
                right: (x + (s.x + s.width) * view.scale).ceil() as i32,
                bottom: (y + (s.y + s.height) * view.scale).ceil() as i32,
            })
            .collect()
    }

    /// Where Mochi sits in the pill, on the screen (physical): `inset` logical pixels
    /// from the pill's left end, at mid-height; the anchor before any shape is known.
    pub fn mochi_slot(&self, inset: f64) -> Option<Point> {
        let placed = self.placed()?;
        let (x, y) = (f64::from(placed.window.0), f64::from(placed.window.1));
        let local = self.shapes().first().map_or(placed.anchor, |s| {
            Point::new(s.x + inset.min(s.width / 2.0), s.y + s.height / 2.0)
        });
        Some(Point::new(x + local.x * placed.scale, y + local.y * placed.scale))
    }

    fn next_motion(&self) -> u64 {
        self.moving.store(true, Ordering::SeqCst);
        self.motion.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn is_current(&self, motion: u64) -> bool {
        self.motion.load(Ordering::SeqCst) == motion
    }

    fn finish(&self, motion: u64) {
        if self.is_current(motion) {
            self.moving.store(false, Ordering::SeqCst);
        }
    }

    fn moving(&self) -> bool {
        self.moving.load(Ordering::SeqCst)
    }

    fn set_target(&self, target: Point) {
        if let Some(p) = self.placed.lock().unwrap().as_mut() {
            p.target = target;
        }
    }

    /// Sends the placement again shortly, once the interface listens.
    pub fn announce_soon(&self) {
        *self.announce_at.lock().unwrap() = Some(Instant::now() + ANNOUNCE_AFTER);
    }

    fn announce_due(&self) -> bool {
        let mut at = self.announce_at.lock().unwrap();
        let due = at.is_some_and(|t| Instant::now() >= t);
        if due {
            *at = None;
        }
        due
    }

    /// Remembers the window that has the keyboard focus, unless it is the island itself.
    fn remember_foreground(&self, own: isize) {
        // SAFETY: GetForegroundWindow has no preconditions.
        let foreground = unsafe { GetForegroundWindow() }.0 as isize;
        if foreground != 0 && foreground != own {
            *self.previous_foreground.lock().unwrap() = Some(foreground);
        }
    }

    /// Gives the keyboard focus to the island, remembering who had it. The window is
    /// otherwise not focusable: hovering and clicking never steal the user's keyboard.
    /// `typing`: the user clicked in one of the island's text fields. Windows may refuse
    /// the foreground to a window the user did not activate (it is never activated by a
    /// click, by design): îlot then joins the input of the window in front for a moment,
    /// the documented way (AttachThreadInput), without any simulated key, and the page
    /// inside takes the focus, or the keys would stop at the window. Only then: anywhere
    /// else (the hub opening…), îlot asks once and never insists.
    pub fn take_focus(&self, window: &WebviewWindow, typing: bool) -> tauri::Result<()> {
        let hwnd = window.hwnd()?;
        self.remember_foreground(hwnd.0 as isize);
        window.set_focusable(true)?;
        self.styles_changed.store(true, Ordering::SeqCst);
        window.set_focus()?;
        if !typing {
            return Ok(());
        }
        // SAFETY: plain Win32 calls on the island's own window, on its UI thread (a
        // synchronous command); the input queues are detached right after.
        unsafe {
            let front = GetForegroundWindow();
            if front != hwnd {
                let front_thread = GetWindowThreadProcessId(front, None);
                let own = GetCurrentThreadId();
                let joined = front_thread != 0 && front_thread != own && AttachThreadInput(own, front_thread, true).as_bool();
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);
                if joined {
                    let _ = AttachThreadInput(own, front_thread, false);
                }
            }
        }
        focus_page(window);
        Ok(())
    }

    /// Hands the keyboard focus back to the last window that had it before the island.
    pub fn give_back_focus(&self, window: &WebviewWindow) {
        let _ = window.set_focusable(false);
        self.styles_changed.store(true, Ordering::SeqCst);
        let Some(raw) = *self.previous_foreground.lock().unwrap() else { return };
        let hwnd = HWND(raw as *mut _);
        // SAFETY: IsWindow accepts any value; SetForegroundWindow only runs on a live window.
        unsafe {
            if IsWindow(Some(hwnd)).as_bool() {
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

/// The keyboard to the page inside the window (WebView2 keeps its own focus).
fn focus_page(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| {
        // SAFETY: a COM call on the island's own webview controller, on its thread.
        unsafe {
            let _ = webview.controller().MoveFocus(webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
        }
    });
}

/// True when the shapes are the folded pill and its music bubble, or none at all.
fn folded(shapes: &[IslandShape]) -> bool {
    shapes.iter().all(|s| s.height <= FOLDED_HEIGHT && s.width <= 2.0 * MAX_PILL_HALF_WIDTH + 2.0 * FOLDED_HEIGHT)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= f64::from(self.left) && x < f64::from(self.right) && y >= f64::from(self.top) && y < f64::from(self.bottom)
    }
}

/// A window is full screen when it covers its whole monitor.
pub fn covers(window: Rect, monitor: Rect) -> bool {
    window.left <= monitor.left
        && window.top <= monitor.top
        && window.right >= monitor.right
        && window.bottom >= monitor.bottom
}

pub fn rect(r: RECT) -> Rect {
    Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

/// Class name of a window (desktop and taskbar are recognized by it).
pub fn class_name(hwnd: HWND) -> String {
    let mut class = [0u16; 64];
    // SAFETY: GetClassNameW writes at most the buffer's length.
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    String::from_utf16_lossy(&class[..len.max(0) as usize])
}

/// Whether a press keeps the window solid: it started on one of the island's shapes (the
/// button went down there) and is still held. A press that started elsewhere never does.
fn held_on_island(grabbed: bool, was_down: bool, down: bool, on_a_shape: bool) -> bool {
    down && (grabbed || (!was_down && on_a_shape))
}

/// The primary mouse button is down. GetAsyncKeyState reads the physical button, so a
/// swapped mouse (left-handed setting) is read on its right button.
/// https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getasynckeystate
pub fn primary_button_down() -> bool {
    // SAFETY: plain queries with no pointer.
    unsafe {
        let key = if GetSystemMetrics(SM_SWAPBUTTON) != 0 { VK_RBUTTON } else { VK_LBUTTON };
        GetAsyncKeyState(i32::from(key.0)) < 0
    }
}

/// Windows « Show animations » is off: the island jumps instead of gliding, as the
/// interface does with `prefers-reduced-motion`.
/// https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow
fn reduced_motion() -> bool {
    let mut enabled = BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL into the local it points to.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut enabled).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_ok() && !enabled.as_bool()
}

fn screen_of(monitor: &Monitor) -> Screen {
    let work = monitor.work_area();
    Screen {
        name: monitor.name().cloned().unwrap_or_default(),
        work: Area {
            x: work.position.x,
            y: work.position.y,
            width: work.size.width as i32,
            height: work.size.height as i32,
        },
        scale: monitor.scale_factor(),
    }
}

/// The monitors as the island sees them, the main one marked.
fn displays_of(window: &WebviewWindow) -> Vec<Display> {
    let Ok(monitors) = window.available_monitors() else { return Vec::new() };
    let primary = window.primary_monitor().ok().flatten().and_then(|m| m.name().cloned());
    monitors
        .iter()
        .map(|m| Display {
            screen: screen_of(m),
            bounds: Area {
                x: m.position().x,
                y: m.position().y,
                width: m.size().width as i32,
                height: m.size().height as i32,
            },
            primary: m.name().is_some() && m.name() == primary.as_ref(),
        })
        .collect()
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(WINDOW_LABEL)
}

/// Puts the window around a resting pill, tells the interface where the anchor is and,
/// after a move by the user, remembers the place.
fn settle(app: &AppHandle, window: &WebviewWindow, screen: &Screen, center: Point, save: bool) {
    let frame = placement::frame(screen, center);
    let island = app.state::<IslandWindow>();
    *island.placed.lock().unwrap() = Some(Placed {
        screen: screen.clone(),
        center,
        target: center,
        speed: Point::default(),
        anchor: frame.anchor,
        opens_down: frame.opens_down,
        window: frame.window,
        scale: screen.scale,
    });
    // The anchor first, then the window: both land within the same frame or two.
    let _ = app.emit_to(WINDOW_LABEL, EVENT_PLACED, island.placement());
    let _ = window.set_position(PhysicalPosition::new(frame.window.0, frame.window.1));
    if save {
        let share = placement::to_share(screen.work, center);
        let saved = app.state::<SettingsStore>().update(|settings| {
            settings.island = Some(IslandPosition { monitor: screen.name.clone(), x: share.x, y: share.y });
        });
        if let Err(error) = saved {
            eprintln!("îlot: island position not saved: {error}");
        }
    }
}

/// Moves the window so that the pill's center is at `center`, the anchor unchanged
/// (the window may leave the screen for a moment; `settle` brings it back).
fn carry(island: &IslandWindow, window: &WebviewWindow, center: Point, speed: Point) {
    // Read before locking: a getter waits for the main thread.
    let scale = window.scale_factor().unwrap_or(1.0);
    let origin = {
        let mut placed = island.placed.lock().unwrap();
        let Some(p) = placed.as_mut() else { return };
        p.center = center;
        p.speed = speed;
        p.scale = scale;
        let origin = placement::window_at(center, p.anchor, scale);
        if origin == p.window {
            return;
        }
        p.window = origin;
        origin
    };
    let _ = window.set_position(PhysicalPosition::new(origin.0, origin.1));
}

/// Back to the saved place (or the top center of the main screen).
fn restore(app: &AppHandle, window: &WebviewWindow) {
    let Ok(monitors) = window.available_monitors() else { return };
    let half_width = app.state::<IslandWindow>().half_width();
    let saved = app.state::<SettingsStore>().get().island;
    let found = saved.and_then(|s| {
        let screen = monitors.iter().map(screen_of).find(|screen| screen.name == s.monitor)?;
        let center = placement::from_share(screen.work, Point::new(s.x, s.y))?;
        Some((placement::clamp_center(&screen, half_width, center), screen))
    });
    let (center, screen) = match found {
        Some(found) => found,
        None => {
            let Ok(Some(primary)) = window.primary_monitor() else { return };
            let screen = screen_of(&primary);
            (placement::top_center(&screen, half_width), screen)
        }
    };
    settle(app, window, &screen, center, false);
}

/// Once per second: a screen unplugged, a new resolution or work area brings the island
/// back to its saved place; a window moved by someone else is put back.
fn keep_in_place(app: &AppHandle, window: &WebviewWindow) {
    let island = app.state::<IslandWindow>();
    if island.moving() {
        return;
    }
    let Some(placed) = island.placed() else {
        restore(app, window);
        return;
    };
    let Ok(monitors) = window.available_monitors() else { return };
    if !monitors.iter().map(screen_of).any(|screen| screen == placed.screen) {
        restore(app, window);
        return;
    }
    if let Ok(position) = window.outer_position()
        && (position.x, position.y) != placed.window
    {
        let _ = window.set_position(PhysicalPosition::new(placed.window.0, placed.window.1));
    }
}

/// The screen under a point, if any.
fn screen_at(app: &AppHandle, p: Point) -> Option<Screen> {
    app.monitor_from_point(p.x, p.y).ok().flatten().map(|m| screen_of(&m))
}

/// The pill follows the cursor 1:1 until the button is released, then is thrown.
fn drag(app: &AppHandle, motion: u64) {
    let island = app.state::<IslandWindow>();
    let (Some(window), Some(start), Ok(cursor)) = (main_window(app), island.placed(), app.cursor_position()) else {
        island.finish(motion);
        return;
    };
    if !primary_button_down() {
        island.finish(motion);
        return;
    }
    let grab = Point::new(cursor.x - start.center.x, cursor.y - start.center.y);
    let half_width = island.half_width();
    let _ = app.emit_to(WINDOW_LABEL, EVENT_DRAG, Dragging { dragging: true });
    let clock = Instant::now();
    let mut moves: VecDeque<(f64, Point)> = VecDeque::new();
    let mut center = start.center;
    while island.is_current(motion) {
        if let Ok(cursor) = app.cursor_position() {
            center = Point::new(cursor.x - grab.x, cursor.y - grab.y);
            carry(&island, &window, center, Point::default());
            let now = clock.elapsed().as_secs_f64();
            moves.push_back((now, center));
            while moves.len() > 2 && moves.front().is_some_and(|(t, _)| now - t > SPEED_WINDOW_S) {
                moves.pop_front();
            }
        }
        if !primary_button_down() {
            break;
        }
        std::thread::sleep(DRAG_TICK);
    }
    let _ = app.emit_to(WINDOW_LABEL, EVENT_DRAG, Dragging { dragging: false });
    if !island.is_current(motion) {
        return;
    }
    let speed = placement::release_speed(moves.make_contiguous());
    let screen = screen_at(app, center).unwrap_or(start.screen);
    let always = app.state::<SettingsStore>().get().modes.position == MAGNETIC;
    let target = placement::drop_target(&screen, half_width, center, speed, always);
    glide(app, &window, motion, &screen, Point::new(speed.x * CARRY, speed.y * CARRY), target);
}

/// Carries the pill to `target` on the demo's spring, then settles and saves.
fn glide(app: &AppHandle, window: &WebviewWindow, motion: u64, screen: &Screen, speed: Point, target: Point) {
    let island = app.state::<IslandWindow>();
    let Some(from) = island.placed() else {
        island.finish(motion);
        return;
    };
    island.set_target(target);
    if !reduced_motion() {
        let mut x = Spring::new(from.center.x, target.x, speed.x);
        let mut y = Spring::new(from.center.y, target.y, speed.y);
        let mut last = Instant::now();
        loop {
            std::thread::sleep(FRAME);
            if !island.is_current(motion) {
                return;
            }
            let seconds = last.elapsed().as_secs_f64().min(MAX_STEP_S);
            last = Instant::now();
            let moving = x.step(seconds) | y.step(seconds);
            carry(&island, window, Point::new(x.value, y.value), Point::new(x.velocity, y.velocity));
            if !moving {
                break;
            }
        }
    }
    if island.is_current(motion) {
        settle(app, window, screen, target, true);
        island.finish(motion);
    }
}

fn spawn_motion(app: AppHandle, motion: impl FnOnce(&AppHandle) + Send + 'static) -> Result<(), String> {
    std::thread::Builder::new()
        .name("ilot-motion".into())
        .spawn(move || motion(&app))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Glides from where the island is (at its current speed) to `target` on its screen.
fn move_island(app: AppHandle, island: &IslandWindow, to: impl FnOnce(&Screen, Point) -> Point) -> Result<(), String> {
    let Some(placed) = island.placed() else { return Ok(()) };
    let target = to(&placed.screen, placed.target);
    let motion = island.next_motion();
    spawn_motion(app, move |app| {
        if let Some(window) = main_window(app) {
            glide(app, &window, motion, &placed.screen, placed.speed, target);
        } else {
            app.state::<IslandWindow>().finish(motion);
        }
    })
}

/// The interface caught the folded pill: the core carries it until the button is released.
#[tauri::command]
pub fn island_drag_start(app: AppHandle, island: State<'_, IslandWindow>) -> Result<(), String> {
    let motion = island.next_motion();
    spawn_motion(app, move |app| drag(app, motion))
}

/// Alt + arrows: moves the island by a step (logical pixels), kept on its screen.
#[tauri::command]
pub fn island_move_by(dx: f64, dy: f64, app: AppHandle, island: State<'_, IslandWindow>) -> Result<(), String> {
    if !dx.is_finite() || !dy.is_finite() {
        return Err(Text::MoveInvalid.into());
    }
    let (dx, dy) = (dx.clamp(-MAX_MOVE, MAX_MOVE), dy.clamp(-MAX_MOVE, MAX_MOVE));
    let half_width = island.half_width();
    move_island(app, &island, |screen, from| {
        placement::clamp_center(screen, half_width, Point::new(from.x + dx * screen.scale, from.y + dy * screen.scale))
    })
}

/// « Recentrer l'île »: back to the top center of its screen.
#[tauri::command]
pub fn island_recenter(app: AppHandle, island: State<'_, IslandWindow>) -> Result<(), String> {
    let half_width = island.half_width();
    move_island(app, &island, |screen, _| placement::top_center(screen, half_width))
}

/// Where the pill sits in its window now, for an interface that just loaded.
#[tauri::command]
pub fn island_placement(island: State<'_, IslandWindow>) -> Placement {
    island.placement()
}

/// Puts the island on another screen, at the same place on it, and remembers it there.
fn move_to_screen(app: &AppHandle, window: &WebviewWindow, to: &Screen) {
    let island = app.state::<IslandWindow>();
    let Some(placed) = island.placed() else { return };
    if placed.screen.name == to.name {
        return;
    }
    // A glide still under way stops: the island leaves from where it was heading.
    let motion = island.next_motion();
    let center = screens::same_place(&placed.screen, to, placed.target, island.half_width());
    settle(app, window, to, center, true);
    island.finish(motion);
}

/// The screens for Réglages › Général, the island's one marked.
#[tauri::command]
pub fn list_screens(app: AppHandle, island: State<'_, IslandWindow>) -> Vec<ScreenView> {
    let Some(window) = main_window(&app) else { return Vec::new() };
    let current = island.screen().map(|s| s.name);
    screens::views(&displays_of(&window), current.as_deref())
}

/// Réglages › Général: the island goes to this screen (one of the system's monitors).
#[tauri::command]
pub fn island_to_screen(id: String, app: AppHandle) -> Result<(), String> {
    let window = main_window(&app).ok_or(Text::IslandWindowMissing)?;
    let display = displays_of(&window).into_iter().find(|d| d.screen.name == id).ok_or(Text::ScreenMissing)?;
    move_to_screen(&app, &window, &display.screen);
    Ok(())
}

/// True when the foreground app is full screen on the island's monitor. Windows' own
/// heuristic (the one that silences notifications) decides; the window rectangle then
/// tells on which monitor, so a maximized window is never mistaken for a full-screen one.
fn fullscreen_on_island_monitor(window: &WebviewWindow) -> bool {
    let Ok(own) = window.hwnd() else { return false };
    let own = HWND(own.0);
    // SAFETY: plain Win32 queries on handles returned by the system; every out-parameter
    // is a properly sized local.
    unsafe {
        let busy = matches!(
            SHQueryUserNotificationState(),
            Ok(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
        );
        if !busy {
            return false;
        }
        let foreground = GetForegroundWindow();
        if foreground.is_invalid() || foreground == own {
            return false;
        }
        if SHELL_CLASSES.contains(&class_name(foreground).as_str()) {
            return false;
        }
        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
        if monitor != MonitorFromWindow(own, MONITOR_DEFAULTTONEAREST) {
            return false;
        }
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        let mut bounds = RECT::default();
        GetMonitorInfoW(monitor, &mut info).as_bool()
            && GetWindowRect(foreground, &mut bounds).is_ok()
            && covers(rect(bounds), rect(info.rcMonitor))
    }
}

/// Places and shows the window, then follows the cursor to switch click-through on and
/// off and to report it to the interface.
pub fn start(app: &AppHandle) -> tauri::Result<()> {
    let window = main_window(app).ok_or(tauri::Error::WindowNotFound)?;
    window.set_ignore_cursor_events(true)?;
    window.set_size(LogicalSize::new(placement::WINDOW_WIDTH, placement::WINDOW_HEIGHT))?;
    restore(app, &window);
    window.show()?;
    let app = app.clone();
    std::thread::Builder::new()
        .name("ilot-cursor".into())
        .spawn(move || watch(&app))?;
    Ok(())
}

/// Follows the cursor until the window is gone. Should a step panic, the window lets
/// clicks through again at once and the watch starts over: a frozen state would leave the
/// island blocking the screen below it, or deaf to the user.
fn watch(app: &AppHandle) {
    while std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| follow_cursor(app))).is_err() {
        eprintln!("îlot: cursor watch restarted after a panic");
        if let Some(window) = main_window(app) {
            let _ = window.set_ignore_cursor_events(true);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn follow_cursor(app: &AppHandle) {
    let own = main_window(app).and_then(|w| w.hwnd().ok()).map_or(0, |h| h.0 as isize);
    let mut ignoring = true;
    // A press that starts on the island keeps the window solid until it is released:
    // pulling Mochi or the music bubble out goes past the island's shape.
    let mut was_down = false;
    let mut grabbed = false;
    let mut last: Option<Pointer> = None;
    let mut hidden = false;
    let mut checked: Option<Instant> = None;
    let mut display = DisplaySettings::default();
    let mut follow = Follow::default();
    loop {
        let Some(window) = main_window(app) else { return };
        let island = app.state::<IslandWindow>();
        if checked.is_none_or(|t| t.elapsed() >= SLOW_CHECKS) {
            checked = Some(Instant::now());
            keep_in_place(app, &window);
            let settings = app.state::<SettingsStore>().get();
            display = settings.display;
            *island.displays.lock().unwrap() = displays_of(&window);
            let hide = settings.hide_in_fullscreen && fullscreen_on_island_monitor(&window);
            if hide != hidden {
                let _ = if hide { window.hide() } else { window.show() };
                hidden = hide;
                buddy::fullscreen(app, hide);
            }
        }
        if island.announce_due() {
            let _ = app.emit_to(WINDOW_LABEL, EVENT_PLACED, island.placement());
        }
        island.remember_foreground(own);
        let cursor = app.cursor_position().ok().map(|c| Point::new(c.x, c.y));
        let mut pause = pause_for(f64::INFINITY);
        if buddy::report_cursor(app, cursor) || island.moving() {
            pause = FRAME;
        }
        if let Some(distance) = buddy::hit_test(app, cursor) {
            pause = pause.min(pause_for(distance));
        }
        if let (Some(cursor), Some(placed), false) = (cursor, island.placed(), hidden) {
            // « Suit la souris »: the folded island goes to the screen the cursor stays on.
            if display.follow_mouse && !island.moving() && island.folded() {
                let displays = island.displays();
                let under = screens::display_under(&displays, cursor).map(|d| d.screen.name.as_str());
                if let Some(name) = follow.update(&placed.screen.name, under, Instant::now())
                    && let Some(to) = displays.iter().find(|d| d.screen.name == name)
                {
                    move_to_screen(app, &window, &to.screen);
                }
            }
        }
        if let (Some(cursor), Some(view), false) = (cursor, island.view(), hidden) {
            let x = (cursor.x - f64::from(view.window.0)) / view.scale;
            let y = (cursor.y - f64::from(view.window.1)) / view.scale;
            let shapes = island.shapes();
            let on_a_shape = shapes.iter().enumerate().any(|(i, s)| s.contains(x, y, i == 0 && view.top_strip));
            let down = primary_button_down();
            grabbed = held_on_island(grabbed, was_down, down, on_a_shape);
            was_down = down;
            let solid = on_a_shape || grabbed;
            if ignoring == solid || island.styles_changed.swap(false, Ordering::SeqCst) {
                ignoring = !solid;
                let _ = window.set_ignore_cursor_events(ignoring);
            }
            if let Some(first) = shapes.first() {
                let distance = first.distance(x, y);
                let now = Pointer { x, y, inside: first.contains(x, y, view.top_strip), near: distance <= NEAR };
                if worth_sending(last, now) {
                    let _ = app.emit_to(WINDOW_LABEL, EVENT_POINTER, now);
                    last = Some(now);
                }
                pause = pause.min(pause_for(distance));
            }
        }
        std::thread::sleep(pause);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_changes_ask_for_the_click_through_style_again() {
        let island = IslandWindow::default();
        assert!(!island.styles_changed.load(Ordering::SeqCst));
        island.styles_changed.store(true, Ordering::SeqCst);
        assert!(island.styles_changed.swap(false, Ordering::SeqCst), "the cursor thread sees it once");
        assert!(!island.styles_changed.load(Ordering::SeqCst));
    }

    const PILL: IslandShape = IslandShape { x: 380.0, y: 12.0, width: 220.0, height: 40.0, radius: 20.0 };

    #[test]
    fn a_press_begun_on_the_island_keeps_it_solid_until_released() {
        // Pressed on the pill, then pulled off it: still held.
        let grabbed = held_on_island(false, false, true, true);
        assert!(grabbed);
        assert!(held_on_island(grabbed, true, true, false));
        // Released: clicks pass through again.
        assert!(!held_on_island(grabbed, true, false, false));
        // A press begun elsewhere (another app's drag) never holds the island.
        assert!(!held_on_island(false, true, true, true));
        assert!(!held_on_island(false, false, true, false));
    }

    #[test]
    fn center_of_the_island_is_inside() {
        assert!(PILL.contains(490.0, 32.0, false));
    }

    #[test]
    fn rounded_corners_let_clicks_through() {
        assert!(!PILL.contains(381.0, 13.0, false));
        assert!(PILL.contains(386.0, 32.0, false));
    }

    #[test]
    fn strip_above_the_island_counts_only_against_the_top_of_the_screen() {
        assert!(PILL.contains(490.0, 0.0, true));
        assert!(PILL.contains(490.0, 5.0, true));
        assert!(!PILL.contains(490.0, -1.0, true));
        assert!(!PILL.contains(300.0, 2.0, true));
        assert!(!PILL.contains(490.0, 5.0, false), "away from the top edge, nothing above the island");
    }

    #[test]
    fn points_beside_or_below_are_outside() {
        assert!(!PILL.contains(379.0, 32.0, true));
        assert!(!PILL.contains(601.0, 32.0, true));
        assert!(!PILL.contains(490.0, 53.0, true));
    }

    #[test]
    fn empty_shape_contains_nothing() {
        assert!(!IslandShape::default().contains(0.0, 0.0, true));
    }

    #[test]
    fn radius_larger_than_half_height_is_clamped() {
        let shape = IslandShape { radius: 99.0, ..PILL };
        assert!(shape.contains(490.0, 32.0, false));
        assert!(shape.contains(382.0, 32.0, false));
    }

    #[test]
    fn shapes_come_as_a_json_list() {
        let shapes: Vec<IslandShape> = serde_json::from_str(
            r#"[{"x":380,"y":12,"width":220,"height":40,"radius":20},{"x":30,"y":68,"width":920,"height":620,"radius":24}]"#,
        )
        .unwrap();
        assert_eq!(shapes[0], PILL);
        let island = IslandWindow::default();
        island.set_shapes(vec![PILL; 9]);
        assert_eq!(island.shapes().len(), MAX_SHAPES);
    }

    #[test]
    fn distance_to_the_island() {
        assert_eq!(PILL.distance(490.0, 32.0), 0.0);
        assert_eq!(PILL.distance(490.0, 152.0), 100.0);
        assert_eq!(PILL.distance(300.0, 12.0), 80.0);
        assert_eq!(PILL.distance(600.0 + 30.0, 52.0 + 40.0), 50.0);
    }

    #[test]
    fn near_the_island_every_move_is_sent_elsewhere_only_changes() {
        let near = Pointer { x: 500.0, y: 80.0, inside: false, near: true };
        assert!(worth_sending(None, near));
        assert!(worth_sending(Some(near), Pointer { x: 501.0, ..near }));
        assert!(!worth_sending(Some(near), near));
        let far = Pointer { x: 900.0, y: 600.0, inside: false, near: false };
        assert!(worth_sending(Some(near), far), "leaving the neighborhood is told once");
        assert!(!worth_sending(Some(far), Pointer { x: 10.0, ..far }));
        let inside = Pointer { x: 490.0, y: 30.0, inside: true, near: true };
        assert!(worth_sending(Some(near), inside));
    }

    #[test]
    fn a_distant_cursor_is_read_less_often() {
        assert_eq!(pause_for(0.0), FRAME);
        assert_eq!(pause_for(160.0), FRAME);
        assert_eq!(pause_for(300.0), Duration::from_millis(50));
        assert_eq!(pause_for(f64::INFINITY), Duration::from_millis(100));
    }

    #[test]
    fn the_folded_pill_gives_the_magnets_their_width() {
        let island = IslandWindow::default();
        assert_eq!(island.half_width(), placement::PILL_HALF_WIDTH);
        island.set_shapes(vec![IslandShape { width: 260.0, ..PILL }]);
        assert_eq!(island.half_width(), 130.0);
        island.set_shapes(vec![IslandShape { width: 620.0, ..PILL }]);
        assert_eq!(island.half_width(), placement::PILL_HALF_WIDTH, "an opened island is not a pill");
    }

    #[test]
    fn before_any_placement_the_island_is_announced_at_the_top_center() {
        let island = IslandWindow::default();
        assert_eq!(island.placement(), Placement { anchor_x: 490.0, anchor_y: 32.0, opens_down: true });
        assert!(island.mochi_slot(24.0).is_none());
    }

    #[test]
    fn the_announce_waits_for_its_delay_and_happens_once() {
        let island = IslandWindow::default();
        assert!(!island.announce_due());
        *island.announce_at.lock().unwrap() = Some(Instant::now());
        assert!(island.announce_due());
        assert!(!island.announce_due());
        island.announce_soon();
        assert!(!island.announce_due());
    }

    #[test]
    fn a_newer_motion_takes_over() {
        let island = IslandWindow::default();
        let first = island.next_motion();
        let second = island.next_motion();
        assert!(!island.is_current(first) && island.is_current(second));
        island.finish(first);
        assert!(island.moving(), "the old motion does not stop the new one");
        island.finish(second);
        assert!(!island.moving());
    }

    #[test]
    fn shapes_and_mochi_on_the_screen() {
        let island = IslandWindow::default();
        let screen = Screen { name: "A".into(), work: Area { x: 0, y: 0, width: 1920, height: 1040 }, scale: 1.5 };
        *island.placed.lock().unwrap() = Some(Placed {
            screen,
            center: Point::new(960.0, 48.0),
            target: Point::new(960.0, 48.0),
            speed: Point::default(),
            anchor: Point::new(490.0, 32.0),
            opens_down: true,
            window: (225, 0),
            scale: 1.5,
        });
        island.set_shapes(vec![PILL]);
        assert_eq!(island.shapes_on_screen(), vec![Rect { left: 795, top: 18, right: 1125, bottom: 78 }]);
        assert_eq!(island.mochi_slot(24.0), Some(Point::new(225.0 + 404.0 * 1.5, 48.0)));
        assert!(island.view().unwrap().top_strip);
    }

    #[test]
    fn a_pill_left_lower_does_not_take_the_strip_above_it() {
        // A screen shorter than the window: the window stays at the top, the pill lower.
        let island = IslandWindow::default();
        let screen = Screen { name: "A".into(), work: Area { x: 0, y: 0, width: 1366, height: 728 }, scale: 1.0 };
        *island.placed.lock().unwrap() = Some(Placed {
            screen,
            center: Point::new(830.0, 128.0),
            target: Point::new(830.0, 128.0),
            speed: Point::default(),
            anchor: Point::new(490.0, 128.0),
            opens_down: true,
            window: (340, 0),
            scale: 1.0,
        });
        let view = island.view().unwrap();
        assert!(!view.top_strip);
        let pill = IslandShape { x: 410.0, y: 108.0, width: 160.0, height: 40.0, radius: 20.0 };
        assert!(!pill.contains(490.0, 20.0, view.top_strip));
        assert!(pill.contains(490.0, 128.0, view.top_strip));
    }

    #[test]
    fn only_a_folded_island_follows_the_cursor_to_another_screen() {
        let bubble = IslandShape { x: 610.0, y: 12.0, width: 190.0, height: 40.0, radius: 20.0 };
        assert!(folded(&[]), "hidden: nothing open");
        assert!(folded(&[PILL]));
        assert!(folded(&[PILL, bubble]), "the music bubble is part of the folded island");
        assert!(!folded(&[IslandShape { width: 440.0, height: 64.0, ..PILL }]), "a banner");
        assert!(!folded(&[PILL, IslandShape { x: 130.0, y: 68.0, width: 720.0, height: 250.0, radius: 24.0 }]), "the hub");
    }

    #[test]
    fn full_screen_detection() {
        let monitor = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };
        assert!(covers(monitor, monitor));
        assert!(covers(Rect { left: -8, top: -8, right: 1928, bottom: 1088 }, monitor));
        assert!(!covers(Rect { left: 0, top: 0, right: 1920, bottom: 1040 }, monitor));
    }
}
