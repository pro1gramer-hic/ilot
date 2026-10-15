//! Messaging notifications (docs/05-NOTIFICATIONS.md). The Windows listener reads the
//! toasts of the apps the user checked and hands them to the interface. Everything stays
//! in memory: no notification text is ever written to disk or to a log.
//!
//! This file holds the pure part (settings, matching rules, parsing), tested with
//! realistic anonymized toasts; `listener.rs` talks to Windows, `foreground.rs` brings
//! the sender's window to the front, `web_tabs.rs` and `uia.rs` recognize WhatsApp Web in
//! a browser tab, `duplicates.rs` shows once a message the app and the tab both received.
//!
//! What Windows gives a listener: the app's display name and the text lines of the toast.
//! The site line that browsers add (`placement="attribution"`, "web.whatsapp.com") is
//! not part of it (checked on Windows 11 with a test toast): a plain browser tab cannot
//! be told apart from another one, but a site installed as an app ("WhatsApp Web") shows
//! its own name. For WhatsApp, îlot then looks at the browser's tabs (`web_tabs.rs`).

mod duplicates;
/// Also lists the windows « Aller au terminal » chooses from (`terminal.rs`).
pub(crate) mod foreground;
mod listener;
mod uia;
mod web_tabs;

/// Shared with the window Mochi is dropped on (window_context.rs): one UI Automation
/// client at a time, and the program behind a window.
pub use foreground::program_path;
pub use uia::turn as uia_turn;

use crate::locale::{Lang, Text, text_in, tr};
use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use tauri::{AppHandle, Manager};

pub const EVENT_INCOMING: &str = "notifications:incoming";
pub const EVENT_STATUS: &str = "notifications:status";

const MAX_SENDER_CHARS: usize = 80;
const MAX_TEXT_CHARS: usize = 400;
/// « Ouvrir » targets kept for the notifications still on screen or in « Plus tard ».
const MAX_TARGETS: usize = 64;

/// The user's choices, saved with the other settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationSettings {
    /// Off by default: îlot reads no notification until the user turns this on.
    pub enabled: bool,
    /// Ids of the rules (apps) the user checked. Empty by default.
    pub allowed: Vec<String>,
    /// Shows « Nouveau message de X » without the message itself.
    pub hide_content: bool,
    pub rules: Vec<AppRule>,
    /// A message from Firefox, Chrome or Edge counts for WhatsApp when the browser shows a
    /// WhatsApp Web tab (read with the accessibility of Windows). On by default; it only
    /// applies once WhatsApp is checked.
    pub whatsapp_tabs: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self { enabled: false, allowed: Vec::new(), hide_content: false, rules: default_rules(), whatsapp_tabs: true }
    }
}

impl NotificationSettings {
    /// Listening is useless (and reads private data for nothing) until an app is checked.
    pub fn wanted(&self) -> bool {
        self.enabled && !self.allowed.is_empty()
    }

    fn allows(&self, rule: &str) -> bool {
        self.allowed.iter().any(|id| id == rule)
    }

    /// The rule of the sites (« Sites web ») in the language of the moment: îlot's own name
    /// for them, saved with the settings in the language of the day they were saved.
    pub fn localized(mut self) -> Self {
        let names: Vec<String> = Lang::ALL.iter().map(|lang| text_in(*lang, Text::WebsitesApp)).collect();
        for rule in self.rules.iter_mut().filter(|rule| rule.id == WEB && names.contains(&rule.label)) {
            rule.label = tr(Text::WebsitesApp);
        }
        self
    }
}

/// Id of the WhatsApp rule, the one a WhatsApp Web tab is credited to.
const WHATSAPP: &str = "whatsapp";
/// Id of the rule of every site open in a browser tab.
const WEB: &str = "web";

/// How to recognize one app: `{ appName, contient?, libellé, icône }` of
/// docs/05-NOTIFICATIONS.md, with lists so that one checkbox covers every variant.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppRule {
    pub id: String,
    /// Name shown in the island and in the settings.
    pub label: String,
    /// One or two letters shown when Windows gives no logo.
    pub icon: String,
    /// Names Windows gives the app (`AppInfo.DisplayInfo.DisplayName`). "WhatsApp" also
    /// covers "WhatsApp Web" (the site installed as an app) and "WhatsApp Beta".
    pub app_names: Vec<String>,
    /// When not empty, the toast must contain one of these texts (any case).
    pub contains: Vec<String>,
    /// Program files whose window « Ouvrir » brings to the front.
    pub programs: Vec<String>,
}

fn rule(id: &str, label: &str, icon: &str, app_names: &[&str], programs: &[&str]) -> AppRule {
    let owned = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect();
    AppRule {
        id: id.to_owned(),
        label: label.to_owned(),
        icon: icon.to_owned(),
        app_names: owned(app_names),
        contains: Vec::new(),
        programs: owned(programs),
    }
}

/// Browsers whose web notifications go through Windows, with their program file.
const BROWSERS: [(&str, &str); 8] = [
    ("Google Chrome", "chrome.exe"),
    ("Chrome", "chrome.exe"),
    ("Microsoft Edge", "msedge.exe"),
    ("Mozilla Firefox", "firefox.exe"),
    ("Firefox", "firefox.exe"),
    ("Brave", "brave.exe"),
    ("Opera", "opera.exe"),
    ("Vivaldi", "vivaldi.exe"),
];

/// Rules for the apps of docs/01-VISION.md, then every site open in a browser tab.
pub fn default_rules() -> Vec<AppRule> {
    let browsers: Vec<&str> = BROWSERS.iter().map(|(name, _)| *name).collect();
    vec![
        rule("whatsapp", "WhatsApp", "Wa", &["WhatsApp"], &["WhatsApp.exe", "WhatsApp.Root.exe"]),
        rule("discord", "Discord", "Di", &["Discord"], &["Discord.exe", "DiscordPTB.exe", "DiscordCanary.exe"]),
        rule("snapchat", "Snapchat", "Sn", &["Snapchat"], &[]),
        rule("telegram", "Telegram", "Tg", &["Telegram"], &["Telegram.exe"]),
        rule("messenger", "Messenger", "Me", &["Messenger"], &["Messenger.exe"]),
        rule("instagram", "Instagram", "Ig", &["Instagram"], &[]),
        rule("teams", "Teams", "Tm", &["Microsoft Teams", "Teams"], &["ms-teams.exe", "Teams.exe"]),
        rule("slack", "Slack", "Sl", &["Slack"], &["slack.exe"]),
        rule("gmail", "Gmail", "Gm", &["Gmail"], &[]),
        rule(WEB, &tr(Text::WebsitesApp), "Wb", &browsers, &[]),
    ]
}

/// A toast as Windows gives it: app name and the text lines of its visual binding.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u32,
    pub app_name: String,
    pub texts: Vec<String>,
}

/// A notification ready for the island: filtered, cleaned and masked when asked.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Incoming {
    pub id: String,
    /// Rule id ("whatsapp").
    pub app: String,
    pub app_label: String,
    pub monogram: String,
    /// The app's logo as a `data:` URL, when Windows has one (never for a browser).
    pub icon: Option<String>,
    pub sender: String,
    pub text: String,
    pub masked: bool,
    /// Epoch milliseconds.
    pub received_at: u64,
}

/// Where « Ouvrir » goes: program files from the rules, never data from the toast.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenTarget {
    /// Any window of these programs will do.
    pub programs: Vec<String>,
    /// Windows of these programs only when their title names the app (a site installed
    /// as an app runs in a browser window titled "WhatsApp").
    pub titled: Vec<String>,
    /// Rule label, looked for in window titles.
    pub label: String,
    /// WhatsApp Web in a tab of the browser in `programs`: « Ouvrir » selects that tab.
    pub whatsapp_tab: bool,
}

/// A toast that passed the rules and the user's list.
#[derive(Debug, PartialEq)]
pub struct Accepted {
    pub incoming: Incoming,
    pub target: OpenTarget,
    /// Sent by the app itself (its logo is meaningful), not by a browser tab.
    pub from_app: bool,
    /// Fingerprint of the message, to show it once when the app and the tab both send it.
    pub print: u64,
}

/// "Discord" matches "Discord" and "Discord PTB", never "Discordia".
fn name_matches(display_name: &str, name: &str) -> bool {
    let display = display_name.trim().to_lowercase();
    let name = name.trim().to_lowercase();
    !name.is_empty() && (display == name || display.starts_with(&format!("{name} ")))
}

fn browser_program(app_name: &str) -> Option<&'static str> {
    BROWSERS.iter().find(|(name, _)| name_matches(app_name, name)).map(|(_, program)| *program)
}

fn rule_matches(rule: &AppRule, toast: &Toast) -> bool {
    rule.app_names.iter().any(|name| name_matches(&toast.app_name, name))
        && (rule.contains.is_empty()
            || rule.contains.iter().any(|needle| {
                let needle = needle.trim().to_lowercase();
                !needle.is_empty() && toast.texts.iter().any(|text| text.to_lowercase().contains(&needle))
            }))
}

/// Invisible characters that can reorder or hide text (bidi controls, zero-width spaces).
/// The zero-width joiner stays: emoji sequences need it.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}' | '\u{FEFF}' | '\u{061C}'
    )
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    cut
}

/// Plain, visible text: no control or invisible character, no blank line, bounded length.
fn clean(text: &str, max_chars: usize) -> String {
    let visible: String = text
        .chars()
        .filter_map(|c| match c {
            '\n' => Some('\n'),
            '\t' => Some(' '),
            c if c.is_control() || is_invisible(c) => None,
            c => Some(c),
        })
        .collect();
    let lines: Vec<&str> = visible.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    truncate(&lines.join("\n"), max_chars)
}

/// The browser whose tabs are worth reading for this toast: a message from Firefox, Chrome
/// or Edge may come from WhatsApp Web, once WhatsApp is checked and the recognition left
/// on. `None` costs nothing: no tab is read.
pub fn tab_lookup(toast: &Toast, settings: &NotificationSettings) -> Option<&'static str> {
    let program = browser_program(&toast.app_name)?;
    let wanted = settings.whatsapp_tabs && settings.allows(WHATSAPP) && settings.rules.iter().any(|rule| rule.id == WHATSAPP);
    (wanted && web_tabs::engine(program).is_some()).then_some(program)
}

/// Applies the rules and the user's choices to a toast. `whatsapp_tab` says that the
/// browser that sent it shows WhatsApp Web (`tab_lookup`, then `uia.rs`): the toast then
/// counts for WhatsApp. `None` means îlot ignores it.
pub fn accept(toast: &Toast, settings: &NotificationSettings, received_at: u64, whatsapp_tab: bool) -> Option<Accepted> {
    let browser = browser_program(&toast.app_name);
    let in_tab = whatsapp_tab && settings.whatsapp_tabs && browser.is_some();
    let tab_rule = settings.rules.iter().find(|rule| in_tab && rule.id == WHATSAPP);
    let rule = tab_rule.or_else(|| settings.rules.iter().find(|rule| rule_matches(rule, toast)))?;
    if !settings.allows(&rule.id) {
        return None;
    }
    let lines: Vec<String> = toast.texts.iter().map(|t| clean(t, MAX_TEXT_CHARS)).filter(|t| !t.is_empty()).collect();
    let sender = lines.first().map_or_else(|| rule.label.clone(), |s| truncate(s, MAX_SENDER_CHARS));
    let text = if settings.hide_content || lines.len() < 2 {
        String::new()
    } else {
        truncate(&lines[1..].join("\n"), MAX_TEXT_CHARS)
    };
    let target = match browser {
        Some(program) => OpenTarget {
            programs: vec![program.to_owned()],
            titled: Vec::new(),
            label: String::new(),
            whatsapp_tab: tab_rule.is_some(),
        },
        None => OpenTarget {
            programs: rule.programs.clone(),
            titled: BROWSERS.iter().map(|(_, program)| (*program).to_owned()).collect(),
            label: rule.label.clone(),
            whatsapp_tab: false,
        },
    };
    let print = duplicates::fingerprint(&rule.id, &lines);
    Some(Accepted {
        incoming: Incoming {
            id: toast.id.to_string(),
            app: rule.id.clone(),
            app_label: rule.label.clone(),
            monogram: rule.icon.clone(),
            icon: None,
            sender,
            text,
            masked: settings.hide_content,
            received_at,
        },
        target,
        from_app: browser.is_none(),
        print,
    })
}

/// Ids present now and not before: the notifications that just arrived.
pub fn new_ids(known: &HashSet<u32>, current: &[u32]) -> Vec<u32> {
    current.iter().copied().filter(|id| !known.contains(id)).collect()
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(BASE64[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A logo as a `data:` URL (allowed by the CSP), only for PNG and JPEG images.
fn image_data_url(bytes: &[u8]) -> Option<String> {
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else {
        return None;
    };
    Some(format!("data:{mime};base64,{}", base64(bytes)))
}

/// Notification access as Windows reports it (`UserNotificationListenerAccessStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Access {
    /// The listener could not be reached (unexpected Windows error).
    Unavailable,
    NotAsked,
    Allowed,
    Denied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub access: Access,
    /// ilot.exe runs with its package identity (packaging/identity).
    pub identity: bool,
    /// The listener is running (enabled, an app checked, access allowed).
    pub listening: bool,
    /// Windows refused its change event (it needs the identity): îlot asks every second.
    pub polling: bool,
}

impl Status {
    const fn idle(access: Access, identity: bool) -> Self {
        Self { access, identity, listening: false, polling: false }
    }
}

/// Wakes the listener thread.
pub enum Wake {
    /// Settings or access changed: start, stop or keep listening.
    Settings,
    /// Windows reported a notification added or removed.
    Changed,
}

/// Shared state of the notification service.
pub struct Notifications {
    wake: Mutex<Sender<Wake>>,
    status: Mutex<Status>,
    targets: Mutex<VecDeque<(String, OpenTarget)>>,
}

impl Notifications {
    fn new(wake: Sender<Wake>) -> Self {
        Self {
            wake: Mutex::new(wake),
            status: Mutex::new(Status::idle(Access::Unavailable, false)),
            targets: Mutex::new(VecDeque::new()),
        }
    }

    fn wake(&self, reason: Wake) {
        let _ = self.wake.lock().unwrap().send(reason);
    }

    /// Stores the new status; true when it changed.
    fn set_status(&self, status: Status) -> bool {
        let mut current = self.status.lock().unwrap();
        let changed = *current != status;
        *current = status;
        changed
    }

    fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    fn remember(&self, id: String, target: OpenTarget) {
        let mut targets = self.targets.lock().unwrap();
        targets.push_back((id, target));
        while targets.len() > MAX_TARGETS {
            targets.pop_front();
        }
    }

    fn target(&self, id: &str) -> Option<OpenTarget> {
        self.targets.lock().unwrap().iter().rev().find(|(known, _)| known == id).map(|(_, t)| t.clone())
    }
}

/// Starts the listener thread. It reads nothing until the user turns the feature on.
pub fn start(app: &AppHandle) -> std::io::Result<()> {
    let (wake, receiver) = mpsc::channel();
    app.manage(Notifications::new(wake.clone()));
    listener::spawn(app.clone(), wake, receiver)
}

/// Settings changed: the listener starts or stops accordingly.
pub fn settings_changed(app: &AppHandle) {
    if let Some(state) = app.try_state::<Notifications>() {
        state.wake(Wake::Settings);
    }
}

/// Fresh access and identity, with what the listener thread last reported.
fn current_status(app: &AppHandle) -> Status {
    let running = app.try_state::<Notifications>().map(|state| state.status());
    Status {
        access: listener::access(),
        identity: listener::package_full_name().is_some(),
        listening: running.is_some_and(|s| s.listening),
        polling: running.is_some_and(|s| s.polling),
    }
}

#[tauri::command]
pub fn notifications_status(app: AppHandle) -> Status {
    current_status(&app)
}

/// Shows Windows' consent prompt (on the UI thread, as Windows requires), then lets the
/// listener start if the user said yes.
#[tauri::command]
pub async fn request_notification_access(app: AppHandle) -> Result<Status, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || listener::request_access(&handle))
        .await
        .map_err(|e| e.to_string())?;
    settings_changed(&app);
    Ok(current_status(&app))
}

/// « Ouvrir »: brings the app (or the browser, on the WhatsApp Web tab) that sent this
/// notification to the front. Off the interface thread, as UI Automation requires.
#[tauri::command]
pub async fn open_notification_app(id: String, app: AppHandle) -> Result<(), String> {
    let target = app.state::<Notifications>().target(&id).ok_or(Text::MessageGone)?;
    let shown = target.clone();
    let brought = tauri::async_runtime::spawn_blocking(move || foreground::bring_to_front(&shown))
        .await
        .map_err(|e| e.to_string())?;
    if brought {
        Ok(())
    } else if target.label.is_empty() {
        Err(Text::BrowserNotOpen.into())
    } else {
        Err(tr(Text::AppNotOpen { app: &target.label }))
    }
}

/// Opens Windows Settings › Confidentialité › Notifications (fixed address).
#[tauri::command]
pub fn open_notification_settings() -> Result<(), String> {
    foreground::open_privacy_settings().then_some(()).ok_or_else(|| tr(Text::WindowsSettingsMissing))
}

/// Debug builds only: `ilot.exe --notifications-probe` prints, in the console only, the
/// toasts Windows currently keeps, to check by hand what îlot can read.
#[cfg(debug_assertions)]
pub fn probe() -> i32 {
    listener::probe()
}

fn settings_of(app: &AppHandle) -> NotificationSettings {
    app.state::<SettingsStore>().get().notifications
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toast(app_name: &str, texts: &[&str]) -> Toast {
        Toast { id: 42, app_name: app_name.to_owned(), texts: texts.iter().map(|t| (*t).to_owned()).collect() }
    }

    fn allowing(ids: &[&str]) -> NotificationSettings {
        NotificationSettings {
            enabled: true,
            allowed: ids.iter().map(|id| (*id).to_owned()).collect(),
            ..NotificationSettings::default()
        }
    }

    fn read(toast: &Toast, settings: &NotificationSettings) -> Option<Incoming> {
        accept(toast, settings, 1_000, false).map(|a| a.incoming)
    }

    #[test]
    fn defaults_read_nothing() {
        let settings = NotificationSettings::default();
        assert!(!settings.enabled);
        assert!(settings.allowed.is_empty());
        assert!(!settings.wanted());
        let whatsapp = toast("WhatsApp", &["Maman", "Tu rentres à quelle heure ?"]);
        assert_eq!(read(&whatsapp, &settings), None);
    }

    #[test]
    fn default_rules_cover_the_apps_of_the_vision_and_the_browsers() {
        let rules = default_rules();
        let ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["whatsapp", "discord", "snapchat", "telegram", "messenger", "instagram", "teams", "slack", "gmail", "web"]
        );
        let unique: HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len());
        assert!(rules.iter().all(|r| !r.label.is_empty() && (1..=2).contains(&r.icon.chars().count())));
        assert!(rules.iter().all(|r| !r.app_names.is_empty() && r.contains.is_empty()));
    }

    #[test]
    fn wanted_needs_the_switch_and_one_app() {
        assert!(allowing(&["discord"]).wanted());
        assert!(!allowing(&[]).wanted());
        assert!(!NotificationSettings { enabled: false, ..allowing(&["discord"]) }.wanted());
    }

    #[test]
    fn whatsapp_app_message() {
        let t = toast("WhatsApp", &["Maman", "Tu rentres à quelle heure ce soir ?"]);
        let accepted = accept(&t, &allowing(&["whatsapp"]), 1_000, false).unwrap();
        let n = &accepted.incoming;
        assert_eq!(n.id, "42");
        assert_eq!(n.app, "whatsapp");
        assert_eq!(n.app_label, "WhatsApp");
        assert_eq!(n.monogram, "Wa");
        assert_eq!(n.sender, "Maman");
        assert_eq!(n.text, "Tu rentres à quelle heure ce soir ?");
        assert!(!n.masked);
        assert_eq!(n.received_at, 1_000);
        assert!(accepted.from_app);
        assert_eq!(accepted.target.programs, ["WhatsApp.exe", "WhatsApp.Root.exe"]);
        assert!(accepted.target.titled.contains(&"msedge.exe".to_owned()));
        assert_eq!(accepted.target.label, "WhatsApp");
    }

    #[test]
    fn whatsapp_group_keeps_the_author_in_the_text() {
        let n = read(&toast("WhatsApp", &["Famille 🏡", "Papa: On mange à 20 h"]), &allowing(&["whatsapp"])).unwrap();
        assert_eq!(n.sender, "Famille 🏡");
        assert_eq!(n.text, "Papa: On mange à 20 h");
    }

    #[test]
    fn whatsapp_web_installed_as_an_app_from_edge() {
        // Edge › « Installer ce site en tant qu'application »: the toasts carry its name.
        let t = toast("WhatsApp Web", &["Léa", "📷 Photo"]);
        let accepted = accept(&t, &allowing(&["whatsapp"]), 0, false).unwrap();
        assert_eq!(accepted.incoming.app, "whatsapp");
        assert_eq!(accepted.incoming.text, "📷 Photo");
        assert!(accepted.target.titled.contains(&"msedge.exe".to_owned()));
    }

    #[test]
    fn a_chrome_message_without_a_whatsapp_tab_is_a_web_notification() {
        // Windows does not give the site line: without a WhatsApp Web tab seen, only
        // « Sites web » can show it.
        let t = toast("Google Chrome", &["Maman", "Tu as pensé au pain ?"]);
        assert_eq!(read(&t, &allowing(&["whatsapp"])), None);
        let accepted = accept(&t, &allowing(&["web"]), 0, false).unwrap();
        assert_eq!(accepted.incoming.app_label, "Sites web");
        assert_eq!(accepted.incoming.sender, "Maman");
        assert_eq!(accepted.incoming.text, "Tu as pensé au pain ?");
        let chrome =
            OpenTarget { programs: vec!["chrome.exe".into()], titled: vec![], label: String::new(), whatsapp_tab: false };
        assert_eq!(accepted.target, chrome);
        assert!(!accepted.from_app);
    }

    #[test]
    fn an_edge_message_without_a_whatsapp_tab() {
        let accepted = accept(&toast("Microsoft Edge", &["Léa", "On se voit demain ?"]), &allowing(&["web"]), 0, false).unwrap();
        assert_eq!(accepted.incoming.sender, "Léa");
        assert_eq!(accepted.target.programs, ["msedge.exe"]);
    }

    #[test]
    fn whatsapp_web_in_a_firefox_tab_counts_for_whatsapp() {
        // Firefox gives two lines, sender then message, under its own name.
        let t = toast("Mozilla Firefox", &["Maman", "Tu rentres à quelle heure ?"]);
        let settings = allowing(&["whatsapp"]);
        assert_eq!(tab_lookup(&t, &settings), Some("firefox.exe"));
        let accepted = accept(&t, &settings, 0, true).unwrap();
        let n = &accepted.incoming;
        assert_eq!((n.app.as_str(), n.app_label.as_str(), n.monogram.as_str()), ("whatsapp", "WhatsApp", "Wa"));
        assert_eq!((n.sender.as_str(), n.text.as_str()), ("Maman", "Tu rentres à quelle heure ?"));
        assert!(!accepted.from_app, "a browser has no WhatsApp logo to show");
        assert_eq!(
            accepted.target,
            OpenTarget { programs: vec!["firefox.exe".into()], titled: vec![], label: String::new(), whatsapp_tab: true }
        );
    }

    #[test]
    fn whatsapp_web_in_chrome_and_edge_tabs_too() {
        let settings = allowing(&["whatsapp"]);
        for (browser, program) in [("Google Chrome", "chrome.exe"), ("Microsoft Edge", "msedge.exe"), ("Firefox", "firefox.exe")] {
            let t = toast(browser, &["Léa", "📷 Photo"]);
            assert_eq!(tab_lookup(&t, &settings), Some(program), "{browser}");
            assert_eq!(accept(&t, &settings, 0, true).unwrap().incoming.app, "whatsapp", "{browser}");
        }
    }

    #[test]
    fn without_a_whatsapp_tab_a_firefox_message_stays_in_sites_web() {
        let t = toast("Mozilla Firefox", &["Boîte de réception", "Nouveau courriel"]);
        assert_eq!(accept(&t, &allowing(&["whatsapp", "web"]), 0, false).unwrap().incoming.app, "web");
        assert_eq!(accept(&t, &allowing(&["whatsapp"]), 0, false), None);
    }

    #[test]
    fn the_tabs_are_read_only_when_it_can_help() {
        let firefox = toast("Mozilla Firefox", &["Maman", "Coucou"]);
        // WhatsApp not checked: nothing to credit.
        assert_eq!(tab_lookup(&firefox, &allowing(&["web"])), None);
        // Recognition turned off.
        let off = NotificationSettings { whatsapp_tabs: false, ..allowing(&["whatsapp"]) };
        assert_eq!(tab_lookup(&firefox, &off), None);
        // No WhatsApp rule left in the user's rules.
        let mut no_rule = allowing(&["whatsapp", "web"]);
        no_rule.rules.retain(|rule| rule.id != "whatsapp");
        assert_eq!(tab_lookup(&firefox, &no_rule), None);
        // The app itself, or a browser whose tabs îlot does not read.
        assert_eq!(tab_lookup(&toast("WhatsApp", &["Maman", "Coucou"]), &allowing(&["whatsapp"])), None);
        assert_eq!(tab_lookup(&toast("Opera", &["Maman", "Coucou"]), &allowing(&["whatsapp"])), None);
    }

    #[test]
    fn turned_off_the_recognition_credits_nothing_to_whatsapp() {
        let t = toast("Mozilla Firefox", &["Maman", "Coucou"]);
        let off = NotificationSettings { whatsapp_tabs: false, ..allowing(&["whatsapp", "web"]) };
        let accepted = accept(&t, &off, 0, true).unwrap();
        assert_eq!(accepted.incoming.app, "web");
        assert!(!accepted.target.whatsapp_tab);
        // An app's own toast is never moved by a tab.
        let discord = accept(&toast("Discord", &["kenji", "ok"]), &allowing(&["discord", "whatsapp"]), 0, true).unwrap();
        assert_eq!(discord.incoming.app, "discord");
    }

    #[test]
    fn the_app_and_the_tab_give_one_fingerprint_even_masked() {
        let settings = NotificationSettings { hide_content: true, ..allowing(&["whatsapp"]) };
        let app = accept(&toast("WhatsApp", &["Maman", "Tu rentres ?"]), &settings, 0, false).unwrap();
        let tab = accept(&toast("Mozilla Firefox", &["Maman", "Tu rentres ?"]), &settings, 3_000, true).unwrap();
        assert_eq!(app.print, tab.print);
        let other = accept(&toast("Mozilla Firefox", &["Maman", "J'arrive"]), &settings, 3_000, true).unwrap();
        assert_ne!(app.print, other.print, "masking hides the text, the fingerprint still tells messages apart");
    }

    #[test]
    fn discord_app_channel_message() {
        let t = toast("Discord", &["kenji (#général, Projet RPG)", "on teste la carte ce soir ?"]);
        let accepted = accept(&t, &allowing(&["discord"]), 0, false).unwrap();
        assert_eq!(accepted.incoming.app_label, "Discord");
        assert_eq!(accepted.incoming.sender, "kenji (#général, Projet RPG)");
        assert_eq!(accepted.incoming.text, "on teste la carte ce soir ?");
        assert_eq!(accepted.target.programs, ["Discord.exe", "DiscordPTB.exe", "DiscordCanary.exe"]);
        assert!(accepted.from_app);
    }

    #[test]
    fn variant_names_match_and_look_alikes_do_not() {
        assert!(read(&toast("Discord PTB", &["kenji", "t'es là ?"]), &allowing(&["discord"])).is_some());
        let teams = toast("Microsoft Teams (work or school)", &["Camille Martin", "Réunion décalée à 14 h"]);
        assert_eq!(read(&teams, &allowing(&["teams"])).unwrap().app, "teams");
        assert_eq!(read(&toast("Telegram Desktop", &["Groupe TP", "ok"]), &allowing(&["telegram"])).unwrap().app, "telegram");
        assert!(read(&toast("Discordia", &["a", "b"]), &allowing(&["discord"])).is_none());
    }

    #[test]
    fn unchecked_and_unknown_apps_stay_hidden() {
        let settings = allowing(&["discord"]);
        assert_eq!(read(&toast("WhatsApp", &["Maman", "Coucou"]), &settings), None);
        assert_eq!(read(&toast("Google Chrome", &["Maman", "Coucou"]), &settings), None);
        assert_eq!(read(&toast("Spotify", &["Nouveau titre", "Lo-fi beats"]), &settings), None);
    }

    #[test]
    fn contains_narrows_a_rule() {
        let mut settings = allowing(&["web"]);
        settings.rules = vec![AppRule { contains: vec!["Urgent".into()], ..default_rules().pop().unwrap() }];
        assert!(read(&toast("Microsoft Edge", &["Prof", "URGENT : salle changée"]), &settings).is_some());
        assert!(read(&toast("Microsoft Edge", &["Pub", "−50 % ce week-end"]), &settings).is_none());
    }

    #[test]
    fn hidden_content_keeps_only_the_sender() {
        let settings = NotificationSettings { hide_content: true, ..allowing(&["whatsapp"]) };
        let n = read(&toast("WhatsApp", &["Maman", "Le code de la porte est 4512"]), &settings).unwrap();
        assert_eq!(n.sender, "Maman");
        assert_eq!(n.text, "");
        assert!(n.masked);
    }

    #[test]
    fn a_toast_without_text_uses_the_app_name() {
        let n = read(&toast("Slack", &[]), &allowing(&["slack"])).unwrap();
        assert_eq!(n.sender, "Slack");
        assert_eq!(n.text, "");
    }

    #[test]
    fn several_body_lines_are_kept_on_separate_lines() {
        let n = read(&toast("Telegram", &["Groupe TP", "Alex: ok", "Sam: j'arrive"]), &allowing(&["telegram"])).unwrap();
        assert_eq!(n.text, "Alex: ok\nSam: j'arrive");
    }

    #[test]
    fn invisible_and_control_characters_are_removed() {
        let t = toast("WhatsApp", &["Ma\u{202E}man\u{200B}", "Salut\u{0007} \u{2066}toi\u{2069}\r\n\n  à plus "]);
        let n = read(&t, &allowing(&["whatsapp"])).unwrap();
        assert_eq!(n.sender, "Maman");
        assert_eq!(n.text, "Salut toi\nà plus");
        // Emoji sequences keep their joiner.
        assert_eq!(clean("👨\u{200D}👩\u{200D}👧", 10), "👨\u{200D}👩\u{200D}👧");
    }

    #[test]
    fn long_texts_are_cut_with_an_ellipsis() {
        let long = "a".repeat(1000);
        let n = read(&toast("WhatsApp", &[&long, &long]), &allowing(&["whatsapp"])).unwrap();
        assert_eq!(n.sender.chars().count(), MAX_SENDER_CHARS);
        assert!(n.sender.ends_with('…'));
        assert_eq!(n.text.chars().count(), MAX_TEXT_CHARS);
        assert_eq!(truncate("court", 10), "court");
        assert_eq!(truncate("é".repeat(5).as_str(), 3), "éé…");
    }

    #[test]
    fn browsers() {
        assert_eq!(browser_program("Google Chrome"), Some("chrome.exe"));
        assert_eq!(browser_program("Google Chrome Beta"), Some("chrome.exe"));
        assert_eq!(browser_program("Microsoft Edge"), Some("msedge.exe"));
        assert_eq!(browser_program("Firefox Developer Edition"), Some("firefox.exe"));
        assert_eq!(browser_program("WhatsApp Web"), None);
    }

    #[test]
    fn new_notifications_are_found_by_id() {
        let known: HashSet<u32> = [1, 2, 3].into_iter().collect();
        assert_eq!(new_ids(&known, &[2, 3, 4, 5]), [4, 5]);
        assert!(new_ids(&known, &[1, 2]).is_empty());
        assert_eq!(new_ids(&HashSet::new(), &[7]), [7]);
    }

    #[test]
    fn base64_matches_rfc_4648() {
        for (input, expected) in
            [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")]
        {
            assert_eq!(base64(input.as_bytes()), expected);
        }
    }

    #[test]
    fn only_png_and_jpeg_logos_become_data_urls() {
        assert_eq!(image_data_url(b"\x89PNG\r\n\x1a\nxyz").unwrap(), "data:image/png;base64,iVBORw0KGgp4eXo=");
        assert!(image_data_url(&[0xFF, 0xD8, 0xFF, 0xE0]).unwrap().starts_with("data:image/jpeg;base64,"));
        assert_eq!(image_data_url(b"<svg onload=alert(1)>"), None);
    }

    #[test]
    fn open_targets_are_bounded_and_found_by_id() {
        let (wake, _receiver) = mpsc::channel();
        let state = Notifications::new(wake);
        for i in 0..(MAX_TARGETS + 5) {
            let target =
                OpenTarget { programs: vec!["slack.exe".into()], titled: vec![], label: format!("App {i}"), whatsapp_tab: false };
            state.remember(i.to_string(), target);
        }
        assert_eq!(state.target("0"), None);
        assert_eq!(state.target(&(MAX_TARGETS + 4).to_string()).unwrap().label, format!("App {}", MAX_TARGETS + 4));
    }

    #[test]
    fn status_changes_are_reported_once() {
        let (wake, _receiver) = mpsc::channel();
        let state = Notifications::new(wake);
        let polling = Status { access: Access::Allowed, identity: false, listening: true, polling: true };
        assert!(state.set_status(polling));
        assert!(!state.set_status(polling));
        assert_eq!(state.status(), polling);
    }

    #[test]
    fn status_serializes_for_the_interface() {
        let json = serde_json::to_string(&Status::idle(Access::NotAsked, false)).unwrap();
        assert_eq!(json, r#"{"access":"notAsked","identity":false,"listening":false,"polling":false}"#);
    }

    #[test]
    fn settings_round_trip_with_camel_case() {
        let parsed: NotificationSettings =
            serde_json::from_str(r#"{"enabled":true,"allowed":["discord"],"hideContent":true}"#).unwrap();
        assert!(parsed.enabled && parsed.hide_content);
        assert_eq!(parsed.rules, default_rules());
        // Settings saved before the WhatsApp Web tabs keep them recognized.
        assert!(parsed.whatsapp_tabs);
        let json = serde_json::to_value(&parsed).unwrap();
        assert_eq!(json["rules"][0]["appNames"][0], "WhatsApp");
        assert_eq!(json["whatsappTabs"], true);
        let off: NotificationSettings = serde_json::from_str(r#"{"whatsappTabs":false}"#).unwrap();
        assert!(!off.whatsapp_tabs);
    }

    #[test]
    fn the_sites_rule_speaks_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(default_rules().iter().find(|r| r.id == WEB).unwrap().label, "Websites");
        let mut saved = allowing(&["web"]);
        for rule in saved.rules.iter_mut().filter(|r| r.id == WEB) {
            rule.label = "Sites web".into();
        }
        let english = saved.localized();
        assert_eq!(english.rules.iter().find(|r| r.id == WEB).unwrap().label, "Websites");
        let mut own = allowing(&["web"]);
        for rule in own.rules.iter_mut().filter(|r| r.id == WEB) {
            rule.label = "Mes sites".into();
        }
        assert_eq!(own.localized().rules.iter().find(|r| r.id == WEB).unwrap().label, "Mes sites", "a name of the user's stays");
    }
}
