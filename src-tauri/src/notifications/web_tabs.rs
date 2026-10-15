//! WhatsApp Web in a browser tab (docs/05-NOTIFICATIONS.md, « WhatsApp : surtout WhatsApp
//! Web dans Firefox »). A site's notification reaches a listener under the browser's name
//! (« Mozilla Firefox »), with its title and text but never the site. îlot credits it to
//! WhatsApp when that browser shows a tab titled like WhatsApp Web: « WhatsApp », or
//! « (3) WhatsApp » with three unread chats. This file holds the pure part (titles and
//! choices); `uia.rs` reads the tabs from Windows.

/// How a browser names its tabs for accessibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// Firefox: the page title, nothing else.
    Gecko,
    /// Chrome and Edge: the page title, sometimes followed by a state
    /// (« (3) WhatsApp - Lecture audio en cours »).
    Chromium,
}

/// The browsers whose tabs îlot reads, by program file.
pub fn engine(program: &str) -> Option<Engine> {
    match program.to_ascii_lowercase().as_str() {
        "firefox.exe" => Some(Engine::Gecko),
        "chrome.exe" | "msedge.exe" => Some(Engine::Chromium),
        _ => None,
    }
}

/// Titles WhatsApp Web gives its page (a business account shows its own).
const TITLES: [&str; 2] = ["WhatsApp", "WhatsApp Business"];

/// Unread chats announced by a WhatsApp Web title: « (3) WhatsApp » gives 3, « WhatsApp »
/// gives 0. `None` for any other page, even one that speaks of WhatsApp.
pub fn whatsapp_unread(title: &str) -> Option<u32> {
    let title = title.trim();
    let (unread, page) = match title.strip_prefix('(').and_then(|rest| rest.split_once(") ")) {
        Some((count, page)) => (count.strip_suffix('+').unwrap_or(count).parse().ok()?, page),
        None => (0, title),
    };
    TITLES.contains(&page).then_some(unread)
}

/// Unread chats of a tab that holds WhatsApp Web, read from its accessible name.
pub fn tab_unread(name: &str, engine: Engine) -> Option<u32> {
    whatsapp_unread(name).or_else(|| match engine {
        Engine::Gecko => None,
        Engine::Chromium => name.split_once(" - ").and_then(|(page, _)| whatsapp_unread(page)),
    })
}

/// A browser window is titled after its active tab, then the browser
/// (« (3) WhatsApp — Mozilla Firefox », « WhatsApp - Google Chrome »): a quick hint that
/// needs no accessibility. Only the last part is cut, so « WhatsApp — Wikipédia — Mozilla
/// Firefox » is not taken for WhatsApp Web.
pub fn window_unread(title: &str) -> Option<u32> {
    let end = [" — ", " – ", " - "].iter().filter_map(|separator| title.rfind(separator)).max();
    whatsapp_unread(end.map_or(title, |end| &title[..end]))
}

/// The first window, frontmost first, whose active tab is WhatsApp Web.
pub fn titled_window<'a>(titles: impl IntoIterator<Item = (isize, &'a str)>) -> Option<isize> {
    titles.into_iter().find(|(_, title)| window_unread(title).is_some()).map(|(hwnd, _)| hwnd)
}

/// The WhatsApp Web tab among a window's tabs, in strip order: one with unread chats (the
/// one that just notified) wins, else the first.
pub fn pick_tab(names: &[String], engine: Engine) -> Option<usize> {
    names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| tab_unread(name, engine).map(|unread| (index, unread)))
        .max_by_key(|&(index, unread)| (unread > 0, std::cmp::Reverse(index)))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whatsapp_web_titles_with_or_without_unread_chats() {
        assert_eq!(whatsapp_unread("WhatsApp"), Some(0));
        assert_eq!(whatsapp_unread("(3) WhatsApp"), Some(3));
        assert_eq!(whatsapp_unread("(128) WhatsApp"), Some(128));
        assert_eq!(whatsapp_unread("(99+) WhatsApp"), Some(99));
        assert_eq!(whatsapp_unread("  (1) WhatsApp "), Some(1));
        assert_eq!(whatsapp_unread("WhatsApp Business"), Some(0));
        assert_eq!(whatsapp_unread("(2) WhatsApp Business"), Some(2));
    }

    #[test]
    fn pages_that_speak_of_whatsapp_do_not_count() {
        for title in [
            "Un article sur WhatsApp — Le Monde",
            "WhatsApp — Wikipédia",
            "WhatsApp Business Platform",
            "Télécharger WhatsApp",
            "whatsapp",
            "(trois) WhatsApp",
            "() WhatsApp",
            "(3)WhatsApp",
            "(3) WhatsApp Web",
            "Discord",
            "",
        ] {
            assert_eq!(whatsapp_unread(title), None, "{title}");
        }
    }

    #[test]
    fn firefox_tabs_are_named_after_the_page_only() {
        assert_eq!(tab_unread("(3) WhatsApp", Engine::Gecko), Some(3));
        assert_eq!(tab_unread("WhatsApp - Lecture audio en cours", Engine::Gecko), None);
        assert_eq!(tab_unread("Un article sur WhatsApp — Le Monde", Engine::Gecko), None);
    }

    #[test]
    fn chrome_and_edge_tabs_may_add_a_state() {
        assert_eq!(tab_unread("(3) WhatsApp - Lecture audio en cours", Engine::Chromium), Some(3));
        assert_eq!(tab_unread("WhatsApp - Audio playing", Engine::Chromium), Some(0));
        assert_eq!(tab_unread("WhatsApp", Engine::Chromium), Some(0));
        assert_eq!(tab_unread("Un article sur WhatsApp - Le Monde", Engine::Chromium), None);
    }

    #[test]
    fn engines_by_program() {
        assert_eq!(engine("firefox.exe"), Some(Engine::Gecko));
        assert_eq!(engine("FIREFOX.EXE"), Some(Engine::Gecko));
        assert_eq!(engine("chrome.exe"), Some(Engine::Chromium));
        assert_eq!(engine("msedge.exe"), Some(Engine::Chromium));
        assert_eq!(engine("opera.exe"), None);
        assert_eq!(engine("WhatsApp.exe"), None);
    }

    #[test]
    fn window_titles_give_a_quick_hint() {
        assert_eq!(window_unread("(3) WhatsApp — Mozilla Firefox"), Some(3));
        assert_eq!(window_unread("WhatsApp — Navigation privée de Mozilla Firefox"), Some(0));
        assert_eq!(window_unread("WhatsApp - Mozilla Firefox"), Some(0));
        assert_eq!(window_unread("(1) WhatsApp - Google Chrome"), Some(1));
        assert_eq!(window_unread("WhatsApp"), Some(0));
        assert_eq!(window_unread("Un article sur WhatsApp — Le Monde — Mozilla Firefox"), None);
        assert_eq!(window_unread("WhatsApp — Wikipédia — Mozilla Firefox"), None);
        assert_eq!(window_unread("Boîte de réception - Gmail — Mozilla Firefox"), None);
        // Edge adds the other pages and the profile: the tabs will tell.
        assert_eq!(window_unread("(3) WhatsApp et 2 autres pages - Personnel – Microsoft Edge"), None);
    }

    #[test]
    fn the_frontmost_window_showing_whatsapp_wins() {
        let windows = [
            (1, "Le Monde — Mozilla Firefox"),
            (2, "(2) WhatsApp — Mozilla Firefox"),
            (3, "WhatsApp — Mozilla Firefox"),
        ];
        assert_eq!(titled_window(windows), Some(2));
        assert_eq!(titled_window([(1, "Le Monde — Mozilla Firefox")]), None);
        assert_eq!(titled_window([]), None);
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_whatsapp_tab_is_picked_among_the_others() {
        let tabs = names(&["Le Monde", "Un article sur WhatsApp — Le Monde", "(3) WhatsApp", "YouTube"]);
        assert_eq!(pick_tab(&tabs, Engine::Gecko), Some(2));
        assert_eq!(pick_tab(&names(&["Nouvel onglet", "WhatsApp"]), Engine::Gecko), Some(1));
        assert_eq!(pick_tab(&names(&["Nouvel onglet", "WhatsApp Business"]), Engine::Gecko), Some(1));
        assert_eq!(pick_tab(&names(&["Le Monde", "Un article sur WhatsApp — Le Monde"]), Engine::Gecko), None);
        assert_eq!(pick_tab(&[], Engine::Gecko), None);
    }

    #[test]
    fn a_tab_with_unread_chats_wins_then_the_first() {
        assert_eq!(pick_tab(&names(&["WhatsApp", "(1) WhatsApp"]), Engine::Gecko), Some(1));
        assert_eq!(pick_tab(&names(&["(2) WhatsApp", "(5) WhatsApp"]), Engine::Gecko), Some(0));
        assert_eq!(pick_tab(&names(&["WhatsApp", "WhatsApp"]), Engine::Gecko), Some(0));
        assert_eq!(pick_tab(&names(&["YouTube", "(4) WhatsApp - Lecture audio en cours"]), Engine::Chromium), Some(1));
    }
}
