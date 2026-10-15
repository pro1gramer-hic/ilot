//! « Ouvrir »: brings the window of the app (or browser) that sent a notification to the
//! front, and selects the WhatsApp Web tab when the message came from it. Windows are
//! found by program file name, taken from the rules, and tabs by their title: nothing
//! that comes from the notification itself is ever run.

use super::OpenTarget;
use super::uia::find_whatsapp;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible, SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow, WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, PWSTR, w};

/// A top-level window the user could switch to.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub hwnd: isize,
    /// Full path of the program that owns the window.
    pub program: String,
    pub title: String,
}

/// True when the program file of `path` is one of `programs` (case does not matter).
pub fn program_matches(path: &str, programs: &[String]) -> bool {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    programs.iter().any(|program| program.eq_ignore_ascii_case(file))
}

/// Time « Ouvrir » gives to find the WhatsApp Web tab among the browser's tabs.
const TAB_SEARCH: Duration = Duration::from_millis(1000);

pub fn program_path(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    // SAFETY: plain queries on a window handle given by EnumWindows; the process handle
    // is closed before returning and the buffer length is passed along with it.
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let read = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut length);
        let _ = CloseHandle(process);
        read.ok()?;
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

unsafe extern "system" fn collect(hwnd: HWND, list: LPARAM) -> BOOL {
    // SAFETY: `list` is the vector passed by `top_level_windows`, alive during EnumWindows.
    let list = unsafe { &mut *(list.0 as *mut Vec<Candidate>) };
    // SAFETY: read-only queries on a handle given by EnumWindows.
    unsafe {
        let owned = GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.is_invalid());
        let tool = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as isize != 0;
        if !IsWindowVisible(hwnd).as_bool() || owned || tool {
            return true.into();
        }
        let mut title = [0u16; 256];
        let length = GetWindowTextW(hwnd, &mut title).max(0) as usize;
        if length == 0 {
            return true.into();
        }
        if let Some(program) = program_path(hwnd) {
            list.push(Candidate {
                hwnd: hwnd.0 as isize,
                program,
                title: String::from_utf16_lossy(&title[..length]),
            });
        }
    }
    true.into()
}

/// Visible top-level windows, frontmost first.
pub fn top_level_windows() -> Vec<Candidate> {
    let mut list: Vec<Candidate> = Vec::new();
    // SAFETY: the callback only uses the vector during the call.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut list as *mut Vec<Candidate> as isize)) };
    list
}

/// The window « Ouvrir » should bring forward: one of the app's own windows (preferably
/// titled after it), otherwise a browser window titled after the app (a site installed as
/// an app). Windows come in z-order, the frontmost first.
pub fn choose<'a>(windows: &'a [Candidate], target: &OpenTarget) -> Option<&'a Candidate> {
    let label = target.label.to_lowercase();
    let named = |c: &&Candidate| !label.is_empty() && c.title.to_lowercase().contains(&label);
    let own: Vec<&Candidate> = windows.iter().filter(|c| program_matches(&c.program, &target.programs)).collect();
    own.iter()
        .copied()
        .find(named)
        .or_else(|| own.first().copied())
        .or_else(|| windows.iter().filter(|c| program_matches(&c.program, &target.titled)).find(named))
}

fn activate(hwnd: isize) -> bool {
    let hwnd = HWND(hwnd as *mut _);
    // SAFETY: the handle comes from EnumWindows; both calls accept a window that closed since.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd).as_bool()
    }
}

/// Brings the target's window to the front; false when none is open. For WhatsApp Web, the
/// browser window that holds it comes forward with that tab selected; when the tab is gone,
/// the browser comes forward as for any site. Called off the interface thread: UI
/// Automation must not run on a thread that owns windows.
pub fn bring_to_front(target: &OpenTarget) -> bool {
    let windows = top_level_windows();
    if target.whatsapp_tab {
        let deadline = Instant::now() + TAB_SEARCH;
        if let Some(found) = target.programs.first().and_then(|program| find_whatsapp(&windows, program, deadline)) {
            found.select_tab();
            return activate(found.hwnd);
        }
    }
    choose(&windows, target).is_some_and(|window| activate(window.hwnd))
}

/// Opens Windows Settings › Confidentialité › Notifications. The address is a constant
/// (https://learn.microsoft.com/windows/apps/develop/launch/launch-settings).
pub fn open_privacy_settings() -> bool {
    // SAFETY: constant, null-terminated strings; ShellExecuteW returns a value above 32
    // on success.
    let result = unsafe { ShellExecuteW(None, w!("open"), w!("ms-settings:privacy-notifications"), None, None, SW_SHOWNORMAL) };
    result.0 as isize > 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_file_names_match_without_case() {
        let programs = vec!["Discord.exe".to_owned()];
        assert!(program_matches(r"C:\Users\léa\AppData\Local\Discord\app-1.0.9\Discord.exe", &programs));
        assert!(program_matches("discord.EXE", &programs));
        assert!(!program_matches(r"C:\Discord.exe\notdiscord.exe", &programs));
        assert!(!program_matches(r"C:\Program Files\Slack\slack.exe", &programs));
    }

    fn target(programs: &[&str], titled: &[&str], label: &str) -> OpenTarget {
        let owned = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect();
        OpenTarget { programs: owned(programs), titled: owned(titled), label: label.to_owned(), whatsapp_tab: false }
    }

    #[test]
    fn the_app_window_comes_first_then_a_browser_window_named_after_it() {
        let windows = [
            Candidate { hwnd: 1, program: r"C:\Edge\msedge.exe".into(), title: "Recherche - Microsoft Edge".into() },
            Candidate { hwnd: 2, program: r"C:\Edge\msedge.exe".into(), title: "WhatsApp".into() },
            Candidate { hwnd: 3, program: r"C:\Discord\Discord.exe".into(), title: "#général | Projet RPG - Discord".into() },
        ];
        let browsers = ["msedge.exe", "chrome.exe"];
        assert_eq!(choose(&windows, &target(&["Discord.exe"], &browsers, "Discord")).unwrap().hwnd, 3);
        assert_eq!(choose(&windows, &target(&["WhatsApp.exe"], &browsers, "WhatsApp")).unwrap().hwnd, 2);
        assert_eq!(choose(&windows, &target(&["slack.exe"], &browsers, "Slack")), None);
        // A browser tab notification: any window of that browser, the frontmost first.
        assert_eq!(choose(&windows, &target(&["msedge.exe"], &[], "")).unwrap().hwnd, 1);
        // Among the app's own windows, the one named after it.
        assert_eq!(choose(&windows, &target(&["msedge.exe"], &[], "WhatsApp")).unwrap().hwnd, 2);
    }

    /// Lists the programs whose windows « Ouvrir » could reach (titles are not printed).
    #[test]
    #[ignore = "reads the real desktop"]
    fn lists_real_windows() {
        let windows = top_level_windows();
        assert!(!windows.is_empty());
        for candidate in windows {
            println!("{}", candidate.program.rsplit('\\').next().unwrap_or_default());
        }
    }
}
