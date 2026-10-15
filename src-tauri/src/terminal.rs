//! « Aller au terminal »: brings the window where a session runs to the foreground, and
//! says honestly how close it got. The relay tells where the agent runs from its own
//! environment (`RelayEnv`):
//!
//! - `TERM_PROGRAM=vscode`: the terminal of VS Code, or of Cursor (an editor built on it)
//!   (https://code.visualstudio.com/docs/terminal/shell-integration);
//! - `WT_SESSION`: Windows Terminal, which adds it to `WSLENV` for WSL. It names a tab
//!   but nothing documented maps it to one (`wt.exe focus-tab` takes a tab number:
//!   https://learn.microsoft.com/en-us/windows/terminal/command-line-arguments), so the
//!   window comes forward and îlot says that the tab was not identified;
//! - otherwise a console window (PowerShell, cmd, WSL), found by its title.
//!
//! Windows are found by program file name and title (`EnumWindows`), then restored and
//! activated (`SetForegroundWindow`): nothing is ever run, and no value from an event is
//! used for anything but a comparison.

use crate::events::Source;
use crate::locale::{Text, tr};
use ilot_bridge::protocol::RelayEnv;
use serde::Serialize;

/// The application an agent runs in, as far as the relay could tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Host {
    WindowsTerminal,
    VsCode,
    Cursor,
    /// Nothing told: a console window is looked for.
    Console,
}

impl Host {
    /// From what the relay saw; `None` when it sent nothing (an older relay).
    pub fn from_env(env: &RelayEnv, source: Source) -> Option<Self> {
        let cursor = env.cursor || source == Source::Cursor;
        if env.term_program.as_deref() == Some("vscode") {
            return Some(if cursor { Self::Cursor } else { Self::VsCode });
        }
        if env.wt_session.is_some() {
            return Some(Self::WindowsTerminal);
        }
        if env.is_empty() {
            return None;
        }
        // Cursor's own hooks run in the editor, outside any terminal.
        Some(if source == Source::Cursor { Self::Cursor } else { Self::Console })
    }

    /// Program files of its windows, lower case.
    fn programs(self) -> &'static [&'static str] {
        match self {
            Self::WindowsTerminal => &["windowsterminal.exe"],
            Self::VsCode => &["code.exe", "code - insiders.exe"],
            Self::Cursor => &["cursor.exe"],
            Self::Console => &["conhost.exe", "openconsole.exe", "powershell.exe", "pwsh.exe", "cmd.exe", "wsl.exe", "ubuntu.exe", "bash.exe"],
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::WindowsTerminal => "Windows Terminal",
            Self::VsCode => "VS Code",
            Self::Cursor => "Cursor",
            Self::Console => "Le terminal",
        }
    }
}

/// A top-level window: its handle, the file name of its program and its title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub hwnd: isize,
    pub program: String,
    pub title: String,
}

/// How close « Aller au terminal » got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    /// The window of the session's project.
    Project,
    /// The right application, but not its tab or window for sure.
    App,
}

/// The window to bring forward for a session of `project` in `host` (windows come in
/// z-order, the frontmost first): one of the host's windows whose title names the project,
/// else its frontmost one. Without a host, any terminal or console naming the project, then
/// Windows Terminal, then a console.
pub fn choose(host: Option<Host>, project: &str, windows: &[Window]) -> Option<(usize, Host, Precision)> {
    let named = |w: &Window| !project.is_empty() && project != "?" && w.title.to_lowercase().contains(&project.to_lowercase());
    let of = |host: Host| move |w: &&Window| host.programs().contains(&w.program.as_str());
    let hosts: Vec<Host> = match host {
        Some(host) => vec![host],
        None => vec![Host::WindowsTerminal, Host::Console],
    };
    let index = |w: &Window| windows.iter().position(|x| x == w).unwrap_or(0);
    for host in &hosts {
        if let Some(window) = windows.iter().filter(of(*host)).find(|w| named(w)) {
            return Some((index(window), *host, Precision::Project));
        }
    }
    hosts.iter().find_map(|host| windows.iter().find(of(*host)).map(|w| (index(w), *host, Precision::App)))
}

/// What îlot says once it tried, in the language of the moment.
pub fn message(found: Option<(Host, Precision)>, activated: bool, project: &str) -> String {
    tr(match found {
        None => Text::TerminalNotFound,
        Some(_) if !activated => Text::TerminalNotInFront,
        Some((Host::WindowsTerminal, Precision::Project)) => Text::TerminalTabInFront { project },
        Some((Host::WindowsTerminal, Precision::App)) => Text::TerminalInFront,
        Some((host @ (Host::VsCode | Host::Cursor), Precision::Project)) => Text::EditorOpenOn { editor: host.label(), project },
        Some((host @ (Host::VsCode | Host::Cursor), Precision::App)) => Text::EditorInFront { editor: host.label() },
        Some((Host::Console, Precision::Project)) => Text::ConsoleOfProjectInFront { project },
        Some((Host::Console, Precision::App)) => Text::ConsoleInFront,
    })
}

/// The result shown in the island.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub found: bool,
    /// The very window of the session's project came forward.
    pub exact: bool,
    pub message: String,
}

/// Brings the session's window forward. Runs off the interface thread.
pub fn go(host: Option<Host>, project: &str) -> Outcome {
    let windows: Vec<Window> = crate::notifications::foreground::top_level_windows()
        .into_iter()
        .map(|c| Window {
            hwnd: c.hwnd,
            program: c.program.rsplit(['\\', '/']).next().unwrap_or_default().to_lowercase(),
            title: c.title,
        })
        .collect();
    let choice = choose(host, project, &windows);
    let activated = choice.is_some_and(|(index, _, _)| activate(windows[index].hwnd));
    let found = choice.map(|(_, host, precision)| (host, precision));
    Outcome {
        found: found.is_some() && activated,
        exact: activated && matches!(found, Some((_, Precision::Project))),
        message: message(found, activated, project),
    }
}

fn activate(hwnd: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};
    let hwnd = HWND(hwnd as *mut _);
    // SAFETY: the handle comes from EnumWindows; both calls accept a window that closed since.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd).as_bool()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(hwnd: isize, program: &str, title: &str) -> Window {
        Window { hwnd, program: program.into(), title: title.into() }
    }

    fn desktop() -> Vec<Window> {
        vec![
            window(1, "msedge.exe", "epilot - Microsoft Edge"),
            window(2, "code.exe", "api.py - site [WSL: Ubuntu] - Visual Studio Code"),
            window(3, "code.exe", "README.md - epilot [WSL: Ubuntu] - Visual Studio Code"),
            window(4, "windowsterminal.exe", "✳ Corrige le fuseau horaire"),
            window(5, "windowsterminal.exe", "user@pc: ~/projects/epilot"),
            window(6, "conhost.exe", "Windows PowerShell"),
        ]
    }

    fn env(wt: bool, term: Option<&str>, cursor: bool) -> RelayEnv {
        RelayEnv { wt_session: wt.then(|| "8c2a1f0e-6b1d-4c1e-9b0a-1f2e3d4c5b6a".into()), term_program: term.map(Into::into), cursor, cwd: None }
    }

    #[test]
    fn the_host_comes_from_the_relay_s_hints() {
        assert_eq!(Host::from_env(&env(true, Some("vscode"), false), Source::ClaudeCode), Some(Host::VsCode), "VS Code's terminal inside WT? the editor wins");
        assert_eq!(Host::from_env(&env(false, Some("vscode"), true), Source::Cursor), Some(Host::Cursor));
        assert_eq!(Host::from_env(&env(true, None, false), Source::Codex), Some(Host::WindowsTerminal));
        assert_eq!(Host::from_env(&RelayEnv { cwd: Some("/p".into()), ..RelayEnv::default() }, Source::GeminiCli), Some(Host::Console));
        assert_eq!(Host::from_env(&RelayEnv { cursor: true, ..RelayEnv::default() }, Source::Cursor), Some(Host::Cursor));
        assert_eq!(Host::from_env(&RelayEnv::default(), Source::ClaudeCode), None, "an older relay says nothing");
    }

    #[test]
    fn the_vs_code_window_of_the_project_comes_first() {
        let windows = desktop();
        assert_eq!(choose(Some(Host::VsCode), "epilot", &windows), Some((2, Host::VsCode, Precision::Project)));
        assert_eq!(choose(Some(Host::VsCode), "other", &windows), Some((1, Host::VsCode, Precision::App)), "the frontmost one");
        assert_eq!(choose(Some(Host::Cursor), "epilot", &windows), None);
    }

    #[test]
    fn windows_terminal_comes_forward_without_claiming_the_tab() {
        let windows = desktop();
        assert_eq!(choose(Some(Host::WindowsTerminal), "site", &windows), Some((3, Host::WindowsTerminal, Precision::App)));
        assert_eq!(message(Some((Host::WindowsTerminal, Precision::App)), true, "site"), "Terminal mis au premier plan, onglet non identifié.");
        // The active tab's title names the project: that tab is on screen.
        assert_eq!(choose(Some(Host::WindowsTerminal), "epilot", &windows), Some((4, Host::WindowsTerminal, Precision::Project)));
    }

    #[test]
    fn without_a_hint_any_terminal_naming_the_project_then_windows_terminal() {
        let windows = desktop();
        assert_eq!(choose(None, "epilot", &windows), Some((4, Host::WindowsTerminal, Precision::Project)));
        assert_eq!(choose(None, "nothing", &windows), Some((3, Host::WindowsTerminal, Precision::App)));
        let consoles = vec![window(9, "conhost.exe", "Windows PowerShell")];
        assert_eq!(choose(None, "?", &consoles), Some((0, Host::Console, Precision::App)));
        assert_eq!(choose(None, "epilot", &[window(1, "msedge.exe", "epilot")]), None, "a browser is no terminal");
    }

    #[test]
    fn the_message_says_how_close_it_got() {
        assert_eq!(message(Some((Host::VsCode, Precision::Project)), true, "epilot"), "VS Code ouvert sur epilot.");
        assert_eq!(message(Some((Host::Cursor, Precision::App)), true, "epilot"), "Cursor mis au premier plan, fenêtre du projet non identifiée.");
        assert_eq!(message(None, false, "epilot"), "Aucune fenêtre de terminal trouvée pour cette session.");
        assert_eq!(message(Some((Host::VsCode, Precision::Project)), false, "epilot"), "Windows n'a pas laissé îlot passer la fenêtre au premier plan.");
    }

    #[test]
    fn what_ilot_did_is_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(message(Some((Host::VsCode, Precision::Project)), true, "epilot"), "VS Code open on epilot.");
        assert_eq!(message(Some((Host::WindowsTerminal, Precision::Project)), true, "site"), "Windows Terminal brought to the front, on the tab of project site.");
        assert_eq!(message(None, false, "epilot"), "No terminal window found for this session.");
    }
}
