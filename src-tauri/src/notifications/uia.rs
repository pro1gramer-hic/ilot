//! Browser tabs read with UI Automation, the accessibility interface of Windows
//! (https://learn.microsoft.com/windows/win32/winauto/entry-uiauto-win32), to find
//! WhatsApp Web (`web_tabs.rs`). Used only when a browser's notification arrives and on
//! « Ouvrir », never in a loop, and never on a thread that owns a window, in the
//! multithreaded apartment (https://learn.microsoft.com/windows/win32/winauto/uiauto-threading).
//!
//! Only the browser's own interface is walked: the kind of each element, then the names
//! of the tabs. A web page (a Document element) is never entered, so nothing of what a
//! page shows is read. Cost: one call per container of the interface, which brings back
//! its children with their kind (`FindAllBuildCache`), one per tab for its name; bounded by
//! an element budget and a deadline, with shorter UI Automation timeouts. Measured on this
//! PC: 130 to 330 ms for Firefox 157 (7 tabs), 70 to 90 ms for Edge 155. Firefox and
//! Chrome start their accessibility engine the first time a program reads them (about one
//! second), and keep it until they close.

use super::foreground::{Candidate, program_matches};
use super::web_tabs::{Engine, engine, pick_tab, titled_window};
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation, IUIAutomation2, IUIAutomationCacheRequest, IUIAutomationCondition,
    IUIAutomationElement, IUIAutomationInvokePattern, IUIAutomationLegacyIAccessiblePattern,
    IUIAutomationSelectionItemPattern, TreeScope_Children, UIA_ButtonControlTypeId, UIA_CONTROLTYPE_ID,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_ControlTypePropertyId, UIA_DocumentControlTypeId,
    UIA_EditControlTypeId, UIA_HyperlinkControlTypeId, UIA_ImageControlTypeId, UIA_InvokePatternId,
    UIA_LegacyIAccessiblePatternId, UIA_MenuBarControlTypeId, UIA_MenuControlTypeId, UIA_MenuItemControlTypeId,
    UIA_ProgressBarControlTypeId, UIA_RadioButtonControlTypeId, UIA_ScrollBarControlTypeId,
    UIA_SelectionItemPatternId, UIA_SeparatorControlTypeId, UIA_SliderControlTypeId, UIA_SplitButtonControlTypeId,
    UIA_TabControlTypeId, UIA_TabItemControlTypeId, UIA_TextControlTypeId, UIA_TitleBarControlTypeId,
    UIA_ToolTipControlTypeId,
};
use windows::core::Interface;

/// Wait for a browser to hand over an element (2 s by default).
const CONNECTION_TIMEOUT_MS: u32 = 1000;
/// Wait for a browser to answer about an element (20 s by default).
const TRANSACTION_TIMEOUT_MS: u32 = 1000;
/// Elements of the browser's interface looked at, at most, per window.
const MAX_ELEMENTS: usize = 1500;
/// The tab strip sits a few levels under the window.
const MAX_DEPTH: usize = 12;

/// Never entered: a web page (nothing it shows is read), the window's own title and menu
/// bars (slow to answer, through the system menu), and controls that hold no tab.
const SKIPPED: [UIA_CONTROLTYPE_ID; 19] = [
    UIA_DocumentControlTypeId,
    UIA_TitleBarControlTypeId,
    UIA_MenuBarControlTypeId,
    UIA_MenuControlTypeId,
    UIA_MenuItemControlTypeId,
    UIA_ButtonControlTypeId,
    UIA_SplitButtonControlTypeId,
    UIA_ImageControlTypeId,
    UIA_TextControlTypeId,
    UIA_EditControlTypeId,
    UIA_HyperlinkControlTypeId,
    UIA_CheckBoxControlTypeId,
    UIA_RadioButtonControlTypeId,
    UIA_SeparatorControlTypeId,
    UIA_ProgressBarControlTypeId,
    UIA_SliderControlTypeId,
    UIA_ScrollBarControlTypeId,
    UIA_ComboBoxControlTypeId,
    UIA_ToolTipControlTypeId,
];

/// One lookup at a time: UI Automation clients created at the same moment on two threads
/// fail to make their conditions (E_FAIL, seen with eight threads at once), and two
/// lookups would only make the browser answer twice.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// The turn of any UI Automation client of îlot (here, and the window Mochi is dropped
/// on, `window_context.rs`): held for the client's whole life.
pub fn turn() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A UI Automation client for one lookup.
pub struct Automation {
    client: IUIAutomation,
    /// Brings back the kind of each child (`UIA_ControlTypePropertyId`) in the same call.
    kinds: IUIAutomationCacheRequest,
    every: IUIAutomationCondition,
    /// Released last, once the objects above are.
    _turn: MutexGuard<'static, ()>,
}

/// A tab of a browser window, with its name (the page title).
pub struct Tab {
    element: IUIAutomationElement,
    pub name: String,
}

impl Automation {
    /// `None` when UI Automation is not available.
    pub fn new() -> Option<Self> {
        let turn = turn();
        // SAFETY: COM is joined in the multithreaded apartment (a no-op when this thread
        // already is), then plain calls on the objects UI Automation returns.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let client: IUIAutomation = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
            if let Ok(timeouts) = client.cast::<IUIAutomation2>() {
                let _ = timeouts.SetConnectionTimeout(CONNECTION_TIMEOUT_MS);
                let _ = timeouts.SetTransactionTimeout(TRANSACTION_TIMEOUT_MS);
            }
            let every = client.CreateTrueCondition().ok()?;
            let kinds = client.CreateCacheRequest().ok()?;
            kinds.AddProperty(UIA_ControlTypePropertyId).ok()?;
            // Every element, not only those of the control view: a container left out of
            // that view would hide the tabs from a children search.
            kinds.SetTreeFilter(&every).ok()?;
            Some(Self { client, kinds, every, _turn: turn })
        }
    }

    /// The tabs of a window, in tree order. Empty when its tree cannot be read in time.
    ///
    /// Breadth first, so that the browser's bars come before what they hold. The first tab
    /// list met (a `Tab` element) is read before anything else, and the walk ends with it
    /// once it gave tabs: Firefox 157 keeps its tabs under a tool bar, two levels down;
    /// Edge 155 under panes, seven levels down, in a tab list within a tab list.
    pub fn tabs(&self, hwnd: isize, deadline: Instant) -> Vec<Tab> {
        let mut tabs = Vec::new();
        // SAFETY: UI Automation accepts any window handle, even one closed since.
        let Ok(root) = (unsafe { self.client.ElementFromHandle(HWND(hwnd as *mut _)) }) else { return tabs };
        let mut strip: VecDeque<(IUIAutomationElement, usize)> = VecDeque::new();
        let mut rest = VecDeque::from([(root, 0usize)]);
        let mut seen = 0usize;
        loop {
            let in_strip = !strip.is_empty();
            let next = if in_strip { strip.pop_front() } else if tabs.is_empty() { rest.pop_front() } else { None };
            let Some((element, depth)) = next else { break };
            if seen >= MAX_ELEMENTS || Instant::now() >= deadline {
                break;
            }
            // SAFETY: plain calls on elements UI Automation returned; a failure only skips
            // this container.
            unsafe {
                let Ok(children) = element.FindAllBuildCache(TreeScope_Children, &self.every, &self.kinds) else {
                    continue;
                };
                for index in 0..children.Length().unwrap_or(0) {
                    let Ok(child) = children.GetElement(index) else { continue };
                    seen += 1;
                    let Ok(kind) = child.CachedControlType() else { continue };
                    if kind == UIA_TabItemControlTypeId {
                        if let Ok(name) = child.CurrentName() {
                            tabs.push(Tab { element: child, name: name.to_string() });
                        }
                    } else if depth + 1 < MAX_DEPTH && !SKIPPED.contains(&kind) {
                        let queue = if in_strip || kind == UIA_TabControlTypeId { &mut strip } else { &mut rest };
                        queue.push_back((child, depth + 1));
                    }
                }
            }
        }
        tabs
    }
}

impl Tab {
    /// Makes this tab the active one: a tab item selects itself (SelectionItem); Invoke,
    /// then the default action of the older accessibility interface, are the fallbacks.
    pub fn select(&self) -> bool {
        let element = &self.element;
        // SAFETY: plain calls on an element UI Automation returned; an unsupported pattern
        // comes back as an error.
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId)
                .and_then(|pattern| pattern.Select())
                .is_ok()
                || element
                    .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                    .and_then(|pattern| pattern.Invoke())
                    .is_ok()
                || element
                    .GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(UIA_LegacyIAccessiblePatternId)
                    .and_then(|pattern| pattern.DoDefaultAction())
                    .is_ok()
        }
    }
}

/// Where WhatsApp Web is: the window, and its tab when it is not the active one.
pub struct Found {
    pub hwnd: isize,
    /// The tab, with the client that read it (dropped after it, keeping the turn).
    tab: Option<(Tab, Automation)>,
}

impl Found {
    /// Makes WhatsApp Web the active tab of its window, when it is not already. A tab that
    /// refuses leaves the window as it is: it still comes to the front.
    pub fn select_tab(&self) {
        if let Some((tab, _)) = &self.tab {
            tab.select();
        }
    }
}

/// Looks for WhatsApp Web in the windows of a browser (`windows` comes frontmost first):
/// first in their titles, then in their tabs until the deadline.
pub fn find_whatsapp(windows: &[Candidate], program: &str, deadline: Instant) -> Option<Found> {
    let engine: Engine = engine(program)?;
    let programs = [program.to_owned()];
    let browser: Vec<&Candidate> = windows.iter().filter(|c| program_matches(&c.program, &programs)).collect();
    if let Some(hwnd) = titled_window(browser.iter().map(|c| (c.hwnd, c.title.as_str()))) {
        return Some(Found { hwnd, tab: None });
    }
    if browser.is_empty() {
        return None;
    }
    let automation = Automation::new()?;
    for window in browser {
        let mut tabs = automation.tabs(window.hwnd, deadline);
        let names: Vec<String> = tabs.iter().map(|tab| tab.name.clone()).collect();
        if let Some(index) = pick_tab(&names, engine) {
            return Some(Found { hwnd: window.hwnd, tab: Some((tabs.swap_remove(index), automation)) });
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::web_tabs::tab_unread;
    use super::*;
    use std::time::Duration;

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(2)
    }

    #[test]
    fn without_a_browser_window_nothing_is_read() {
        let others = [Candidate { hwnd: 1, program: r"C:\Discord\Discord.exe".into(), title: "WhatsApp".into() }];
        assert!(find_whatsapp(&others, "firefox.exe", soon()).is_none());
        assert!(find_whatsapp(&[], "firefox.exe", soon()).is_none());
        // Browsers whose tabs îlot does not know.
        let opera = [Candidate { hwnd: 1, program: r"C:\Opera\opera.exe".into(), title: "WhatsApp - Opera".into() }];
        assert!(find_whatsapp(&opera, "opera.exe", soon()).is_none());
    }

    #[test]
    fn a_window_titled_whatsapp_needs_no_accessibility() {
        let windows = [
            Candidate { hwnd: 7, program: r"C:\Firefox\firefox.exe".into(), title: "Le Monde — Mozilla Firefox".into() },
            Candidate { hwnd: 8, program: r"C:\Firefox\firefox.exe".into(), title: "(2) WhatsApp — Mozilla Firefox".into() },
        ];
        let found = find_whatsapp(&windows, "firefox.exe", soon()).unwrap();
        assert_eq!(found.hwnd, 8);
        assert!(found.tab.is_none());
    }

    #[test]
    fn an_unreadable_window_gives_no_tab() {
        let automation = Automation::new().expect("UI Automation is part of Windows");
        // A handle that names no window, then a deadline already passed.
        assert!(automation.tabs(0x7fff_fff0, soon()).is_empty());
        drop(automation);
        let firefox = [Candidate {
            hwnd: 0x7fff_fff0,
            program: r"C:\Firefox\firefox.exe".into(),
            title: "Le Monde — Mozilla Firefox".into(),
        }];
        assert!(find_whatsapp(&firefox, "firefox.exe", soon()).is_none());
        assert!(find_whatsapp(&firefox, "firefox.exe", Instant::now()).is_none());
    }

    /// A window of this test holding a Win32 tab control (comctl32), on its own thread with
    /// a message loop: UI Automation reads it from the test thread as it reads a browser.
    mod win32 {
        use super::super::*;
        use super::soon;
        use std::sync::mpsc;
        use std::thread::JoinHandle;
        use std::time::Duration;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::Controls::{
            ICC_TAB_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, TCIF_TEXT, TCITEMW, TCM_GETCURSEL,
            TCM_INSERTITEMW, WC_TABCONTROLW,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, MSG, PostThreadMessageW, SW_SHOWNOACTIVATE,
            SendMessageW, ShowWindow, TranslateMessage, WINDOW_STYLE, WM_QUIT, WS_CHILD, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
        };
        use windows::core::{PWSTR, w};

        struct TabWindow {
            window: isize,
            tabs: isize,
            thread: u32,
            pump: Option<JoinHandle<()>>,
        }

        impl TabWindow {
            fn open(names: &'static [&'static str]) -> Self {
                let (sender, receiver) = mpsc::channel();
                let pump = std::thread::spawn(move || {
                    // SAFETY: creates, fills and shows windows owned by this thread, then
                    // runs their message loop until the test posts WM_QUIT.
                    unsafe {
                        let classes = INITCOMMONCONTROLSEX {
                            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
                            dwICC: ICC_TAB_CLASSES,
                        };
                        assert!(InitCommonControlsEx(&classes).as_bool());
                        let window = CreateWindowExW(
                            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                            w!("STATIC"),
                            w!("îlot test onglets"),
                            WINDOW_STYLE(WS_POPUP.0),
                            -24_000,
                            -24_000,
                            640,
                            120,
                            None,
                            None,
                            None,
                            None,
                        )
                        .unwrap();
                        let tabs = CreateWindowExW(
                            Default::default(),
                            WC_TABCONTROLW,
                            w!(""),
                            WS_CHILD | WS_VISIBLE,
                            0,
                            0,
                            640,
                            120,
                            Some(window),
                            None,
                            None,
                            None,
                        )
                        .unwrap();
                        for (index, name) in names.iter().enumerate() {
                            let mut text: Vec<u16> = name.encode_utf16().chain([0]).collect();
                            let item = TCITEMW { mask: TCIF_TEXT, pszText: PWSTR(text.as_mut_ptr()), ..Default::default() };
                            SendMessageW(tabs, TCM_INSERTITEMW, Some(WPARAM(index)), Some(LPARAM(&raw const item as isize)));
                        }
                        let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
                        sender.send((window.0 as isize, tabs.0 as isize, GetCurrentThreadId())).unwrap();
                        let mut message = MSG::default();
                        while GetMessageW(&mut message, None, 0, 0).as_bool() {
                            let _ = TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                        let _ = DestroyWindow(window);
                    }
                });
                let (window, tabs, thread) = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
                Self { window, tabs, thread, pump: Some(pump) }
            }

            fn selected(&self) -> isize {
                // SAFETY: the tab control lives on the pump thread, which answers messages.
                unsafe { SendMessageW(HWND(self.tabs as *mut _), TCM_GETCURSEL, None, None).0 }
            }
        }

        impl Drop for TabWindow {
            fn drop(&mut self) {
                // SAFETY: asks the pump thread to leave its loop and destroy its window.
                unsafe {
                    let _ = PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0));
                }
                if let Some(pump) = self.pump.take() {
                    let _ = pump.join();
                }
            }
        }

        #[test]
        fn the_tabs_of_a_window_are_read_then_one_is_selected() {
            let window = TabWindow::open(&["Accueil", "Un article sur WhatsApp — Le Monde", "(3) WhatsApp", "YouTube"]);
            assert_eq!(window.selected(), 0);
            let automation = Automation::new().unwrap();
            let tabs = automation.tabs(window.window, soon());
            let names: Vec<String> = tabs.iter().map(|tab| tab.name.clone()).collect();
            assert_eq!(names, ["Accueil", "Un article sur WhatsApp — Le Monde", "(3) WhatsApp", "YouTube"]);
            let index = pick_tab(&names, Engine::Gecko).unwrap();
            assert_eq!(index, 2);
            assert!(tabs[index].select());
            assert_eq!(window.selected(), 2);
        }

        #[test]
        fn a_window_without_whatsapp_has_no_tab_to_pick() {
            let window = TabWindow::open(&["Accueil", "Un article sur WhatsApp — Le Monde"]);
            let names: Vec<String> =
                Automation::new().unwrap().tabs(window.window, soon()).into_iter().map(|tab| tab.name).collect();
            assert_eq!(names.len(), 2);
            assert_eq!(pick_tab(&names, Engine::Gecko), None);
        }
    }

    /// Which ways of selecting a tab the browser offers (nothing is selected here).
    fn patterns(tab: &Tab) -> String {
        // SAFETY: read-only queries on an element UI Automation returned.
        unsafe {
            let selection = tab.element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId).is_ok();
            let invoke = tab.element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId).is_ok();
            let legacy =
                tab.element.GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(UIA_LegacyIAccessiblePatternId).is_ok();
            format!("SelectionItem {selection} · Invoke {invoke} · LegacyIAccessible {legacy}")
        }
    }

    /// Reads the tab strips of the browsers open on this PC without touching them. Prints
    /// only kinds of elements, counts and durations: never a title nor a tab name.
    #[test]
    #[ignore = "reads the real browsers"]
    fn probes_the_real_browsers() {
        let windows = super::super::foreground::top_level_windows();
        let automation = Automation::new().unwrap();
        for program in ["firefox.exe", "chrome.exe", "msedge.exe"] {
            let programs = [program.to_owned()];
            for window in windows.iter().filter(|c| program_matches(&c.program, &programs)) {
                let start = Instant::now();
                let tabs = automation.tabs(window.hwnd, Instant::now() + Duration::from_secs(5));
                let names: Vec<String> = tabs.iter().map(|tab| tab.name.clone()).collect();
                let elapsed = start.elapsed();
                let whatsapp = names.iter().filter(|name| tab_unread(name, engine(program).unwrap()).is_some()).count();
                println!(
                    "{program} · fenêtre titrée WhatsApp : {} · {} onglet(s), dont {whatsapp} WhatsApp · lu en {elapsed:?}",
                    titled_window([(window.hwnd, window.title.as_str())]).is_some(),
                    tabs.len(),
                );
                if let Some(tab) = tabs.first() {
                    println!("    premier onglet : {}", patterns(tab));
                }
            }
        }
    }
}
