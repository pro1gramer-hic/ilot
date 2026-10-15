//! The app behind a media session (Spotify, a browser…): its visible windows, read to
//! recognize the site that plays, and brought to the foreground by « Ouvrir ».
//! Nothing here runs a command: windows are found by process name and activated, and
//! packaged apps (Microsoft Store) are activated by Windows from their app id.

use crate::locale::Text;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Com::{CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::{AO_NONE, ApplicationActivationManager, IApplicationActivationManager, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{BOOL, HSTRING, PCWSTR, PWSTR, w};

const NOT_FOUND: Text<'static> = Text::AppWindowMissing;

/// Executable names of the app behind an SMTC app id ("msedge.exe" for "MSEdge").
pub fn process_names(app_id: &str) -> Vec<String> {
    let id = app_id.to_lowercase();
    const KNOWN: [(&str, &str); 8] = [
        ("spotify", "spotify.exe"),
        ("msedge", "msedge.exe"),
        ("chrome", "chrome.exe"),
        ("firefox", "firefox.exe"),
        ("308046b0af4a39cb", "firefox.exe"),
        ("brave", "brave.exe"),
        ("opera", "opera.exe"),
        ("vivaldi", "vivaldi.exe"),
    ];
    if let Some((_, exe)) = KNOWN.iter().find(|(key, _)| id.contains(key)) {
        return vec![(*exe).to_owned()];
    }
    let file = id.rsplit(['\\', '/']).next().unwrap_or(&id);
    if file.ends_with(".exe") { vec![file.to_owned()] } else { Vec::new() }
}

/// The window to bring forward: the first one showing the media, or else the first one
/// (windows come in Z order, so the one the user saw last).
pub fn pick_window(windows: &[(isize, String)], media_title: &str) -> Option<isize> {
    let title = media_title.trim().to_lowercase();
    windows
        .iter()
        .find(|(_, text)| !title.is_empty() && text.to_lowercase().contains(&title))
        .or_else(|| windows.first())
        .map(|(hwnd, _)| *hwnd)
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
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        path.rsplit('\\').next().map(str::to_lowercase)
    }
}

struct Search<'a> {
    names: &'a [String],
    found: Vec<(isize, String)>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam is the `Search` passed by `windows_of`, alive during EnumWindows.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };
    // SAFETY: read-only queries on a window handle given by EnumWindows.
    let visible_top_level = unsafe { IsWindowVisible(hwnd).as_bool() && GetWindow(hwnd, GW_OWNER).is_err() };
    if visible_top_level {
        let mut text = [0u16; 512];
        // SAFETY: the buffer is a properly sized local.
        let len = unsafe { GetWindowTextW(hwnd, &mut text) };
        if len > 0 && process_name(hwnd).is_some_and(|name| search.names.contains(&name)) {
            search.found.push((hwnd.0 as isize, String::from_utf16_lossy(&text[..len as usize])));
        }
    }
    true.into()
}

/// Visible top-level windows of these processes, with their titles, in Z order.
pub fn windows_of(names: &[String]) -> Vec<(isize, String)> {
    if names.is_empty() {
        return Vec::new();
    }
    let mut search = Search { names, found: Vec::new() };
    // SAFETY: `collect` only uses the pointer during this call.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut search as *mut Search as isize)) };
    search.found
}

/// Titles of the visible windows of these processes ("Ma vidéo - YouTube - Microsoft Edge").
pub fn window_titles(names: &[String]) -> Vec<String> {
    windows_of(names).into_iter().map(|(_, title)| title).collect()
}

/// Microsoft Store apps have ids like "Publisher.App_hash!Entry".
pub fn is_packaged(app_id: &str) -> bool {
    app_id.split_once('!').is_some_and(|(package, entry)| package.contains('_') && !entry.is_empty())
}

/// Asks Windows to activate an installed packaged app; a running app comes to the front.
fn activate_packaged(app_id: &str) -> windows::core::Result<u32> {
    // SAFETY: COM is joined in the multithreaded apartment (a no-op when already done) and
    // the activation manager only receives the app id, as a null-terminated string.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let manager: IApplicationActivationManager =
            CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER)?;
        manager.ActivateApplication(&HSTRING::from(app_id), PCWSTR::null(), AO_NONE)
    }
}

/// Brings the app that plays to the foreground: its window, or else Windows activates the
/// packaged app. Spotify hidden in the notification area has no window: its own
/// `spotify:` link opens it.
pub fn bring_to_front(app_id: &str, media_title: &str) -> Result<(), String> {
    let windows = windows_of(&process_names(app_id));
    if let Some(raw) = pick_window(&windows, media_title) {
        let hwnd = HWND(raw as *mut _);
        // SAFETY: the handle comes from EnumWindows; both calls accept a stale handle.
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            return if SetForegroundWindow(hwnd).as_bool() { Ok(()) } else { Err(NOT_FOUND.into()) };
        }
    }
    if is_packaged(app_id) && activate_packaged(app_id).is_ok() {
        return Ok(());
    }
    if app_id.to_lowercase().contains("spotify") {
        // SAFETY: constant, null-terminated strings; no data from the session is used.
        let result = unsafe { ShellExecuteW(None, w!("open"), w!("spotify:"), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
        // ShellExecuteW reports success with a value above 32.
        if result.0 as isize > 32 {
            return Ok(());
        }
    }
    Err(NOT_FOUND.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_names_of_known_apps() {
        assert_eq!(process_names("Spotify.exe"), ["spotify.exe"]);
        assert_eq!(process_names("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), ["spotify.exe"]);
        assert_eq!(process_names("MSEdge"), ["msedge.exe"]);
        assert_eq!(process_names("MSEdge._crx_agimnkijcaahngcdmfeangaknmldooml"), ["msedge.exe"]);
        assert_eq!(process_names("Chrome"), ["chrome.exe"]);
        assert_eq!(process_names("308046B0AF4A39CB"), ["firefox.exe"]);
    }

    #[test]
    fn process_names_of_other_apps() {
        assert_eq!(process_names(r"C:\Program Files\VideoLAN\VLC\vlc.exe"), ["vlc.exe"]);
        assert_eq!(process_names("Foobar.EXE"), ["foobar.exe"]);
        assert!(process_names("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic").is_empty());
    }

    #[test]
    fn packaged_apps_are_recognized() {
        assert!(is_packaged("15EF7777.Crunchyroll_mgdgtskya6f22!App"));
        assert!(is_packaged("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"));
        assert!(!is_packaged("Spotify.exe"));
        assert!(!is_packaged("MSEdge"));
        assert!(!is_packaged("Weird!"));
    }

    #[test]
    fn the_window_showing_the_media_wins() {
        let windows = vec![
            (1, "Boîte de réception - Outlook - Microsoft Edge".to_owned()),
            (2, "Pluie de nuit - YouTube - Microsoft Edge".to_owned()),
        ];
        assert_eq!(pick_window(&windows, "pluie de nuit"), Some(2));
        assert_eq!(pick_window(&windows, "Autre"), Some(1));
        assert_eq!(pick_window(&windows, ""), Some(1));
        assert_eq!(pick_window(&[], "Pluie"), None);
    }

    /// Lists the browser and Spotify windows of this PC, without touching them.
    #[test]
    #[ignore = "needs a Windows desktop"]
    fn lists_real_windows() {
        for app in ["MSEdge", "Chrome", "Spotify.exe", "308046B0AF4A39CB"] {
            println!("{app}: {:?}", window_titles(&process_names(app)));
        }
    }

    #[test]
    fn a_missing_window_is_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let error = bring_to_front(r"C:\nowhere\ilot-test-nothing.exe", "ilot-test-no-title").unwrap_err();
        assert_eq!(error, "Couldn't find this app's window.");
    }
}
