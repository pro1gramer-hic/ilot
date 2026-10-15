//! The main tool's pastille: is VS Code (or Cursor, Codex, Antigravity) open, and which
//! project its window in front shows, read from the window's title. VS Code titles its
//! windows `${dirty}${activeEditorShort}${separator}${rootName}${separator}${profileName}${separator}${appName}`
//! with « - » between the parts on Windows
//! (https://github.com/microsoft/vscode/blob/main/src/vs/workbench/browser/parts/titlebar/windowTitle.ts,
//! `defaultWindowTitle`); `rootName` may carry a remote such as « [WSL: Ubuntu] »
//! (https://code.visualstudio.com/docs/configure/custom-layout). Cursor and Antigravity
//! are VS Code forks that keep this title; the Codex app's title is not documented.
//! Bringing the tool forward activates its own window: no command, no data from outside.

use crate::locale::{Text, tr};
use crate::source_app;
use serde::Serialize;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};

pub struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    /// Executables, lowercase (as source_app compares them).
    processes: &'static [&'static str],
    /// `appName` at the end of its titles, longest first.
    apps: &'static [&'static str],
}

pub static TOOLS: [Tool; 4] = [
    Tool {
        id: "vscode",
        name: "VS Code",
        processes: &["code.exe", "code - insiders.exe"],
        apps: &["Visual Studio Code - Insiders", "Visual Studio Code"],
    },
    Tool { id: "cursor", name: "Cursor", processes: &["cursor.exe"], apps: &["Cursor"] },
    // The Codex app comes from the Microsoft Store; its executable is not documented: both
    // names below are guesses to check on a real PC (docs/JOURNAL.md).
    Tool { id: "codex", name: "Codex", processes: &["codex.exe", "chatgpt.exe"], apps: &["Codex", "ChatGPT"] },
    Tool { id: "antigravity", name: "Antigravity", processes: &["antigravity.exe"], apps: &["Antigravity"] },
];

pub fn find(id: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|t| t.id == id)
}

/// What the pastille shows about the tool.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub open: bool,
    /// The tool's window is the one in front.
    pub focused: bool,
    /// The project of its window in front, as the title reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Every part of the title that may name the project: the interface keeps the one an
    /// agent of îlot works in, if any.
    pub candidates: Vec<String>,
}

/// One part of a title, without a remote or workspace mark: « ilot [WSL: Ubuntu] » → « ilot ».
fn clean(part: &str) -> String {
    let mut part = part.trim();
    if let Some(stripped) = part.strip_suffix(" (Workspace)") {
        part = stripped.trim_end();
    }
    if part.ends_with(']')
        && let Some(open) = part.rfind(" [")
    {
        part = part[..open].trim_end();
    }
    part.to_owned()
}

/// The project a window's title names, and every part that might name it.
pub fn parse_title(title: &str, apps: &[&str]) -> (Option<String>, Vec<String>) {
    let mut title = title.trim().trim_start_matches('\u{25cf}').trim();
    for app in apps {
        if title == *app {
            return (None, Vec::new());
        }
        if let Some(rest) = title.strip_suffix(app).and_then(|rest| rest.strip_suffix(" - ")) {
            title = rest;
            break;
        }
    }
    let mut parts: Vec<String> = Vec::new();
    for part in title.split(" - ").map(clean).filter(|p| !p.is_empty()) {
        if !parts.contains(&part) {
            parts.push(part);
        }
    }
    // « file - root - … »: the root comes second when a file is open, alone otherwise.
    let guess = match parts.as_slice() {
        [] => None,
        [only] => Some(only.clone()),
        [_, root, ..] => Some(root.clone()),
    };
    parts.truncate(4);
    (guess, parts)
}

fn names(tool: &Tool) -> Vec<String> {
    tool.processes.iter().map(|p| (*p).to_owned()).collect()
}

/// Reads the tool's windows now (EnumWindows: a few hundred microseconds).
pub fn look(tool: &Tool) -> ToolView {
    let windows = source_app::windows_of(&names(tool));
    // SAFETY: plain query without arguments.
    let front = unsafe { GetForegroundWindow() }.0 as isize;
    let focused = windows.iter().find(|(hwnd, _)| *hwnd == front);
    let Some((_, title)) = focused.or_else(|| windows.first()) else { return ToolView::default() };
    let (project, candidates) = parse_title(title, tool.apps);
    ToolView { open: true, focused: focused.is_some(), project, candidates }
}

/// Brings the tool's window in front (the last one used).
pub fn bring_to_front(tool: &Tool) -> Result<(), String> {
    let windows = source_app::windows_of(&names(tool));
    let (raw, _) = windows.first().ok_or_else(|| tr(Text::ToolNotOpen { tool: tool.name }))?;
    let hwnd = HWND(*raw as *mut _);
    // SAFETY: the handle comes from EnumWindows; both calls accept a stale handle.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if SetForegroundWindow(hwnd).as_bool() { Ok(()) } else { Err(tr(Text::ToolFrontRefused { tool: tool.name })) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VSCODE: &[&str] = &["Visual Studio Code - Insiders", "Visual Studio Code"];

    #[test]
    fn vs_code_titles_name_their_project() {
        assert_eq!(parse_title("main.rs - ilot - Visual Studio Code", VSCODE).0.as_deref(), Some("ilot"));
        assert_eq!(parse_title("● main.rs - ilot - Visual Studio Code", VSCODE).0.as_deref(), Some("ilot"));
        assert_eq!(parse_title("ilot - Visual Studio Code", VSCODE).0.as_deref(), Some("ilot"));
        assert_eq!(parse_title("Welcome - Visual Studio Code", VSCODE).0.as_deref(), Some("Welcome"));
        assert_eq!(parse_title("Visual Studio Code", VSCODE), (None, vec![]));
        let (project, candidates) = parse_title("pastilles.ts - ilot [WSL: Ubuntu] - Travail - Visual Studio Code - Insiders", VSCODE);
        assert_eq!(project.as_deref(), Some("ilot"));
        assert_eq!(candidates, ["pastilles.ts", "ilot", "Travail"]);
        assert_eq!(parse_title("app.ts - mono (Workspace) - Visual Studio Code", VSCODE).0.as_deref(), Some("mono"));
    }

    #[test]
    fn forks_keep_the_same_title() {
        let cursor = find("cursor").unwrap();
        assert_eq!(parse_title("lib.rs - epilot - Cursor", cursor.apps).0.as_deref(), Some("epilot"));
        let antigravity = find("antigravity").unwrap();
        assert_eq!(parse_title("index.html - site - Antigravity", antigravity.apps).0.as_deref(), Some("site"));
    }

    #[test]
    fn every_tool_is_known_by_its_process() {
        for tool in &TOOLS {
            assert!(tool.processes.iter().all(|p| p.ends_with(".exe") && *p == p.to_lowercase()));
        }
        assert!(find("emacs").is_none());
    }
}
