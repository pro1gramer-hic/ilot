//! Last resort of « Détacher la vidéo » (docs/09-DEMO-FINALE.md, 5 bis, step 3): the
//! window that plays the video, a browser or the Crunchyroll app, is made small and kept
//! above the others, then put back exactly as it was. Only its place changes: nothing is
//! read from it, drawn or copied, and its own player keeps showing the video with its
//! rights. Windows lets an app move another app's window of the same user
//! (https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos).

use crate::locale::Text;
use crate::players::Area;
use crate::source_app::{pick_window, process_names};
use std::ffi::c_void;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoTaskMemFree};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongW, GetWindowPlacement, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOPMOST, IsIconic, IsWindow, IsWindowVisible, IsZoomed,
    SW_RESTORE, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPlacement,
    SetWindowPos, ShowWindowAsync, WINDOWPLACEMENT, WS_EX_TOPMOST,
};
use windows::core::{BOOL, PWSTR};

/// Logical size of the lifted window: about the smallest a browser keeps usable, its
/// video then close to the size of îlot's own small player.
const LIFTED: (f64, f64) = (560.0, 400.0);
/// Gap to the screen edges (logical), as for îlot's players.
const MARGIN: f64 = 24.0;
/// After the first move, when the window is checked again: a browser restoring itself from
/// maximized (Edge, Chrome) applies its own saved size a moment later, over îlot's.
const RECHECK_MS: [u64; 4] = [120, 300, 600, 1000];
/// Store apps (UWP) are framed by this process, whose windows carry the app's name.
const FRAME_HOST: &str = "applicationframehost.exe";
/// What a browser puts in the app identity of a web app it installed (YouTube or
/// Crunchyroll « installed from Chrome »): "Chrome._crx_<id>".
const WEB_APP_MARK: &str = "_crx_";

const NOT_FOUND: Text<'static> = Text::VideoWindowMissing;
const REFUSED: Text<'static> = Text::WindowMoveRefused;

/// A window îlot lifted, and how to put it back.
pub struct Lifted {
    hwnd: isize,
    placement: WINDOWPLACEMENT,
    was_topmost: bool,
}

// ---------------------------------------------------------------------------------------
// Pure rules (tested below)

/// Where the lifted window goes: bottom right of the work area of its screen, in
/// physical pixels (`scale` is the window's DPI over 96): x, y, width, height.
pub fn lifted_frame(work: Area, scale: f64) -> (i32, i32, i32, i32) {
    let width = ((LIFTED.0 * scale).round() as i32).min(work.width);
    let height = ((LIFTED.1 * scale).round() as i32).min(work.height);
    let margin = (MARGIN * scale).round() as i32;
    let x = (work.x + work.width - width - margin).max(work.x);
    let y = (work.y + work.height - height - margin).max(work.y);
    (x, y, width, height)
}

/// A window may show the session: one of the app's own processes, or, for a Store app
/// (no executable in its app id), its frame or its own process with the app's name in
/// the title.
pub fn is_candidate(process: &str, title: &str, names: &[String], app_label: &str) -> bool {
    if !names.is_empty() {
        return names.iter().any(|name| name == process);
    }
    let label = app_label.trim().to_lowercase();
    let own_process = process.trim_end_matches(".exe") == label.replace(' ', "");
    label.chars().count() >= 3 && (process == FRAME_HOST || own_process) && title.to_lowercase().contains(&label)
}

/// A window that may show the session: its handle, title and app identity.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub hwnd: isize,
    pub title: String,
    /// The window's own app identity (`System.AppUserModel.ID`), when it has one.
    pub identity: Option<String>,
    /// Found by its program (`is_candidate`), not only by its identity.
    pub by_program: bool,
}

/// The windows to choose from: the ones that carry the session's own app identity, whatever
/// program draws them (a Store app made of a web page, like Crunchyroll's, plays in an
/// `msedge.exe` window that carries the Store app's id); for a web app installed from a
/// browser (`_crx_`), else any installed web app's window, never the browser's ordinary
/// windows while one exists (browsers may shorten the identity they give the media
/// session); else the candidates found by their program, as before. `found` holds both.
pub fn narrow_to_own_window(found: Vec<Candidate>, app_id: &str) -> Vec<(isize, String)> {
    let id = app_id.to_lowercase();
    let pick = |keep: &dyn Fn(&Candidate) -> bool| -> Vec<(isize, String)> {
        found.iter().filter(|c| keep(c)).map(|c| (c.hwnd, c.title.clone())).collect()
    };
    let identity_is = |c: &Candidate, keep: &dyn Fn(&str) -> bool| c.identity.as_deref().is_some_and(|i| keep(&i.to_lowercase()));
    let own = pick(&|c| identity_is(c, &|identity| identity == id));
    if !own.is_empty() {
        return own;
    }
    if id.contains(WEB_APP_MARK) {
        let web_apps = pick(&|c| c.by_program && identity_is(c, &|identity| identity.contains(WEB_APP_MARK)));
        if !web_apps.is_empty() {
            return web_apps;
        }
    }
    pick(&|c| c.by_program)
}

// ---------------------------------------------------------------------------------------
// Windows

/// The app identity Windows groups a window under on the taskbar: a web app installed
/// from Chrome or Edge has its own (https://learn.microsoft.com/en-us/windows/win32/shell/appids).
fn window_identity(hwnd: HWND) -> Option<String> {
    // SAFETY: COM calls on a window handle from EnumWindows (a stale one only makes them
    // fail); the string PropVariantToStringAlloc gives back is freed once copied.
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd).ok()?;
        let value = store.GetValue(&PKEY_AppUserModel_ID).ok()?;
        let text = PropVariantToStringAlloc(&value).ok()?;
        let identity = text.to_string().ok();
        CoTaskMemFree(Some(text.0 as *const c_void));
        identity.filter(|i| !i.is_empty())
    }
}

fn process_name(hwnd: HWND) -> Option<String> {
    let mut pid = 0;
    // SAFETY: plain Win32 queries; the process handle is closed before returning and the
    // buffer is a properly sized local.
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 512];
        let mut len = buffer.len() as u32;
        let read = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        read.ok()?;
        String::from_utf16_lossy(&buffer[..len as usize]).rsplit('\\').next().map(str::to_lowercase)
    }
}

/// A suspended Store app keeps a visible but cloaked window: it is not on screen.
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    // SAFETY: the attribute is a DWORD written into a local of that size.
    let read = unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, (&raw mut cloaked).cast::<c_void>(), size_of::<u32>() as u32)
    };
    read.is_ok() && cloaked != 0
}

struct Search<'a> {
    names: &'a [String],
    label: &'a str,
    /// The session's app id, lower case: a window carrying it is a candidate too.
    app_id: String,
    found: Vec<Candidate>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam is the `Search` passed by `find`, alive during EnumWindows.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };
    // SAFETY: read-only queries on a window handle given by EnumWindows.
    let top_level = unsafe { IsWindowVisible(hwnd).as_bool() && GetWindow(hwnd, GW_OWNER).is_err() };
    if top_level && !is_cloaked(hwnd) {
        let mut text = [0u16; 512];
        // SAFETY: the buffer is a properly sized local.
        let len = unsafe { GetWindowTextW(hwnd, &mut text) };
        let title = String::from_utf16_lossy(&text[..len.max(0) as usize]);
        if len > 0 {
            let by_program = process_name(hwnd).is_some_and(|process| is_candidate(&process, &title, search.names, search.label));
            let identity = window_identity(hwnd);
            let own = identity.as_deref().is_some_and(|i| i.to_lowercase() == search.app_id);
            if by_program || own {
                search.found.push(Candidate { hwnd: hwnd.0 as isize, title, identity, by_program });
            }
        }
    }
    true.into()
}

/// The window that most likely shows the media: the one carrying the session's own app
/// identity; then the one titled with the media, or else the frontmost.
fn find(app_id: &str, media_title: &str, app_label: &str) -> Option<HWND> {
    let names = process_names(app_id);
    // SAFETY: COM on this thread for the windows' property stores; already set up is fine.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let mut search = Search { names: &names, label: app_label, app_id: app_id.to_lowercase(), found: Vec::new() };
    // SAFETY: `collect` only uses the pointer during this call.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut search as *mut Search as isize)) };
    let windows = narrow_to_own_window(search.found, app_id);
    pick_window(&windows, media_title).map(|raw| HWND(raw as *mut _))
}

/// Makes the window that plays the session small and keeps it on top.
pub fn lift(app_id: &str, media_title: &str, app_label: &str) -> Result<Lifted, String> {
    let hwnd = find(app_id, media_title, app_label).ok_or(NOT_FOUND)?;
    let mut placement = WINDOWPLACEMENT { length: size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
    // SAFETY: Win32 calls on a handle from EnumWindows (a stale one only makes them fail);
    // every out-parameter is a properly sized local.
    unsafe {
        GetWindowPlacement(hwnd, &mut placement).map_err(|_| NOT_FOUND)?;
        let was_topmost = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
        if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
            let _ = ShowWindowAsync(hwnd, SW_RESTORE);
        }
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() {
            return Err(NOT_FOUND.into());
        }
        let work = info.rcWork;
        let area = Area { x: work.left, y: work.top, width: work.right - work.left, height: work.bottom - work.top };
        let frame = lifted_frame(area, f64::from(GetDpiForWindow(hwnd).max(96)) / 96.0);
        place(hwnd, frame).map_err(|_| REFUSED)?;
        hold(hwnd.0 as isize, frame);
        Ok(Lifted { hwnd: hwnd.0 as isize, placement, was_topmost })
    }
}

/// Small, bottom right, above the others. Posted to the app's own thread: a busy or frozen
/// app never blocks îlot.
fn place(hwnd: HWND, (x, y, width, height): (i32, i32, i32, i32)) -> windows::core::Result<()> {
    let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_ASYNCWINDOWPOS;
    // SAFETY: a handle from EnumWindows; a stale one only makes the call fail.
    unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, flags) }
}

/// True when the window already has the frame îlot gave it (to the pixel).
pub fn holds(rect: (i32, i32, i32, i32), frame: (i32, i32, i32, i32)) -> bool {
    rect == frame
}

/// The app may put its own size back right after îlot's (a browser coming out of
/// maximized does): for about a second, îlot checks and gives it the small frame again.
fn hold(raw: isize, frame: (i32, i32, i32, i32)) {
    std::thread::spawn(move || {
        let hwnd = HWND(raw as *mut _);
        let mut waited = 0;
        for at in RECHECK_MS {
            std::thread::sleep(std::time::Duration::from_millis(at - waited));
            waited = at;
            let mut rect = RECT::default();
            // SAFETY: the handle is checked first; GetWindowRect fills a local.
            let gone = unsafe { !IsWindow(Some(hwnd)).as_bool() || GetWindowRect(hwnd, &mut rect).is_err() };
            if gone {
                return;
            }
            let now = (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top);
            if !holds(now, frame) {
                let _ = place(hwnd, frame);
            }
        }
    });
}

/// Puts the window back where, as large and as high as it was (if it still exists).
pub fn put_back(lifted: &Lifted) {
    let hwnd = HWND(lifted.hwnd as *mut _);
    // SAFETY: the handle is checked first; both calls accept a window that went away since.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return;
        }
        let _ = SetWindowPlacement(hwnd, &lifted.placement);
        if !lifted.was_topmost {
            let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS;
            let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Area = Area { x: 0, y: 0, width: 1920, height: 1040 };

    #[test]
    fn the_lifted_window_sits_bottom_right() {
        assert_eq!(lifted_frame(SCREEN, 1.0), (1336, 616, 560, 400));
        assert_eq!(lifted_frame(SCREEN, 1.5), (1044, 404, 840, 600));
        let left = Area { x: -1366, y: 0, width: 1366, height: 728 };
        assert_eq!(lifted_frame(left, 1.0), (-584, 304, 560, 400));
        let tiny = Area { x: 0, y: 0, width: 500, height: 300 };
        assert_eq!(lifted_frame(tiny, 1.0), (0, 0, 500, 300));
    }

    #[test]
    fn browsers_are_found_by_their_process() {
        let edge = vec!["msedge.exe".to_owned()];
        assert!(is_candidate("msedge.exe", "Épisode 7 - Regardez sur Crunchyroll - Microsoft Edge", &edge, "Microsoft Edge"));
        assert!(!is_candidate("chrome.exe", "Épisode 7 - Crunchyroll - Google Chrome", &edge, "Microsoft Edge"));
        assert!(!is_candidate("ilot.exe", "Crunchyroll · îlot", &edge, "Microsoft Edge"));
    }

    #[test]
    fn store_apps_are_found_by_their_frame_and_name() {
        assert!(is_candidate("applicationframehost.exe", "Crunchyroll", &[], "Crunchyroll"));
        assert!(is_candidate("crunchyroll.exe", "Crunchyroll", &[], "Crunchyroll"));
        assert!(!is_candidate("msedge.exe", "Épisode 7 - Crunchyroll - Microsoft Edge", &[], "Crunchyroll"));
        assert!(!is_candidate("applicationframehost.exe", "Paramètres", &[], "Crunchyroll"));
        assert!(!is_candidate("applicationframehost.exe", "TV", &[], "TV"), "a name too short matches anything");
    }

    fn window(hwnd: isize, title: &str, identity: Option<&str>) -> Candidate {
        Candidate { hwnd, title: title.into(), identity: identity.map(str::to_owned), by_program: true }
    }

    #[test]
    fn a_web_app_from_chrome_lifts_its_own_window() {
        let browser = window(1, "Épisode 7 - Crunchyroll - Google Chrome", Some("Chrome"));
        let youtube = window(2, "Ma vidéo - YouTube", Some("Chrome._crx_agimnkijcaahngcdmfeangaknmldooml"));
        let crunchyroll = window(3, "Épisode 7", Some("Chrome._crx_okmnfnoapbbfjnjgbeffeblcadbbclkj"));
        let all = vec![browser.clone(), youtube.clone(), crunchyroll.clone()];
        // Its own identity first, whatever the titles say.
        assert_eq!(narrow_to_own_window(all.clone(), "Chrome._crx_okmnfnoapbbfjnjgbeffeblcadbbclkj"), [(3, "Épisode 7".to_owned())]);
        // A shortened identity (seen on a real PC for YouTube): the installed web apps' windows,
        // never the browser's ordinary one, then the title picks.
        let shortened = narrow_to_own_window(all.clone(), "Chrome._crx_agimnkijcamfeangaknmldooml");
        assert_eq!(shortened.iter().map(|w| w.0).collect::<Vec<_>>(), [2, 3]);
        assert_eq!(pick_window(&shortened, "Ma vidéo"), Some(2));
        // No web app window at all: every candidate, as before.
        assert_eq!(narrow_to_own_window(vec![browser.clone()], "Chrome._crx_agimnkijcamfeangaknmldooml").len(), 1);
        // A tab in the browser: the browser's own windows (its identity), not the web apps'.
        assert_eq!(narrow_to_own_window(all.clone(), "Chrome"), [(1, "Épisode 7 - Crunchyroll - Google Chrome".to_owned())]);
        // A session whose identity no window carries: every candidate found by its program.
        assert_eq!(narrow_to_own_window(all, "MSEdge").len(), 3);
    }

    #[test]
    fn the_small_frame_is_given_again_until_it_holds() {
        let frame = lifted_frame(SCREEN, 1.0);
        assert!(holds(frame, frame));
        assert!(!holds((0, 0, 1920, 1040), frame), "the app put its own size back");
        assert!(RECHECK_MS.windows(2).all(|w| w[0] < w[1]) && RECHECK_MS[3] <= 1000);
    }

    #[test]
    fn a_store_app_made_of_a_web_page_lifts_its_edge_window() {
        // Seen on a real PC: the Crunchyroll app of the Store plays in an msedge.exe window
        // that carries the Store app's id; its program alone would not have found it.
        let id = "15EF7777.Crunchyroll_mgdgtskya6f22!App";
        let app = Candidate { by_program: false, ..window(7, "Crunchyroll", Some(id)) };
        let other = window(8, "Paramètres", None);
        assert_eq!(narrow_to_own_window(vec![other.clone(), app], id), [(7, "Crunchyroll".to_owned())]);
        // A window found only by its identity never comes in for another session.
        let stray = Candidate { by_program: false, ..window(9, "Crunchyroll", Some(id)) };
        assert_eq!(narrow_to_own_window(vec![other, stray], "Spotify.exe"), [(8, "Paramètres".to_owned())]);
    }

    /// Lifts the frontmost Edge window for two seconds, then puts it back.
    #[test]
    #[ignore = "moves a real window"]
    fn lifts_and_puts_back_a_real_window() {
        let lifted = lift("MSEdge", "", "Microsoft Edge").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
        put_back(&lifted);
    }

    #[test]
    fn a_missing_window_is_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let error = lift(r"C:\nowhere\ilot-test-nothing.exe", "ilot-test-no-title", "ilot-test-nothing").err().unwrap();
        assert_eq!(error, "Couldn't find this video's window.");
    }
}
