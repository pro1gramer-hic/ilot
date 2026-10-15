//! Icon in the notification area: Réglages, Ne pas déranger, Quitter, in the language of
//! the moment (the menu is built again when it changes).

use crate::locale::{self, Lang, Text, tr};
use crate::settings::{Settings, SettingsStore};
use crate::window::WINDOW_LABEL;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};

pub const EVENT_OPEN_SETTINGS: &str = "island:open-settings";
const TRAY_ID: &str = "ilot";

/// The menu shown, and the language it speaks.
struct Tray {
    menu: Mutex<(CheckMenuItem<Wry>, Lang)>,
}

/// The menu in the language of the moment; its check box follows the settings.
fn menu(app: &AppHandle, settings: &Settings) -> tauri::Result<(Menu<Wry>, CheckMenuItem<Wry>)> {
    let open = MenuItem::with_id(app, "settings", tr(Text::TraySettings), true, None::<&str>)?;
    let do_not_disturb =
        CheckMenuItem::with_id(app, "dnd", tr(Text::TrayDoNotDisturb), true, settings.do_not_disturb, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tr(Text::TrayQuit), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &do_not_disturb, &separator, &quit])?;
    Ok((menu, do_not_disturb))
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<SettingsStore>().get();
    let (menu, do_not_disturb) = menu(app, &settings)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("îlot")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                let _ = app.emit_to(WINDOW_LABEL, EVENT_OPEN_SETTINGS, ());
            }
            "dnd" => {
                let mut settings = app.state::<SettingsStore>().get();
                settings.do_not_disturb = !settings.do_not_disturb;
                let _ = crate::apply_settings(app, settings);
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(Tray { menu: Mutex::new((do_not_disturb, locale::current())) });
    Ok(())
}

/// Keeps the menu in line with the settings: its check mark, and its language (a new menu
/// when the language changed).
pub fn sync(app: &AppHandle, settings: &Settings) {
    let Some(tray) = app.try_state::<Tray>() else { return };
    let mut shown = tray.menu.lock().unwrap();
    let lang = locale::current();
    if shown.1 != lang
        && let Some(icon) = app.tray_by_id(TRAY_ID)
        && let Ok((menu, do_not_disturb)) = menu(app, settings)
        && icon.set_menu(Some(menu)).is_ok()
    {
        *shown = (do_not_disturb, lang);
        return;
    }
    let _ = shown.0.set_checked(settings.do_not_disturb);
}
