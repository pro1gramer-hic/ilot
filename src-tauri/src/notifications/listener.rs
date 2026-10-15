//! The Windows side of notifications: package identity, access, and the thread that
//! watches toasts with `UserNotificationListener`
//! (https://learn.microsoft.com/windows/apps/develop/notifications/app-notifications/notification-listener).

use super::duplicates::Recent;
use super::{Access, Accepted, EVENT_INCOMING, EVENT_STATUS, Notifications, Status, Toast, Wake};
use super::{accept, foreground, image_data_url, new_ids, settings_of, tab_lookup, uia};
use crate::window::WINDOW_LABEL;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use windows::ApplicationModel::AppInfo;
use windows::Foundation::{Size, TypedEventHandler};
use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus};
use windows::UI::Notifications::{
    KnownNotificationBindings, NotificationKinds, UserNotification, UserNotificationChangedEventArgs,
};
use windows::Win32::Foundation::{E_FAIL, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::core::PWSTR;

/// Safety net while Windows' change event works: access can be withdrawn silently.
const CHECK: Duration = Duration::from_secs(10);
/// Without the change event (no identity), îlot looks this often at the size and date of
/// Windows' notification store, which costs nearly nothing...
const WATCH: Duration = Duration::from_millis(500);
/// ...and reads the list (about 30 ms of processor) only when they move, or this often.
const SAFETY_READ: Duration = Duration::from_secs(5);
/// Without the event nor the store, îlot reads the list this often.
const POLL: Duration = Duration::from_secs(1);
/// Time given to find a WhatsApp Web tab when a browser's notification arrives.
const TAB_SEARCH: Duration = Duration::from_millis(700);
const MAX_LOGO_BYTES: u32 = 64 * 1024;
const LOGO_SIZE: Size = Size { Width: 32.0, Height: 32.0 };

/// Full name of îlot's package identity, if Windows gave it one: the call fails with
/// APPMODEL_ERROR_NO_PACKAGE otherwise
/// (https://devblogs.microsoft.com/insidemsix/is-this-a-packaged-process/).
pub fn package_full_name() -> Option<String> {
    let mut length = 0u32;
    // SAFETY: without a buffer, the call only writes the needed length.
    let rc = unsafe { GetCurrentPackageFullName(&mut length, None) };
    if rc != ERROR_INSUFFICIENT_BUFFER {
        return None;
    }
    let mut buffer = vec![0u16; length as usize];
    // SAFETY: the buffer holds `length` UTF-16 units, as Windows asked.
    let rc = unsafe { GetCurrentPackageFullName(&mut length, Some(PWSTR(buffer.as_mut_ptr()))) };
    let end = (length as usize).saturating_sub(1).min(buffer.len());
    (rc == ERROR_SUCCESS).then(|| String::from_utf16_lossy(&buffer[..end]))
}

fn to_access(status: UserNotificationListenerAccessStatus) -> Access {
    match status {
        UserNotificationListenerAccessStatus::Allowed => Access::Allowed,
        UserNotificationListenerAccessStatus::Denied => Access::Denied,
        _ => Access::NotAsked,
    }
}

/// Access as Windows sees it now. Without package identity, Windows still answers (and
/// lets îlot read when « Accès aux notifications » is on), but refuses the change event.
pub fn access() -> Access {
    UserNotificationListener::Current().and_then(|l| l.GetAccessStatus()).map_or(Access::Unavailable, to_access)
}

/// Asks the user. `RequestAccessAsync` must start on the UI thread; the answer is then
/// awaited here, off the UI thread, while Windows shows its prompt.
pub fn request_access(app: &AppHandle) -> Access {
    let (sender, receiver) = mpsc::channel();
    let asked = app.run_on_main_thread(move || {
        let _ = sender.send(UserNotificationListener::Current().and_then(|l| l.RequestAccessAsync()));
    });
    if asked.is_err() {
        return Access::Unavailable;
    }
    match receiver.recv() {
        Ok(Ok(operation)) => operation.join().map_or(Access::Unavailable, to_access),
        _ => Access::Unavailable,
    }
}

/// App name and text lines of a toast. Its first binding is used when there is no
/// `ToastGeneric` one (older templates, used by Firefox).
fn read_toast(notification: &UserNotification) -> windows::core::Result<Toast> {
    let app_name = notification.AppInfo()?.DisplayInfo()?.DisplayName()?.to_string_lossy();
    let visual = notification.Notification()?.Visual()?;
    let binding = match visual.GetBinding(&KnownNotificationBindings::ToastGeneric()?) {
        Ok(binding) => binding,
        Err(_) => visual.Bindings()?.into_iter().next().ok_or_else(|| windows::core::Error::from(E_FAIL))?,
    };
    let texts = binding
        .GetTextElements()?
        .into_iter()
        .filter_map(|element| element.Text().ok())
        .map(|text| text.to_string_lossy())
        .collect();
    Ok(Toast { id: notification.Id()?, app_name, texts })
}

/// The app's logo as a `data:` URL, read from Windows (small PNG or JPEG only).
fn read_logo(app: &AppInfo) -> Option<String> {
    let stream = app.DisplayInfo().ok()?.GetLogo(LOGO_SIZE).ok()?.OpenReadAsync().ok()?.join().ok()?;
    let size = u32::try_from(stream.Size().ok()?).ok().filter(|size| (1..=MAX_LOGO_BYTES).contains(size))?;
    let buffer = Buffer::Create(size).ok()?;
    let filled = stream.ReadAsync(&buffer, size, InputStreamOptions::None).ok()?.join().ok()?;
    let mut bytes = vec![0u8; filled.Length().ok()? as usize];
    DataReader::FromBuffer(&filled).ok()?.ReadBytes(&mut bytes).ok()?;
    image_data_url(&bytes)
}

/// Size and write time of Windows' notification store (`wpndatabase.db` and its log in
/// %LOCALAPPDATA%\Microsoft\Windows\Notifications). Undocumented, so only a hint that
/// something changed: the files are never opened.
type Stamp = Vec<(u64, Option<SystemTime>)>;

fn store_stamp() -> Option<Stamp> {
    let folder = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join(r"Microsoft\Windows\Notifications");
    let stamp: Stamp = ["wpndatabase.db", "wpndatabase.db-wal"]
        .iter()
        .filter_map(|name| std::fs::metadata(folder.join(name)).ok())
        .map(|meta| (meta.len(), meta.modified().ok()))
        .collect();
    (!stamp.is_empty()).then_some(stamp)
}

/// Whether to read the list now, and the wait before the next look.
fn plan(first: bool, event: bool, stamp: Option<&Stamp>, previous: Option<&Stamp>, since_read: Duration) -> (bool, Duration) {
    match (event, stamp) {
        (true, _) => (true, CHECK),
        (false, None) => (true, POLL),
        (false, Some(stamp)) => (first || previous != Some(stamp) || since_read >= SAFETY_READ, WATCH),
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// A running listener.
struct Session {
    listener: UserNotificationListener,
    /// Change subscription; `None` when Windows refused it (îlot then polls).
    token: Option<i64>,
    /// Ids already seen; `None` until the first read, which only takes stock so that
    /// old notifications are never shown again.
    known: Option<HashSet<u32>>,
}

impl Session {
    fn open(wake: Sender<Wake>) -> windows::core::Result<Self> {
        let listener = UserNotificationListener::Current()?;
        let handler = TypedEventHandler::<UserNotificationListener, UserNotificationChangedEventArgs>::new(move |_, _| {
            let _ = wake.send(Wake::Changed);
            Ok(())
        });
        let token = listener.NotificationChanged(&handler).ok();
        Ok(Self { listener, token, known: None })
    }

    /// Notifications that appeared since the previous read.
    fn arrivals(&mut self) -> windows::core::Result<Vec<UserNotification>> {
        let list = self.listener.GetNotificationsAsync(NotificationKinds::Toast)?.join()?;
        let items: Vec<(u32, UserNotification)> =
            list.into_iter().filter_map(|n| n.Id().ok().map(|id| (id, n))).collect();
        let ids: Vec<u32> = items.iter().map(|(id, _)| *id).collect();
        let fresh = self.known.as_ref().map(|known| new_ids(known, &ids)).unwrap_or_default();
        self.known = Some(ids.into_iter().collect());
        Ok(items.into_iter().filter(|(id, _)| fresh.contains(id)).map(|(_, n)| n).collect())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(token) = self.token {
            let _ = self.listener.RemoveNotificationChanged(token);
        }
    }
}

struct Watcher {
    app: AppHandle,
    wake: Sender<Wake>,
    session: Option<Session>,
    /// Logos by app id, for this session only.
    logos: HashMap<String, Option<String>>,
    /// Store stamp seen at the last look (polling only).
    stamp: Option<Stamp>,
    last_read: Instant,
    /// Messages shown during the last seconds, to skip the copy the app or the tab sends.
    recent: Recent,
}

pub fn spawn(app: AppHandle, wake: Sender<Wake>, receiver: Receiver<Wake>) -> std::io::Result<()> {
    let watcher = Watcher {
        app,
        wake,
        session: None,
        logos: HashMap::new(),
        stamp: None,
        last_read: Instant::now(),
        recent: Recent::default(),
    };
    std::thread::Builder::new().name("ilot-notifications".into()).spawn(move || watcher.run(&receiver))?;
    Ok(())
}

impl Watcher {
    fn run(mut self, receiver: &Receiver<Wake>) {
        let mut wait = self.tick();
        loop {
            let woken = match wait {
                None => receiver.recv().is_ok(),
                Some(delay) => !matches!(receiver.recv_timeout(delay), Err(RecvTimeoutError::Disconnected)),
            };
            if !woken {
                return;
            }
            wait = self.tick();
        }
    }

    /// One pass: starts or stops listening, checks access before every read (as Windows
    /// asks: a withdrawn access silently gives an empty list) and hands over what arrived.
    /// Returns the wait before the next pass; `None` sleeps until the settings change.
    fn tick(&mut self) -> Option<Duration> {
        let settings = settings_of(&self.app);
        let identity = package_full_name().is_some();
        if !settings.wanted() {
            self.stop();
            self.publish(Status::idle(access(), identity));
            return None;
        }
        if self.session.is_none() {
            let access = access();
            if access != Access::Allowed {
                self.publish(Status::idle(access, identity));
                return Some(CHECK);
            }
            self.session = Session::open(self.wake.clone()).ok();
        }
        let Some(session) = self.session.as_mut() else {
            self.publish(Status::idle(Access::Unavailable, identity));
            return Some(CHECK);
        };
        let event = session.token.is_some();
        let stamp = if event { None } else { store_stamp() };
        let first = session.known.is_none();
        let (read, wait) = plan(first, event, stamp.as_ref(), self.stamp.as_ref(), self.last_read.elapsed());
        self.stamp = stamp;
        if read {
            let access = access();
            if access != Access::Allowed {
                self.stop();
                self.publish(Status::idle(access, identity));
                return Some(CHECK);
            }
            let Ok(fresh) = session.arrivals() else {
                self.stop();
                self.publish(Status::idle(Access::Unavailable, identity));
                return Some(CHECK);
            };
            self.last_read = Instant::now();
            // Tabs are read once per browser for the notifications that arrived together.
            let mut tabs = HashMap::new();
            for notification in &fresh {
                self.deliver(notification, &settings, &mut tabs);
            }
        }
        self.publish(Status { access: Access::Allowed, identity, listening: true, polling: !event });
        Some(wait)
    }

    /// Stops listening and forgets everything read so far.
    fn stop(&mut self) {
        self.session = None;
        self.stamp = None;
        self.logos.clear();
        self.recent.clear();
    }

    /// Sends one notification to the island if it passes the rules and was not already
    /// shown by the app or the tab. A broken toast is skipped without stopping the others.
    /// A browser's notification first looks for a WhatsApp Web tab in that browser
    /// (`tabs` keeps the answer per browser for this batch).
    fn deliver(
        &mut self,
        notification: &UserNotification,
        settings: &super::NotificationSettings,
        tabs: &mut HashMap<&'static str, bool>,
    ) {
        let Ok(toast) = read_toast(notification) else { return };
        let whatsapp_tab = tab_lookup(&toast, settings).is_some_and(|program| {
            *tabs.entry(program).or_insert_with(|| {
                let deadline = Instant::now() + TAB_SEARCH;
                uia::find_whatsapp(&foreground::top_level_windows(), program, deadline).is_some()
            })
        });
        let now = now_ms();
        let Some(Accepted { mut incoming, target, from_app, print }) = accept(&toast, settings, now, whatsapp_tab) else {
            return;
        };
        if self.recent.is_copy(print, &toast.app_name, now) {
            return;
        }
        if from_app {
            incoming.icon = notification.AppInfo().ok().and_then(|app| self.logo(&app));
        }
        self.app.state::<Notifications>().remember(incoming.id.clone(), target);
        let _ = self.app.emit_to(WINDOW_LABEL, EVENT_INCOMING, &incoming);
    }

    fn logo(&mut self, app: &AppInfo) -> Option<String> {
        let key = app.AppUserModelId().ok()?.to_string_lossy();
        self.logos.entry(key).or_insert_with(|| read_logo(app)).clone()
    }

    fn publish(&self, status: Status) {
        if self.app.state::<Notifications>().set_status(status) {
            let _ = self.app.emit_to(WINDOW_LABEL, EVENT_STATUS, status);
        }
    }
}

/// One line per toast: its app, the rule it matches and its length. The text only with
/// `--show-text`, so that a console read by someone else (an AI agent's transcript) never
/// gets the content of the user's messages.
#[cfg(debug_assertions)]
fn probe_report(toast: &Toast, accepted: Option<&Accepted>, show_text: bool) -> String {
    let rule = accepted.map_or_else(|| "aucune règle".to_owned(), |a| format!("règle {}", a.incoming.app_label));
    let length: usize = toast.texts.iter().map(|text| text.chars().count()).sum();
    let mut report = format!("#{} · {:?} · {rule} · {length} caractères", toast.id, toast.app_name);
    if show_text {
        if let Some(a) = accepted {
            report += &format!("\n    expéditeur {:?} · texte {:?}", a.incoming.sender, a.incoming.text);
        }
        report += &format!("\n    textes bruts : {:?}", toast.texts);
    }
    report
}

/// Prints the toasts Windows keeps (console only), to check by hand what îlot can read.
#[cfg(debug_assertions)]
pub fn probe() -> i32 {
    let show_text = std::env::args().any(|arg| arg == "--show-text");
    println!("îlot · sonde des notifications (console seulement, rien n'est écrit sur disque)");
    if !show_text {
        println!("Textes masqués : --show-text les affiche (jamais dans la console d'un agent IA).");
    }
    match package_full_name() {
        Some(name) => println!("Identité : {name}"),
        None => println!("Identité manquante : ilot.exe tourne sans identité de package (voir packaging/identity)."),
    }
    let access = access();
    println!("Accès : {access:?}");
    if access != Access::Allowed {
        println!("Autorise l'accès dans îlot (Réglages › Notifications), puis relance la sonde.");
        return 2;
    }
    let Ok(listener) = UserNotificationListener::Current() else {
        println!("UserNotificationListener indisponible.");
        return 3;
    };
    let ignore = TypedEventHandler::<UserNotificationListener, UserNotificationChangedEventArgs>::new(|_, _| Ok(()));
    match listener.NotificationChanged(&ignore) {
        Ok(token) => {
            println!("Événement NotificationChanged : disponible (îlot est prévenu à chaque message)");
            let _ = listener.RemoveNotificationChanged(token);
        }
        Err(error) => {
            println!("Événement NotificationChanged : refusé ({error}) ; îlot surveillera le magasin de notifications");
        }
    }
    let list = match listener.GetNotificationsAsync(NotificationKinds::Toast).and_then(|op| op.join()) {
        Ok(list) => list,
        Err(error) => {
            println!("Lecture impossible : {error}");
            return 4;
        }
    };
    let every_app = super::NotificationSettings {
        enabled: true,
        allowed: super::default_rules().into_iter().map(|rule| rule.id).collect(),
        ..super::NotificationSettings::default()
    };
    // WhatsApp Web as it is open now (the toasts below may be older than the tabs).
    let windows = foreground::top_level_windows();
    let mut tabs: HashMap<&'static str, bool> = HashMap::new();
    for program in ["firefox.exe", "chrome.exe", "msedge.exe"] {
        let start = Instant::now();
        let found = uia::find_whatsapp(&windows, program, start + TAB_SEARCH).is_some();
        println!("Onglet WhatsApp Web dans {program} : {} ({:?})", if found { "oui" } else { "non" }, start.elapsed());
        tabs.insert(program, found);
    }
    println!("{} notification(s) dans le centre de notifications :", list.Size().unwrap_or(0));
    for notification in &list {
        match read_toast(&notification) {
            Ok(toast) => {
                let in_tab = tab_lookup(&toast, &every_app).is_some_and(|program| tabs.get(program) == Some(&true));
                println!("{}", probe_report(&toast, accept(&toast, &every_app, 0, in_tab).as_ref(), show_text));
            }
            Err(error) => println!("    notification illisible : {error}"),
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::super::{AppRule, NotificationSettings};
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_programs_have_no_identity() {
        // cargo's test programs are never in the identity package.
        assert_eq!(package_full_name(), None);
    }

    #[test]
    fn reads_on_every_event_otherwise_when_the_store_moves() {
        let a: Stamp = vec![(10, None)];
        let b: Stamp = vec![(20, None)];
        let soon = Duration::from_millis(600);
        // With Windows' change event, every wake-up reads.
        assert_eq!(plan(false, true, None, None, soon), (true, CHECK));
        // Without it: the first look takes stock, then only a moved store reads...
        assert_eq!(plan(true, false, Some(&a), None, soon), (true, WATCH));
        assert_eq!(plan(false, false, Some(&a), Some(&a), soon), (false, WATCH));
        assert_eq!(plan(false, false, Some(&b), Some(&a), soon), (true, WATCH));
        // ...or a few seconds without reading, in case the store moved elsewhere.
        assert_eq!(plan(false, false, Some(&a), Some(&a), SAFETY_READ), (true, WATCH));
        // Without the store either: plain polling.
        assert_eq!(plan(false, false, None, None, soon), (true, POLL));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn the_probe_shows_no_text_unless_asked() {
        let toast = Toast { id: 7, app_name: "WhatsApp".into(), texts: vec!["Maman".into(), "Rentre vite".into()] };
        let rule = AppRule {
            id: "whatsapp".into(),
            label: "WhatsApp".into(),
            icon: "W".into(),
            app_names: vec!["WhatsApp".into()],
            ..AppRule::default()
        };
        let settings =
            NotificationSettings { enabled: true, allowed: vec!["whatsapp".into()], hide_content: false, rules: vec![rule], whatsapp_tabs: true };
        let accepted = accept(&toast, &settings, 0, false);
        let quiet = probe_report(&toast, accepted.as_ref(), false);
        assert_eq!(quiet, "#7 · \"WhatsApp\" · règle WhatsApp · 16 caractères");
        assert_eq!(probe_report(&toast, None, false), "#7 · \"WhatsApp\" · aucune règle · 16 caractères");
        let shown = probe_report(&toast, accepted.as_ref(), true);
        assert!(shown.starts_with(&quiet) && shown.contains("Rentre vite"), "{shown}");
    }

    /// Shows a fake notification (Send-TestToast.ps1), reads it back through the same path
    /// as îlot, then removes it. Prints only that fake notification, what Windows allows a
    /// program without identity, and the cost of one read of the list.
    #[test]
    #[ignore = "shows a fake Windows notification"]
    fn reads_a_fake_toast_end_to_end() {
        let listener = UserNotificationListener::Current().unwrap();
        println!("sans identité · accès {:?}", listener.GetAccessStatus().map(to_access));
        let ignore = TypedEventHandler::<UserNotificationListener, UserNotificationChangedEventArgs>::new(|_, _| Ok(()));
        let event = listener.NotificationChanged(&ignore);
        println!("sans identité · événement NotificationChanged {:?}", event.as_ref().map(|_| "accepté"));
        if let Ok(token) = event {
            listener.RemoveNotificationChanged(token).unwrap();
        }
        assert_eq!(listener.GetAccessStatus().map(to_access).unwrap(), Access::Allowed);
        let mut session = Session { listener: listener.clone(), token: None, known: None };
        assert!(session.arrivals().unwrap().is_empty(), "the first read only takes stock");

        let stamp_before = store_stamp();
        let title = format!("Test îlot {}", std::process::id());
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../packaging/identity/Send-TestToast.ps1");
        let sent = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&script)
            .args(["-Title", &title, "-Message", "Message factice"])
            .status()
            .unwrap();
        assert!(sent.success());

        let start = Instant::now();
        let (notification, toast) = loop {
            let ours = session
                .arrivals()
                .unwrap()
                .into_iter()
                .find_map(|n| read_toast(&n).ok().filter(|t| t.texts.first() == Some(&title)).map(|t| (n, t)));
            if let Some(found) = ours {
                break found;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "fake notification not seen");
            std::thread::sleep(Duration::from_millis(200));
        };
        println!("lue en {:?} : {toast:?}", start.elapsed());
        println!("magasin de notifications modifié : {}", store_stamp() != stamp_before);
        // The site line (placement="attribution") is not given to listeners.
        assert_eq!(toast.texts, [title.as_str(), "Message factice"]);

        let rule = AppRule {
            id: "test".into(),
            label: "Test".into(),
            icon: "T".into(),
            app_names: vec![toast.app_name.clone()],
            ..AppRule::default()
        };
        let settings =
            NotificationSettings { enabled: true, allowed: vec!["test".into()], hide_content: false, rules: vec![rule], whatsapp_tabs: true };
        let accepted = accept(&toast, &settings, 0, false).unwrap();
        assert_eq!((accepted.incoming.sender.as_str(), accepted.incoming.text.as_str()), (title.as_str(), "Message factice"));
        listener.RemoveNotification(notification.Id().unwrap()).unwrap();

        let reads = 20;
        let cpu_before = process_cpu();
        let start = Instant::now();
        for _ in 0..reads {
            session.arrivals().unwrap();
        }
        let cpu = process_cpu() - cpu_before;
        println!("une lecture de la liste : {:?} ({:?} de processeur)", start.elapsed() / reads, cpu / reads);
    }

    /// Processor time used by this process (user + kernel).
    fn process_cpu() -> Duration {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
        let (mut created, mut exited, mut kernel, mut user) =
            (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        // SAFETY: four valid out-pointers on the current process.
        unsafe { GetProcessTimes(GetCurrentProcess(), &mut created, &mut exited, &mut kernel, &mut user) }.unwrap();
        let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        Duration::from_nanos((ticks(kernel) + ticks(user)) * 100)
    }
}
