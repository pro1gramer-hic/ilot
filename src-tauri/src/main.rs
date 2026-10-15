// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_volume;
mod artwork;
mod atomic;
mod attachments;
mod audio;
mod autostart;
mod buddy;
mod chat;
mod clipboard;
mod connectors;
mod core;
mod decisions;
mod dictation;
mod events;
mod export;
mod locale;
mod mail;
mod media;
mod memory;
mod native_host;
mod notifications;
mod on_top;
mod pastilles;
mod permissions;
mod pipe_server;
mod placement;
mod players;
mod risk;
mod screens;
mod sessions;
mod settings;
mod sounds;
mod shortcuts;
mod source_app;
mod stats;
mod terminal;
mod translate;
mod tray;
mod uninstall;
mod vault;
mod video;
mod voice;
mod volume;
mod window;
mod window_context;
mod wsl;

use connectors::{Answer, Caps, Status, Target};
use core::Core;
use locale::{Text, tr};
use serde::Serialize;
use settings::{Settings, SettingsStore};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use window::{IslandShape, IslandWindow, WINDOW_LABEL};

const EVENT_TOGGLE: &str = "island:toggle";
const EVENT_SETTINGS: &str = "settings:changed";
/// A shortcut of a named action was pressed (payload: the action id).
const EVENT_SHORTCUT: &str = "shortcut:action";

/// Clickable outlines of the island window (the island first, then the open panel and
/// the music bubble): clicks pass through everywhere else.
#[tauri::command]
fn set_island_shape(shapes: Vec<IslandShape>, island: State<'_, IslandWindow>) {
    island.set_shapes(shapes);
}

#[tauri::command]
fn focus_island(typing: Option<bool>, window: WebviewWindow, island: State<'_, IslandWindow>) -> Result<(), String> {
    island.take_focus(&window, typing.unwrap_or(false)).map_err(|e| e.to_string())
}

#[tauri::command]
fn release_focus(window: WebviewWindow, island: State<'_, IslandWindow>) {
    island.give_back_focus(&window);
}

#[tauri::command]
fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

#[tauri::command]
fn save_settings(settings: Settings, app: AppHandle) -> Result<Settings, String> {
    let current = app.state::<SettingsStore>().get();
    apply_settings(&app, settings.with_positions_of(&current).with_shortcuts_of(&current))
}

/// Gives `action` a new shortcut (empty: none), registered with Windows before it is
/// saved: one Windows refuses leaves the previous one in place.
#[tauri::command]
fn set_shortcut(action: String, shortcut: String, app: AppHandle) -> Result<Settings, String> {
    let current = app.state::<SettingsStore>().get();
    apply_settings(&app, shortcuts::assign(&current, &action, &shortcut)?)
}

/// The Monday report of last week, or of this week so far (none: nothing counted yet).
#[tauri::command]
fn week_report(which: stats::Which, core: State<'_, AppCore>) -> Option<stats::Report> {
    core.stats().report(which, events::now_ms())
}

/// True once per Monday when last week's report is ready to be offered.
#[tauri::command]
fn take_monday_report(core: State<'_, AppCore>) -> bool {
    core.stats().take_monday(events::now_ms())
}

/// « Effacer mes statistiques »: the counters file goes.
#[tauri::command]
fn clear_stats(core: State<'_, AppCore>) -> Result<(), String> {
    core.stats().clear().map_err(|e| tr(Text::StatsClearFailed { error: &e.to_string() }))
}

type AppCore = Arc<Core<AppHandle>>;

#[tauri::command]
fn answer_permission(id: String, answer: Answer, core: State<'_, AppCore>) -> bool {
    core.answer(&id, answer)
}

/// The user's answers to an agent's questions (checked by the core against them).
#[tauri::command]
fn answer_questions(id: String, answers: Vec<connectors::questions::QuestionAnswer>, core: State<'_, AppCore>) -> bool {
    core.answer_questions(&id, &answers)
}

/// The latest change of a session, for the hub's live view (memory only).
#[tauri::command]
fn session_diff(id: String, core: State<'_, AppCore>) -> Option<connectors::diff::FileDiff> {
    core.session_diff(&id)
}

/// « Aller au terminal »: the window of a session (or of the session that matters most),
/// brought forward off the interface thread.
#[tauri::command]
async fn go_to_terminal(session_id: Option<String>, core: State<'_, AppCore>) -> Result<terminal::Outcome, String> {
    let Some((host, project)) = core.terminal_of(session_id.as_deref()) else {
        return Ok(terminal::Outcome { found: false, exact: false, message: tr(Text::NoSessionToJoin) });
    };
    tauri::async_runtime::spawn_blocking(move || terminal::go(host, &project)).await.map_err(|e| e.to_string())
}

/// « Tout suspendre »: refuses the waiting requests and every new one until turned off.
#[tauri::command]
fn suspend_agents(on: bool, core: State<'_, AppCore>) {
    core.suspend(on);
}

/// The decisions journal of this run (memory only), newest first.
#[tauri::command]
fn decisions(core: State<'_, AppCore>) -> Vec<decisions::Decision> {
    core.decisions()
}

/// System volume, read when the wheel gauge appears.
#[tauri::command]
fn get_volume() -> Result<f32, String> {
    volume::level()
}

/// Wheel on the pill: moves the system volume by whole notches.
#[tauri::command]
fn change_volume(notches: i32) -> Result<f32, String> {
    volume::change(notches)
}

/// Current sessions, waiting permissions and « Tout suspendre », for an interface that
/// just loaded.
#[tauri::command]
fn agents_snapshot(core: State<'_, AppCore>) -> serde_json::Value {
    serde_json::json!({ "sessions": core.sessions(), "permissions": core.permissions(), "suspended": core.suspended() })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetView {
    id: String,
    label: String,
    status: Status,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectorView {
    id: &'static str,
    name: String,
    caps: Caps,
    targets: Vec<TargetView>,
}

fn connector_views() -> Vec<ConnectorView> {
    let targets = connectors::all_targets();
    connectors::registry()
        .iter()
        .map(|c| ConnectorView {
            id: c.id(),
            name: c.display_name(),
            caps: c.capabilities(),
            targets: targets
                .iter()
                .map(|t| TargetView { id: t.id.clone(), label: t.label.clone(), status: c.status(t) })
                .collect(),
        })
        .collect()
}

/// Lists the AIs and where they are installed (starts `wsl.exe`, so off the main thread).
#[tauri::command]
async fn list_connectors() -> Result<Vec<ConnectorView>, String> {
    tauri::async_runtime::spawn_blocking(connector_views).await.map_err(|e| e.to_string())
}

/// The relay sits next to îlot's executable.
fn relay_exe() -> Result<PathBuf, String> {
    let path = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("ilot-hook.exe");
    if path.exists() { Ok(path) } else { Err(tr(Text::RelayMissing { path: &path.display().to_string() })) }
}

fn find_target(target_id: &str) -> Result<Target, String> {
    connectors::all_targets()
        .into_iter()
        .find(|t| t.id == target_id)
        .ok_or_else(|| tr(Text::InstallationMissing { id: target_id }))
}

/// Plugs or unplugs an AI, on the user's click only.
#[tauri::command]
async fn set_connected(connector_id: String, target_id: String, connected: bool) -> Result<Vec<ConnectorView>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let connector = connectors::find(&connector_id).ok_or(Text::ConnectorUnknown)?;
        let target = find_target(&target_id)?;
        let result = if connected { connector.connect(&target, &relay_exe()?) } else { connector.disconnect(&target) };
        result.map_err(|e| tr(Text::ConnectFailed { error: &e.to_string() }))?;
        Ok(connector_views())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Starts the pipe server with a fresh token that relays read from `pipe.token`, written
/// by the server once the pipe is îlot's.
fn start_bridge(core: AppCore) -> Result<(), String> {
    use ilot_bridge::identity::{current_user_sid, pipe_name, token_path};
    let sid = current_user_sid().map_err(|e| e.to_string())?;
    let config = pipe_server::ServerConfig {
        pipe_name: pipe_name(&sid),
        token: core::random_hex(32),
        token_file: token_path().ok_or("LOCALAPPDATA introuvable")?,
        sid,
        permission_timeout: core::PERMISSION_TIMEOUT,
    };
    let handler: Arc<dyn pipe_server::Handler> = core.clone();
    // Built inside the runtime: tokio pipes need its reactor.
    let server = tauri::async_runtime::block_on(async { pipe_server::start(config, handler) }).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn(server);
    tauri::async_runtime::spawn(async move {
        let mut every_minute = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            every_minute.tick().await;
            core.tick();
        }
    });
    Ok(())
}

/// Saves new settings and brings the shortcuts, the core's language, the tray menu and
/// the interface in line.
pub fn apply_settings(app: &AppHandle, settings: Settings) -> Result<Settings, String> {
    let settings = settings.sanitized();
    let store = app.state::<SettingsStore>();
    let previous = store.get();
    if previous.launch_at_startup != settings.launch_at_startup {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        autostart::set(settings.launch_at_startup, &exe).map_err(|e| tr(Text::AutostartFailed { error: &e.to_string() }))?;
    }
    shortcuts::sync(app.global_shortcut(), &app.state::<shortcuts::Bindings>(), &previous, &settings)?;
    locale::set(&settings.language);
    let settings = Settings { notifications: settings.notifications.localized(), ..settings };
    store.replace(settings.clone()).map_err(|e| e.to_string())?;
    voice::settings_changed(app, &settings.voice);
    tray::sync(app, &settings);
    notifications::settings_changed(app);
    pastilles::settings_changed(app, &settings.pastilles);
    for label in [WINDOW_LABEL, players::MINI_LABEL, buddy::BUDDY_LABEL, video::YOUTUBE_BAR, video::CRUNCHYROLL_BAR] {
        let _ = app.emit_to(label, EVENT_SETTINGS, &settings);
    }
    Ok(settings)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Edge or Chrome starts îlot as the native messaging host of its extension: relay and quit.
    if native_host::is_host_launch(&args) {
        std::process::exit(native_host::run_host(&args[1]));
    }
    if args.iter().any(|a| a == "--uninstall-cleanup") {
        std::process::exit(uninstall::run(args.iter().any(|a| a == "--remove-data")));
    }
    #[cfg(debug_assertions)]
    if std::env::args().any(|arg| arg == "--notifications-probe") {
        std::process::exit(notifications::probe());
    }
    // The command-line modes above run even while îlot runs; from here on, one îlot only.
    tauri::Builder::default()
        // First plugin, as its documentation asks (https://v2.tauri.app/plugin/single-instance/).
        // A second îlot (start with Windows plus a double click) opens this one's panel and
        // quits before it reaches the pipe or its token.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.emit_to(WINDOW_LABEL, tray::EVENT_OPEN_SETTINGS, ());
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let action = app.try_state::<shortcuts::Bindings>().and_then(|b| b.action_of(shortcut.id()));
                    let _ = match action.as_deref() {
                        Some(shortcuts::ISLAND) => app.emit_to(WINDOW_LABEL, EVENT_TOGGLE, ()),
                        Some(action) => app.emit_to(WINDOW_LABEL, EVENT_SHORTCUT, action),
                        None => Ok(()),
                    };
                })
                .build(),
        )
        .register_uri_scheme_protocol(players::PLAYER_SCHEME, |ctx, request| {
            players::serve_player(ctx.webview_label(), &request)
        })
        .setup(|app| {
            // The language first: the settings read next speak it (the sites' rule).
            let path = settings::settings_path();
            locale::set(&path.as_deref().map(settings::load).unwrap_or_default().language);
            let store = SettingsStore::open(path);
            let saved = store.get();
            app.manage(store);
            app.manage(shortcuts::Bindings::default());
            app.manage(voice::Voice::default());
            app.manage(IslandWindow::default());
            app.manage(buddy::Buddy::default());
            app.manage(media::MediaHub::default());
            app.manage(video::VideoHub::default());
            app.manage(native_host::Extension::default());
            app.manage(audio::AudioMeter::default());
            app.manage(attachments::Attachments::default());
            app.manage(chat::Chat::default());
            app.manage(window_context::Contexts::default());
            app.manage(dictation::Dictation::default());
            app.manage(pastilles::PastilleHub::default());
            // Another app may already own a shortcut: îlot still runs, hover and tray work,
            // and the other shortcuts too.
            let failed = shortcuts::start(app.global_shortcut(), &app.state::<shortcuts::Bindings>(), &saved);
            if !failed.is_empty() {
                eprintln!("îlot: shortcuts not registered: {failed:?}");
            }
            let counters = stats::Stats::open(stats::stats_path(), stats::local_day);
            let core: AppCore = Arc::new(Core::with_stats(app.handle().clone(), counters));
            app.manage(core.clone());
            // Without the bridge îlot still runs; agents then simply ask in their terminal.
            if let Err(error) = start_bridge(core) {
                eprintln!("îlot: bridge not started: {error}");
            }
            tray::create(app.handle())?;
            window::start(app.handle())?;
            players::setup(app.handle())?;
            video::setup(app.handle());
            // Without it, Crunchyroll in a browser is simply kept on top instead of floating.
            if let Err(error) = native_host::start(app.handle()) {
                eprintln!("îlot: browser extension link not started: {error}");
            }
            media::start(app.handle())?;
            notifications::start(app.handle())?;
            pastilles::start(app.handle());
            Ok(())
        })
        // A loaded (or reloaded) island hears again where its pill sits.
        .on_page_load(|webview, payload| {
            if webview.label() == WINDOW_LABEL && matches!(payload.event(), PageLoadEvent::Finished) {
                webview.state::<IslandWindow>().announce_soon();
            }
        })
        .on_window_event(|window, event| {
            if window.label() != WINDOW_LABEL {
                players::on_window_event(window, event);
                video::on_window_event(window, event);
                buddy::on_window_event(window, event);
                return;
            }
            match event {
                // A drop on the island: its paths are remembered, nothing is opened.
                WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                    window.state::<attachments::Attachments>().remember(paths);
                }
                // Alt+F4 must not leave îlot running without its island: quitting goes through the tray.
                WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
                // Once the user goes elsewhere, clicks on the island stop taking the keyboard.
                WindowEvent::Focused(false) => {
                    let _ = window.set_focusable(false);
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            set_island_shape,
            window::island_drag_start,
            window::island_move_by,
            window::island_recenter,
            window::island_placement,
            window::list_screens,
            window::island_to_screen,
            buddy::buddy_show,
            buddy::buddy_move,
            buddy::buddy_spots,
            buddy::buddy_recall,
            buddy::buddy_home,
            buddy::buddy_hide,
            buddy::buddy_grab,
            buddy::buddy_settle,
            buddy::buddy_restore,
            buddy::open_wardrobe,
            sounds::play_sound,
            sounds::sound_overrides,
            sounds::sound_file,
            sounds::open_sounds_folder,
            focus_island,
            release_focus,
            get_settings,
            save_settings,
            set_shortcut,
            week_report,
            take_monday_report,
            clear_stats,
            export::save_report_image,
            voice::voice_listen,
            voice::voice_say,
            voice::voice_hush,
            voice::voice_ask,
            answer_permission,
            answer_questions,
            session_diff,
            go_to_terminal,
            suspend_agents,
            decisions,
            agents_snapshot,
            get_volume,
            change_volume,
            list_connectors,
            set_connected,
            media::get_media,
            media::media_command,
            media::media_seek,
            media::media_mode,
            media::open_media_source,
            app_volume::app_volume,
            app_volume::set_app_volume,
            audio::audio_meter,
            clipboard::copy_text,
            clipboard::read_clipboard_text,
            players::open_mini_player,
            players::close_mini_player,
            video::play_youtube,
            video::open_crunchyroll,
            video::detach_video,
            video::dock_video,
            video::video_state,
            video::video_bar_move,
            video::close_video,
            notifications::notifications_status,
            notifications::request_notification_access,
            notifications::open_notification_app,
            notifications::open_notification_settings,
            vault::secret_store,
            vault::secret_has,
            vault::secret_remove,
            chat::chat_send,
            chat::chat_cancel,
            chat::chat_models,
            attachments::file_attach,
            mail::mail_app,
            mail::mail_send,
            window_context::buddy_drop_window,
            window_context::context_read,
            window_context::context_confirm,
            window_context::context_forget,
            dictation::dictation_available,
            dictation::dictation_start,
            dictation::dictation_stop,
            pastilles::pastilles_snapshot,
            pastilles::pastilles_catalog,
            pastilles::pastille_refresh,
            pastilles::pastille_show_tool,
            pastilles::ollama_pull,
            pastilles::ollama_pull_cancel,
            translate::translate_text,
            memory::island_memory
        ])
        .run(tauri::generate_context!())
        .expect("error while running îlot");
}
