//! Global shortcuts (prompt 12, lot 5): the island's own (`settings.shortcut`) and one per
//! named action (`settings.shortcuts`), all registered here by the core: the interface
//! never holds the plugin's permissions. A shortcut that Windows or another app already
//! holds fails to register (`RegisterHotKey`): the previous one stays and the user reads
//! why. https://v2.tauri.app/plugin/global-shortcut/ ,
//! https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey

use crate::locale::{Text, tr};
use crate::settings::{DEFAULT_SHORTCUT, Settings};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use tauri::Runtime;
use tauri_plugin_global_shortcut::{Code, GlobalShortcut, Modifiers, Shortcut};

/// The island's shortcut opens and closes it (Ctrl+Alt+Space by default).
pub const ISLAND: &str = "toggle-island";

/// Actions a shortcut can start besides the island. Their handlers live in the interface
/// (src/lib/stores/actions.svelte.ts), which only lists those it can really run.
pub const ACTIONS: [&str; 8] = [
    "open-chat",
    "go-to-alert",
    "go-to-terminal",
    "next-pastille",
    "mute",
    "mochi-to-desktop",
    "open-wardrobe",
    "translate-clipboard",
];

/// The shortcut of each action; an action without one has no entry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutSettings {
    /// Action id → shortcut as the interface wrote it ("Ctrl+Alt+C").
    pub actions: BTreeMap<String, String>,
}

impl ShortcutSettings {
    /// Keeps the known actions whose shortcut follows the rules and is nobody else's (the
    /// island first, then the actions in their order).
    pub fn sanitized(self, island: &str) -> Self {
        let mut taken: Vec<u32> = parse(island).map(|s| s.id()).into_iter().collect();
        let mut actions = BTreeMap::new();
        for (action, text) in self.actions {
            let Ok(shortcut) = parse(&text) else { continue };
            if ACTIONS.contains(&action.as_str()) && !taken.contains(&shortcut.id()) {
                taken.push(shortcut.id());
                actions.insert(action, text.trim().to_owned());
            }
        }
        Self { actions }
    }
}

/// Parses a shortcut and applies îlot's rules: Ctrl or Alt in it (a lone key, or Shift
/// and a key, would take that key from every app), never the Windows key (Windows keeps
/// it), never F12 (kept for debuggers).
pub fn parse(text: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = text.trim().parse().map_err(|_| tr(Text::ShortcutInvalid { shortcut: text.trim() }))?;
    if shortcut.mods.contains(Modifiers::SUPER) {
        return Err(Text::ShortcutWindowsKey.into());
    }
    if shortcut.key == Code::F12 {
        return Err(Text::ShortcutF12.into());
    }
    if !shortcut.mods.intersects(Modifiers::CONTROL | Modifiers::ALT) {
        return Err(Text::ShortcutNeedsModifier.into());
    }
    Ok(shortcut)
}

/// Every shortcut of these settings with its action, the island's first (its default
/// when the saved one no longer parses).
pub fn table(settings: &Settings) -> Vec<(String, Shortcut)> {
    let island = parse(&settings.shortcut).or_else(|_| parse(DEFAULT_SHORTCUT));
    let island = island.into_iter().map(|s| (ISLAND.to_owned(), s));
    let actions = settings.shortcuts.actions.iter().filter_map(|(a, t)| parse(t).ok().map(|s| (a.clone(), s)));
    island.chain(actions).collect()
}

/// The settings with `text` as the shortcut of `action` (empty: none). The island always
/// keeps one: it is how the keyboard reaches îlot.
pub fn assign(current: &Settings, action: &str, text: &str) -> Result<Settings, String> {
    let text = text.trim();
    let mut next = current.clone();
    if action != ISLAND && !ACTIONS.contains(&action) {
        return Err(tr(Text::ActionUnknown { action }));
    }
    if text.is_empty() {
        if action == ISLAND {
            return Err(Text::IslandKeepsShortcut.into());
        }
        next.shortcuts.actions.remove(action);
        return Ok(next);
    }
    let shortcut = parse(text)?;
    if table(current).iter().any(|(other, s)| other != action && s.id() == shortcut.id()) {
        return Err(Text::ShortcutUsedByIlot.into());
    }
    if action == ISLAND {
        next.shortcut = text.to_owned();
    } else {
        next.shortcuts.actions.insert(action.to_owned(), text.to_owned());
    }
    Ok(next)
}

/// Who registers shortcuts with Windows: the plugin in the app, a fake in the tests.
pub trait Registrar {
    fn register(&self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&self, shortcut: Shortcut);
}

impl<R: Runtime> Registrar for GlobalShortcut<R> {
    fn register(&self, shortcut: Shortcut) -> Result<(), String> {
        GlobalShortcut::register(self, shortcut).map_err(|e| e.to_string())
    }

    fn unregister(&self, shortcut: Shortcut) {
        let _ = GlobalShortcut::unregister(self, shortcut);
    }
}

/// Which action each registered shortcut starts (by the plugin's shortcut id).
#[derive(Default)]
pub struct Bindings(Mutex<HashMap<u32, String>>);

impl Bindings {
    pub fn action_of(&self, id: u32) -> Option<String> {
        self.0.lock().unwrap().get(&id).cloned()
    }

    fn replace(&self, table: &[(String, Shortcut)]) {
        *self.0.lock().unwrap() = table.iter().map(|(action, s)| (s.id(), action.clone())).collect();
    }
}

/// At launch: registers every shortcut on its own (one taken by another app does not stop
/// the others). Returns the actions whose shortcut could not be registered.
pub fn start(registrar: &impl Registrar, bindings: &Bindings, settings: &Settings) -> Vec<String> {
    let mut registered = Vec::new();
    let mut failed = Vec::new();
    for (action, shortcut) in table(settings) {
        match registrar.register(shortcut) {
            Ok(()) => registered.push((action, shortcut)),
            Err(_) => failed.push(action),
        }
    }
    bindings.replace(&registered);
    failed
}

/// Registers the shortcuts `next` has and `previous` had not, then lets go of those it
/// no longer has. All or nothing: when Windows refuses one, those just registered go and
/// the previous ones stay.
pub fn sync(registrar: &impl Registrar, bindings: &Bindings, previous: &Settings, next: &Settings) -> Result<(), String> {
    let before = table(previous);
    let after = table(next);
    let had = |s: &Shortcut| before.iter().any(|(_, b)| b.id() == s.id());
    let keeps = |s: &Shortcut| after.iter().any(|(_, a)| a.id() == s.id());
    let mut added = Vec::new();
    for (action, shortcut) in after.iter().filter(|(_, s)| !had(s)) {
        if registrar.register(*shortcut).is_err() {
            for undo in added {
                registrar.unregister(undo);
            }
            let text = if action == ISLAND { next.shortcut.clone() } else { next.shortcuts.actions[action].clone() };
            return Err(tr(Text::ShortcutTaken { shortcut: &text }));
        }
        added.push(*shortcut);
    }
    for (_, shortcut) in before.iter().filter(|(_, s)| !keeps(s)) {
        registrar.unregister(*shortcut);
    }
    bindings.replace(&after);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Windows as a test sees it: a few shortcuts belong to other apps.
    #[derive(Default)]
    struct FakeWindows {
        taken_elsewhere: Vec<u32>,
        ours: RefCell<Vec<u32>>,
    }

    impl Registrar for FakeWindows {
        fn register(&self, shortcut: Shortcut) -> Result<(), String> {
            let mut ours = self.ours.borrow_mut();
            if self.taken_elsewhere.contains(&shortcut.id()) || ours.contains(&shortcut.id()) {
                return Err("HotKey already registered".into());
            }
            ours.push(shortcut.id());
            Ok(())
        }

        fn unregister(&self, shortcut: Shortcut) {
            self.ours.borrow_mut().retain(|id| *id != shortcut.id());
        }
    }

    fn id(text: &str) -> u32 {
        parse(text).unwrap().id()
    }

    #[test]
    fn rules_refuse_lone_keys_the_windows_key_and_f12() {
        assert!(parse(DEFAULT_SHORTCUT).is_ok());
        assert!(parse("Ctrl+Alt+C").is_ok());
        assert!(parse("Alt+Shift+F9").is_ok());
        assert_eq!(parse("Ctrl+Banane").unwrap_err(), "Raccourci invalide : Ctrl+Banane");
        assert!(parse("Shift+C").unwrap_err().starts_with("Ajoute Ctrl ou Alt"));
        assert!(parse("F8").unwrap_err().starts_with("Ajoute Ctrl ou Alt"));
        assert_eq!(parse("Super+Alt+C").unwrap_err(), "La touche Windows est réservée à Windows.");
        assert_eq!(parse("Ctrl+F12").unwrap_err(), "F12 est réservée par Windows au débogage.");
        assert_eq!(id("Control+Alt+KeyC"), id("Ctrl+Alt+C"), "two spellings, one shortcut");
    }

    #[test]
    fn a_shortcut_used_twice_is_refused_whoever_holds_it() {
        let settings = assign(&Settings::default(), "open-chat", "Ctrl+Alt+C").unwrap();
        assert_eq!(settings.shortcuts.actions["open-chat"], "Ctrl+Alt+C");
        let twice = assign(&settings, "mute", "Control+Alt+KeyC").unwrap_err();
        assert_eq!(twice, "Ce raccourci sert déjà à une autre action d'îlot.");
        let island = assign(&settings, "mute", "Ctrl+Alt+Space").unwrap_err();
        assert_eq!(island, twice, "the island's shortcut counts too");
        assert!(assign(&settings, "open-chat", "Ctrl+Alt+C").is_ok(), "giving an action its own shortcut again is fine");
        let moved = assign(&settings, ISLAND, "Ctrl+Alt+C").unwrap_err();
        assert_eq!(moved, twice);
    }

    #[test]
    fn clearing_removes_an_action_but_never_the_island() {
        let settings = assign(&Settings::default(), "mute", "Ctrl+Alt+M").unwrap();
        let cleared = assign(&settings, "mute", "  ").unwrap();
        assert!(cleared.shortcuts.actions.is_empty());
        assert!(assign(&settings, ISLAND, "").unwrap_err().starts_with("L'île garde toujours"));
        assert_eq!(assign(&settings, "fly", "Ctrl+Alt+F").unwrap_err(), "Action inconnue : fly");
    }

    #[test]
    fn a_shortcut_taken_by_windows_keeps_the_old_one() {
        let windows = FakeWindows { taken_elsewhere: vec![id("Ctrl+Alt+T")], ..FakeWindows::default() };
        let bindings = Bindings::default();
        let before = assign(&Settings::default(), "go-to-alert", "Ctrl+Alt+A").unwrap();
        assert!(start(&windows, &bindings, &before).is_empty());
        let wanted = assign(&before, "go-to-alert", "Ctrl+Alt+T").unwrap();
        let refused = sync(&windows, &bindings, &before, &wanted).unwrap_err();
        assert_eq!(refused, "Ctrl+Alt+T est déjà pris par Windows ou une autre app. L'ancien raccourci reste.");
        assert!(windows.ours.borrow().contains(&id("Ctrl+Alt+A")), "the old shortcut still works");
        assert_eq!(bindings.action_of(id("Ctrl+Alt+A")).as_deref(), Some("go-to-alert"));
        assert_eq!(bindings.action_of(id("Ctrl+Alt+T")), None);
    }

    #[test]
    fn a_change_registers_the_new_shortcut_then_lets_the_old_one_go() {
        let windows = FakeWindows::default();
        let bindings = Bindings::default();
        let before = assign(&Settings::default(), "go-to-alert", "Ctrl+Alt+A").unwrap();
        start(&windows, &bindings, &before);
        let after = assign(&before, "go-to-alert", "Ctrl+Shift+A").unwrap();
        sync(&windows, &bindings, &before, &after).unwrap();
        let ours = windows.ours.borrow().clone();
        assert!(ours.contains(&id("Ctrl+Shift+A")) && !ours.contains(&id("Ctrl+Alt+A")));
        assert!(ours.contains(&id(DEFAULT_SHORTCUT)), "the island keeps its shortcut");
        assert_eq!(bindings.action_of(id("Ctrl+Shift+A")).as_deref(), Some("go-to-alert"));
        assert_eq!(bindings.action_of(id(DEFAULT_SHORTCUT)).as_deref(), Some(ISLAND));
        let island = assign(&after, ISLAND, "Ctrl+Alt+I").unwrap();
        sync(&windows, &bindings, &after, &island).unwrap();
        assert_eq!(bindings.action_of(id("Ctrl+Alt+I")).as_deref(), Some(ISLAND));
        assert!(!windows.ours.borrow().contains(&id(DEFAULT_SHORTCUT)));
    }

    #[test]
    fn a_shortcut_taken_at_launch_does_not_stop_the_others() {
        let windows = FakeWindows { taken_elsewhere: vec![id("Ctrl+Alt+M")], ..FakeWindows::default() };
        let bindings = Bindings::default();
        let settings = assign(&assign(&Settings::default(), "mute", "Ctrl+Alt+M").unwrap(), "go-to-alert", "Ctrl+Alt+A").unwrap();
        assert_eq!(start(&windows, &bindings, &settings), ["mute"]);
        assert_eq!(bindings.action_of(id("Ctrl+Alt+A")).as_deref(), Some("go-to-alert"));
        assert_eq!(bindings.action_of(id(DEFAULT_SHORTCUT)).as_deref(), Some(ISLAND));
    }

    #[test]
    fn saved_shortcuts_are_cleaned_on_load() {
        let saved = ShortcutSettings {
            actions: BTreeMap::from([
                ("go-to-alert".to_owned(), " Ctrl+Alt+A ".to_owned()),
                ("mute".to_owned(), "Ctrl+Alt+A".to_owned()),
                ("open-chat".to_owned(), "Ctrl+Alt+Space".to_owned()),
                ("fly".to_owned(), "Ctrl+Alt+F".to_owned()),
                ("open-wardrobe".to_owned(), "Win+W".to_owned()),
            ]),
        };
        let clean = saved.sanitized(DEFAULT_SHORTCUT);
        assert_eq!(clean.actions, BTreeMap::from([("go-to-alert".to_owned(), "Ctrl+Alt+A".to_owned())]));
    }

    #[test]
    fn rules_are_explained_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(parse("Ctrl+Banane").unwrap_err(), "Invalid shortcut: Ctrl+Banane");
        assert!(parse("Shift+C").unwrap_err().starts_with("Add Ctrl or Alt"));
        assert_eq!(parse("Super+Alt+C").unwrap_err(), "The Windows key is reserved for Windows.");
        assert_eq!(parse("Ctrl+F12").unwrap_err(), "Windows reserves F12 for debugging.");
        assert_eq!(assign(&Settings::default(), "nope", "Ctrl+Alt+N").unwrap_err(), "Unknown action: nope");
        assert_eq!(assign(&Settings::default(), ISLAND, "").unwrap_err(), "The island always keeps a shortcut: it's how it opens from the keyboard.");
    }
}
