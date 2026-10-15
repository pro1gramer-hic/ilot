//! The mini-player and the page of the YouTube player, apart from the island.
//!
//! - `mini`: îlot's own page (`mini.html`), always on top, frameless and draggable. Its
//!   capability (capabilities/mini.json) grants the media commands and nothing else.
//! - `youtube`: the window itself lives in video.rs, with Crunchyroll's. Its page comes
//!   from the `ilot-player` protocol below, with its own CSP that only allows the
//!   youtube-nocookie.com frame: the island keeps its strict CSP, without remote content.
//!   A page of our own also gives the embed the `Referer` that YouTube requires
//!   (developers.google.com/youtube/terms/required-minimum-functionality).
//!
//! The mini-player and the full YouTube window remember where the user left them
//! (settings.json, atomic write).

use crate::locale::{Text, tr};
use crate::media::is_video_id;
use crate::settings::{Settings, SettingsStore, WindowPosition};
use crate::video::{self, Site};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};

pub const MINI_LABEL: &str = "mini";
pub const VIDEO_LABEL: &str = "youtube";
pub const PLAYER_SCHEME: &str = "ilot-player";
/// Host of the `ilot-player` pages in WebView2 (`use_https_scheme`).
const PLAYER_HOST: &str = "ilot-player.localhost";

/// The mini-player's pill (300 × 64) plus room for its shadow, in logical pixels; the
/// page grows it to 400 × 246 for a video.
pub const MINI_SIZE: (f64, f64) = (348.0, 112.0);
/// Gap between a player and the screen edges, in logical pixels.
const MARGIN: f64 = 24.0;
/// Highest start time accepted, in seconds (24 h).
pub const MAX_START: u32 = 86_400;
/// A move is saved once the window has stopped moving for this long.
const SAVE_AFTER: Duration = Duration::from_millis(600);
/// Windows parks minimized windows at this position: never saved.
const MINIMIZED: i32 = -32_000;

const PLAYER_CSP: &str = "default-src 'none'; frame-src https://www.youtube-nocookie.com; \
     style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'";

// ---------------------------------------------------------------------------------------
// YouTube page (pure, tested)

/// Official privacy-enhanced embed URL of a video.
pub fn embed_url(video_id: &str, start: u32) -> String {
    format!("https://www.youtube-nocookie.com/embed/{video_id}?autoplay=1&rel=0&start={start}")
}

/// Page of the YouTube window that holds the embed of this video.
pub fn player_url(video_id: &str, start: u32) -> String {
    format!("https://{PLAYER_HOST}/embed/{video_id}?start={start}")
}

/// The YouTube window may only show its own player page.
pub fn is_player_url(url: &Url) -> bool {
    url.host_str() == Some(PLAYER_HOST) && url.path().starts_with("/embed/")
}

/// Video and start time asked by a player page request, when valid.
pub fn parse_player_request(path: &str, query: Option<&str>) -> Option<(String, u32)> {
    let id = path.strip_prefix("/embed/")?;
    if !is_video_id(id) {
        return None;
    }
    let start = query
        .into_iter()
        .flat_map(|q| q.split('&'))
        .find_map(|pair| pair.strip_prefix("start="))
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    Some((id.to_owned(), start.min(MAX_START)))
}

/// Player page: one frame, no script. The id is checked, so nothing can be injected.
pub fn page_html(video_id: &str, start: u32) -> String {
    format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>YouTube · îlot</title>\
         <style>html,body{{margin:0;height:100%;background:#000;overflow:hidden}}\
         iframe{{display:block;width:100%;height:100%;border:0}}</style></head><body>\
         <iframe src=\"{}\" title=\"Lecteur YouTube\" allow=\"autoplay; encrypted-media; picture-in-picture; fullscreen\" \
         allowfullscreen referrerpolicy=\"strict-origin-when-cross-origin\"></iframe></body></html>",
        embed_url(video_id, start).replace('&', "&amp;")
    )
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = code;
    response
}

/// Answers the `ilot-player` protocol, for the YouTube window only.
pub fn serve_player(webview_label: &str, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    if webview_label != VIDEO_LABEL {
        return status(StatusCode::FORBIDDEN);
    }
    let Some((id, start)) = parse_player_request(request.uri().path(), request.uri().query()) else {
        return status(StatusCode::NOT_FOUND);
    };
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CONTENT_SECURITY_POLICY, PLAYER_CSP)
        .header(header::REFERRER_POLICY, "strict-origin-when-cross-origin")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "no-store")
        .body(page_html(&id, start).into_bytes())
        .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

// ---------------------------------------------------------------------------------------
// Placement (pure, tested)

/// Rectangle in physical pixels (a monitor's work area).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// A saved position is kept only if the top of the window, where it is grabbed, is
/// still on a screen (a monitor may have been unplugged since).
pub fn is_reachable(position: WindowPosition, width: i32, areas: &[Area]) -> bool {
    const GRAB: i32 = 48;
    areas.iter().any(|a| {
        let left = position.x.max(a.x);
        let right = (position.x + width).min(a.x + a.width);
        let top_inside = position.y >= a.y && position.y + GRAB / 2 <= a.y + a.height;
        right - left >= GRAB && top_inside
    })
}

/// Bottom-right corner of a work area, `offset` further left (to sit beside another player).
pub fn bottom_right(area: Area, size: (i32, i32), margin: i32, offset: i32) -> WindowPosition {
    WindowPosition {
        x: (area.x + area.width - size.0 - margin - offset).max(area.x),
        y: (area.y + area.height - size.1 - margin).max(area.y),
    }
}

pub fn is_minimized(position: WindowPosition) -> bool {
    position.x <= MINIMIZED && position.y <= MINIMIZED
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Mini,
    Video,
}

pub fn with_position(settings: &mut Settings, slot: Slot, position: WindowPosition) {
    match slot {
        Slot::Mini => settings.mini_player = Some(position),
        Slot::Video => settings.video_player = Some(position),
    }
}

// ---------------------------------------------------------------------------------------
// Windows

fn area(monitor: &Monitor) -> Area {
    let work = monitor.work_area();
    Area {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width as i32,
        height: work.size.height as i32,
    }
}

/// Puts the window where the user left it, or else at the bottom right of the main screen.
pub fn place(window: &WebviewWindow, saved: Option<WindowPosition>, offset: f64) -> tauri::Result<()> {
    let size = window.outer_size()?;
    let size = (size.width as i32, size.height as i32);
    let areas: Vec<Area> = window.available_monitors()?.iter().map(area).collect();
    let position = match saved {
        Some(position) if is_reachable(position, size.0, &areas) => position,
        _ => {
            let Some(primary) = window.primary_monitor()? else { return Ok(()) };
            let scale = primary.scale_factor();
            bottom_right(area(&primary), size, (MARGIN * scale) as i32, (offset * scale) as i32)
        }
    };
    window.set_position(PhysicalPosition::new(position.x, position.y))
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Opens the mini-player, or brings it back if it is already open.
#[tauri::command]
pub async fn open_mini_player(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MINI_LABEL) {
        return window.show().map_err(error);
    }
    let window = WebviewWindowBuilder::new(&app, MINI_LABEL, WebviewUrl::App("mini.html".into()))
        .title(tr(Text::MiniPlayerTitle))
        .inner_size(MINI_SIZE.0, MINI_SIZE.1)
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
        .map_err(error)?;
    place(&window, app.state::<SettingsStore>().get().mini_player, 0.0).map_err(error)?;
    window.show().map_err(error)
}

#[tauri::command]
pub fn close_mini_player(app: AppHandle) {
    if let Some(window) = app.get_webview_window(MINI_LABEL) {
        let _ = window.destroy();
    }
}

/// Brings îlot's YouTube window back in front, at its full size (« Afficher la vidéo »).
pub fn show_video_window(app: &AppHandle) -> Result<(), String> {
    video::show_full(app, Site::YouTube)
}

/// Saves player positions in the background, once a drag is over.
pub struct PositionSaver(Sender<(Slot, WindowPosition)>);

/// Saves a player's position once the user stops moving it.
pub fn save_position(app: &AppHandle, slot: Slot, position: WindowPosition) {
    let _ = app.state::<PositionSaver>().0.send((slot, position));
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let (tx, rx) = mpsc::channel::<(Slot, WindowPosition)>();
    app.manage(PositionSaver(tx));
    let app = app.clone();
    std::thread::Builder::new().name("ilot-positions".into()).spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut moves = vec![first];
            while let Ok(next) = rx.recv_timeout(SAVE_AFTER) {
                moves.push(next);
            }
            let saved = app.state::<SettingsStore>().update(|settings| {
                for (slot, position) in moves {
                    with_position(settings, slot, position);
                }
            });
            if let Err(error) = saved {
                eprintln!("îlot: player position not saved: {error}");
            }
        }
    })?;
    Ok(())
}

/// Moves of the mini-player are saved (video.rs follows the video windows).
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != MINI_LABEL {
        return;
    }
    if let WindowEvent::Moved(moved) = event {
        let position = WindowPosition { x: moved.x, y: moved.y };
        if !is_minimized(position) {
            save_position(window.app_handle(), Slot::Mini, position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_uses_the_privacy_enhanced_domain() {
        assert_eq!(
            embed_url("dQw4w9WgXcQ", 42),
            "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?autoplay=1&rel=0&start=42"
        );
        assert_eq!(player_url("dQw4w9WgXcQ", 0), "https://ilot-player.localhost/embed/dQw4w9WgXcQ?start=0");
    }

    #[test]
    fn player_requests_are_validated() {
        assert_eq!(parse_player_request("/embed/dQw4w9WgXcQ", Some("start=90")), Some(("dQw4w9WgXcQ".into(), 90)));
        assert_eq!(parse_player_request("/embed/dQw4w9WgXcQ", None), Some(("dQw4w9WgXcQ".into(), 0)));
        assert_eq!(parse_player_request("/embed/dQw4w9WgXcQ", Some("start=999999")), Some(("dQw4w9WgXcQ".into(), 86_400)));
        assert_eq!(parse_player_request("/embed/dQw4w9WgXcQ", Some("start=-5")), Some(("dQw4w9WgXcQ".into(), 0)));
        assert_eq!(parse_player_request("/embed/\"><script>", None), None);
        assert_eq!(parse_player_request("/embed/dQw4w9WgXcQ/x", None), None);
        assert_eq!(parse_player_request("/index.html", None), None);
    }

    #[test]
    fn only_the_player_page_may_load_in_the_video_window() {
        assert!(is_player_url(&Url::parse("https://ilot-player.localhost/embed/dQw4w9WgXcQ").unwrap()));
        assert!(!is_player_url(&Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap()));
        assert!(!is_player_url(&Url::parse("https://ilot-player.localhost.evil.com/embed/x").unwrap()));
        assert!(!is_player_url(&Url::parse("http://tauri.localhost/index.html").unwrap()));
    }

    #[test]
    fn page_holds_only_the_embed() {
        let html = page_html("dQw4w9WgXcQ", 5);
        assert!(html.contains("src=\"https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?autoplay=1&amp;rel=0&amp;start=5\""));
        assert!(html.contains("referrerpolicy=\"strict-origin-when-cross-origin\""));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn protocol_serves_the_video_window_only() {
        let request = Request::builder().uri("ilot-player://localhost/embed/dQw4w9WgXcQ?start=3").body(Vec::new()).unwrap();
        let page = serve_player(VIDEO_LABEL, &request);
        assert_eq!(page.status(), StatusCode::OK);
        let csp = page.headers()[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
        assert!(csp.starts_with("default-src 'none'; frame-src https://www.youtube-nocookie.com;"));
        assert_eq!(serve_player("main", &request).status(), StatusCode::FORBIDDEN);
        assert_eq!(serve_player(MINI_LABEL, &request).status(), StatusCode::FORBIDDEN);
        let bad = Request::builder().uri("ilot-player://localhost/embed/nope").body(Vec::new()).unwrap();
        assert_eq!(serve_player(VIDEO_LABEL, &bad).status(), StatusCode::NOT_FOUND);
    }

    const SCREEN: Area = Area { x: 0, y: 0, width: 1920, height: 1040 };
    const LEFT: Area = Area { x: -1366, y: 0, width: 1366, height: 728 };

    #[test]
    fn saved_positions_on_a_screen_are_kept() {
        assert!(is_reachable(WindowPosition { x: 1500, y: 500 }, 340, &[SCREEN]));
        assert!(is_reachable(WindowPosition { x: -800, y: 100 }, 340, &[SCREEN, LEFT]));
        // Mostly off the right edge, but its top can still be grabbed.
        assert!(is_reachable(WindowPosition { x: 1850, y: 500 }, 340, &[SCREEN]));
    }

    #[test]
    fn positions_off_every_screen_are_dropped() {
        assert!(!is_reachable(WindowPosition { x: -800, y: 100 }, 340, &[SCREEN]));
        assert!(!is_reachable(WindowPosition { x: 1900, y: 500 }, 340, &[SCREEN]));
        assert!(!is_reachable(WindowPosition { x: 500, y: -200 }, 340, &[SCREEN]));
        assert!(!is_reachable(WindowPosition { x: 500, y: 1030 }, 340, &[SCREEN]));
        assert!(!is_reachable(WindowPosition { x: 0, y: 0 }, 340, &[]));
    }

    #[test]
    fn default_place_is_bottom_right_beside_the_other_player() {
        assert_eq!(bottom_right(SCREEN, (340, 500), 24, 0), WindowPosition { x: 1556, y: 516 });
        assert_eq!(bottom_right(SCREEN, (496, 309), 24, 364), WindowPosition { x: 1036, y: 707 });
        assert_eq!(bottom_right(LEFT, (2000, 2000), 24, 0), WindowPosition { x: -1366, y: 0 });
    }

    #[test]
    fn minimized_windows_are_not_saved() {
        assert!(is_minimized(WindowPosition { x: -32_000, y: -32_000 }));
        assert!(!is_minimized(WindowPosition { x: -32_000, y: 10 }));
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
        const PLAYER_PAGE: &str = "https://ilot-player.localhost/embed/dQw4w9WgXcQ?start=0";

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

        /// True when the capabilities let this window call the command; a refusal must
        /// come from the access control, not from anything else.
        fn allowed(window: &WebviewWindow<MockRuntime>, cmd: &str, url: &str) -> bool {
            let request = InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: url.parse().unwrap(),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            match get_ipc_response(window, request) {
                Ok(_) => true,
                Err(error) => {
                    assert!(error.to_string().contains("not allowed"), "{cmd}: {error}");
                    false
                }
            }
        }

        #[test]
        fn the_youtube_window_reaches_no_command() {
            let app = app();
            let video = window(&app, super::VIDEO_LABEL);
            for cmd in ["get_media", "get_settings", "save_settings", "plugin:event|listen"] {
                assert!(!allowed(&video, cmd, PLAYER_PAGE), "{cmd} from the player page");
                assert!(!allowed(&video, cmd, LOCAL), "{cmd} even from a local page");
            }
        }

        #[test]
        fn the_mini_player_reaches_media_but_not_settings_changes() {
            let app = app();
            let mini = window(&app, super::MINI_LABEL);
            assert!(allowed(&mini, "get_media", LOCAL));
            assert!(allowed(&mini, "get_settings", LOCAL));
            assert!(!allowed(&mini, "save_settings", LOCAL));
            assert!(!allowed(&mini, "get_media", PLAYER_PAGE), "remote pages get nothing");
        }
    }

    #[test]
    fn positions_go_to_their_slot() {
        let mut settings = Settings::default();
        with_position(&mut settings, Slot::Mini, WindowPosition { x: 1, y: 2 });
        with_position(&mut settings, Slot::Video, WindowPosition { x: 3, y: 4 });
        assert_eq!(settings.mini_player, Some(WindowPosition { x: 1, y: 2 }));
        assert_eq!(settings.video_player, Some(WindowPosition { x: 3, y: 4 }));
    }
}
