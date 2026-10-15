//! User settings, kept as JSON in `%APPDATA%\ilot\settings.json`.

use crate::atomic::write_atomic;
use crate::notifications::NotificationSettings;
use crate::shortcuts::ShortcutSettings;
use crate::voice::VoiceSettings;
use crate::pastilles::config::PastilleSettings;
use crate::translate::TranslateSettings;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+Space";
pub const DEFAULT_MOCHI_NAME: &str = "Mochi";
/// The name shows in a small space: 14 characters at most (docs/03-DESIGN.md).
pub const MAX_MOCHI_NAME: usize = 14;
const PALETTES: [&str; 6] = ["lavande", "peche", "menthe", "ciel", "citron", "nuit"];
/// Mochi's three silhouettes (docs/09-DEMO-FINALE.md, section 6); « Mèche » by default.
const KINDS: [&str; 3] = ["meche", "ecran", "oreilles"];
/// The nine faces; the chosen one becomes Mochi's face at rest.
const FACES: [&str; 9] = ["idle", "blink", "happy", "wow", "worry", "sleep", "love", "star", "wink"];
/// The seven themes; each one rewrites the same design tokens (section 1).
pub const THEMES: [&str; 7] = ["fusion", "obsidienne", "neon", "terminal", "aurore", "bonbon", "chaos"];
/// Languages of the interface; the first one follows Windows.
pub const LANGUAGES: [&str; 3] = ["auto", "fr", "en"];

/// `value` when it is one of `allowed`, else the first allowed value.
fn one_of(value: String, allowed: &[&str]) -> String {
    if allowed.contains(&value.as_str()) { value } else { allowed[0].to_owned() }
}

/// Mochi's name, color, silhouette, face at rest and whether he moves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MochiSettings {
    pub name: String,
    pub palette: String,
    pub animated: bool,
    pub kind: String,
    pub face: String,
}

impl Default for MochiSettings {
    fn default() -> Self {
        Self {
            name: DEFAULT_MOCHI_NAME.to_owned(),
            palette: PALETTES[0].to_owned(),
            animated: true,
            kind: KINDS[0].to_owned(),
            face: FACES[0].to_owned(),
        }
    }
}

impl MochiSettings {
    /// Trims the name to its limit (a blank name gives "Mochi") and rejects unknown
    /// colors, silhouettes and faces.
    pub fn sanitized(self) -> Self {
        let name: String = self.name.trim().chars().filter(|c| !c.is_control()).take(MAX_MOCHI_NAME).collect();
        Self {
            name: if name.trim().is_empty() { DEFAULT_MOCHI_NAME.to_owned() } else { name.trim().to_owned() },
            palette: one_of(self.palette, &PALETTES),
            animated: self.animated,
            kind: one_of(self.kind, &KINDS),
            face: one_of(self.face, &FACES),
        }
    }
}

/// Display modes of the « Modes » tab (docs/09-DEMO-FINALE.md, section 7); the first
/// value of each list is the default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Modes {
    /// Music in the folded island.
    pub music: String,
    /// Agents in the pill.
    pub agents: String,
    pub messages: String,
    /// Finished task.
    pub done: String,
    /// Mochi out of the island.
    pub mochi: String,
    pub relief: String,
    pub position: String,
}

const MODE_MUSIC: [&str; 4] = ["titre", "disque", "egaliseur", "aucun"];
const MODE_AGENTS: [&str; 3] = ["compteur", "points", "activite"];
const MODE_MESSAGES: [&str; 3] = ["apercu", "expediteur", "discret"];
const MODE_DONE: [&str; 2] = ["banniere", "discret"];
/// « Sur le bureau » (prompt 12, lot 1) comes last: the first value stays the default.
const MODE_MOCHI: [&str; 4] = ["balade", "suit", "reste", "bureau"];
const MODE_RELIEF: [&str; 3] = ["3d", "leger", "plat"];
const MODE_POSITION: [&str; 2] = ["libre", "aimant"];

impl Default for Modes {
    fn default() -> Self {
        Self {
            music: MODE_MUSIC[0].to_owned(),
            agents: MODE_AGENTS[0].to_owned(),
            messages: MODE_MESSAGES[0].to_owned(),
            done: MODE_DONE[0].to_owned(),
            mochi: MODE_MOCHI[0].to_owned(),
            relief: MODE_RELIEF[0].to_owned(),
            position: MODE_POSITION[0].to_owned(),
        }
    }
}

impl Modes {
    /// Unknown values fall back to each mode's default.
    pub fn sanitized(self) -> Self {
        Self {
            music: one_of(self.music, &MODE_MUSIC),
            agents: one_of(self.agents, &MODE_AGENTS),
            messages: one_of(self.messages, &MODE_MESSAGES),
            done: one_of(self.done, &MODE_DONE),
            mochi: one_of(self.mochi, &MODE_MOCHI),
            relief: one_of(self.relief, &MODE_RELIEF),
            position: one_of(self.position, &MODE_POSITION),
        }
    }
}

/// The chat's providers: three on the Internet with the user's key, two on this PC.
pub const CHAT_PROVIDERS: [&str; 5] = ["anthropic", "gemini", "openai", "ollama", "lmstudio"];
/// A model id as the providers write them (`claude-…`, `gemini-…`, `llama3.2:3b`,
/// `google/gemma-3-12b`): at most this long.
const MAX_MODEL_CHARS: usize = 128;

/// The chat (prompt 12, lot 3): the provider and the model it opens on. The keys live in
/// the vault (vault.rs), never here; an empty model means « the provider's first one ».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatSettings {
    pub provider: String,
    pub model: String,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self { provider: CHAT_PROVIDERS[0].to_owned(), model: String::new() }
    }
}

/// True for a model id made of letters, digits and `.`, `_`, `-`, `:`, `/`, `@`.
pub fn valid_model(model: &str) -> bool {
    !model.is_empty()
        && model.chars().count() <= MAX_MODEL_CHARS
        && model.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '/' | '@'))
}

impl ChatSettings {
    /// An unknown provider falls back to Claude; a model id that looks wrong is dropped.
    pub fn sanitized(self) -> Self {
        let model = self.model.trim().to_owned();
        Self { provider: one_of(self.provider, &CHAT_PROVIDERS), model: if valid_model(&model) { model } else { String::new() } }
    }
}

/// Where the folded island rests: the screen (its device name) and the center of the
/// pill as a share of that screen's work area, so that it survives a new resolution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IslandPosition {
    pub monitor: String,
    pub x: f64,
    pub y: f64,
}

/// Top-left corner of a window, in physical pixels of the virtual screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

/// îlot's own sounds (prompt 12, lot 1): one volume for all of them and a switch that
/// cuts them all. Which sound plays when is decided by the interface (src/lib/sound).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SoundSettings {
    /// From 0 (silent) to 1.
    pub volume: f64,
    pub muted: bool,
}

pub const DEFAULT_SOUND_VOLUME: f64 = 0.5;

impl Default for SoundSettings {
    fn default() -> Self {
        Self { volume: DEFAULT_SOUND_VOLUME, muted: false }
    }
}

impl SoundSettings {
    /// A volume outside 0 to 1 is brought back within; a damaged one gives the default.
    pub fn sanitized(self) -> Self {
        let volume = if self.volume.is_finite() { self.volume.clamp(0.0, 1.0) } else { DEFAULT_SOUND_VOLUME };
        Self { volume, ..self }
    }
}

/// What Mochi wears, one piece per slot, and whether the season's outfit fills the free
/// slots (src/lib/mochi/outfits.ts draws them). "" leaves a slot free, "aucun" keeps it
/// bare even in season.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Wardrobe {
    pub head: String,
    pub eyes: String,
    pub neck: String,
    pub ears: String,
    pub seasonal: bool,
}

impl Default for Wardrobe {
    fn default() -> Self {
        Self { head: String::new(), eyes: String::new(), neck: String::new(), ears: String::new(), seasonal: true }
    }
}

const HEAD_PIECES: [&str; 9] = ["", "aucun", "bonnet", "noel", "casquette", "sorciere", "couronne", "fete", "fleur"];
const EYE_PIECES: [&str; 4] = ["", "aucun", "rondes", "soleil"];
const NECK_PIECES: [&str; 4] = ["", "aucun", "echarpe", "noeud"];
const EAR_PIECES: [&str; 4] = ["", "aucun", "casque", "cacheoreilles"];

impl Wardrobe {
    /// A piece that does not exist, or that goes elsewhere, leaves its slot empty.
    pub fn sanitized(self) -> Self {
        Self {
            head: one_of(self.head, &HEAD_PIECES),
            eyes: one_of(self.eyes, &EYE_PIECES),
            neck: one_of(self.neck, &NECK_PIECES),
            ears: one_of(self.ears, &EAR_PIECES),
            seasonal: self.seasonal,
        }
    }
}

/// Which screen holds the island and when it hides (Réglages › Général).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplaySettings {
    /// The island moves to the screen the cursor is on, at the same place on it.
    pub follow_mouse: bool,
    /// The island turns see-through, in place, while nothing goes on, and comes back on
    /// hover or when something happens. Files written before it was renamed said
    /// `hideWhenIdle`.
    #[serde(alias = "hideWhenIdle")]
    pub fade_when_idle: bool,
}

/// Where Mochi was left on the desktop (« Sur le bureau »): the screen and his center as
/// a share of its work area. Owned by the core, like the island's place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskPlace {
    pub monitor: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// One of `THEMES`; an older « dark » or « light » falls back to « fusion ».
    pub theme: String,
    pub modes: Modes,
    pub hide_in_fullscreen: bool,
    pub do_not_disturb: bool,
    pub shortcut: String,
    /// Where the user left the mini-player; owned by the core, not by the interface.
    pub mini_player: Option<WindowPosition>,
    /// Where the user left the YouTube player window.
    pub video_player: Option<WindowPosition>,
    /// Where the user left the island; owned by the core like the player windows.
    pub island: Option<IslandPosition>,
    pub notifications: NotificationSettings,
    pub mochi: MochiSettings,
    /// Banner when an agent finishes ("Claude a terminé · …").
    pub announce_finished: bool,
    /// Start îlot when the user signs in to Windows (off by default).
    pub launch_at_startup: bool,
    pub sound: SoundSettings,
    pub wardrobe: Wardrobe,
    pub display: DisplaySettings,
    /// Where Mochi rests on the desktop while he is out « Sur le bureau ».
    pub desk: Option<DeskPlace>,
    /// Shortcuts of the named actions; the island's own stays in `shortcut`.
    pub shortcuts: ShortcutSettings,
    /// « OK Mochi » (off by default).
    pub voice: VoiceSettings,
    pub chat: ChatSettings,
    /// Réglages › Pastilles: the main tool and up to four more (none by default).
    pub pastilles: PastilleSettings,
    /// Language of the interface: "auto" follows Windows, else "fr" or "en".
    pub language: String,
    /// DeepL's target language (Réglages › Chat); the key lives in the vault, never here.
    pub translate: TranslateSettings,
}

impl Settings {
    /// Settings sent by the interface never move the player windows: their positions
    /// come from the core, which saw the last move.
    pub fn with_positions_of(self, current: &Settings) -> Settings {
        Settings {
            mini_player: current.mini_player,
            video_player: current.video_player,
            island: current.island.clone(),
            desk: current.desk.clone(),
            ..self
        }
    }

    /// Shortcuts change only through `set_shortcut`, which registers them with Windows
    /// first: settings saved by the interface keep the ones the core holds.
    pub fn with_shortcuts_of(self, current: &Settings) -> Settings {
        Settings { shortcut: current.shortcut.clone(), shortcuts: current.shortcuts.clone(), ..self }
    }

    /// Unknown themes and modes fall back to their defaults; Mochi stays within bounds;
    /// shortcuts that break the rules or come twice are dropped.
    pub fn sanitized(self) -> Settings {
        Settings {
            theme: one_of(self.theme, &THEMES),
            modes: self.modes.sanitized(),
            mochi: self.mochi.sanitized(),
            sound: self.sound.sanitized(),
            wardrobe: self.wardrobe.sanitized(),
            shortcuts: self.shortcuts.sanitized(&self.shortcut),
            chat: self.chat.sanitized(),
            pastilles: self.pastilles.sanitized(),
            language: one_of(self.language, &LANGUAGES),
            notifications: self.notifications.localized(),
            translate: self.translate.sanitized(),
            ..self
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: THEMES[0].to_owned(),
            modes: Modes::default(),
            hide_in_fullscreen: true,
            do_not_disturb: false,
            shortcut: DEFAULT_SHORTCUT.to_owned(),
            mini_player: None,
            video_player: None,
            island: None,
            notifications: NotificationSettings::default(),
            mochi: MochiSettings::default(),
            announce_finished: true,
            launch_at_startup: false,
            sound: SoundSettings::default(),
            wardrobe: Wardrobe::default(),
            display: DisplaySettings::default(),
            desk: None,
            shortcuts: ShortcutSettings::default(),
            voice: VoiceSettings::default(),
            chat: ChatSettings::default(),
            pastilles: PastilleSettings::default(),
            language: LANGUAGES[0].to_owned(),
            translate: TranslateSettings::default(),
        }
    }
}

pub fn settings_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("ilot").join("settings.json"))
}

/// Reads the settings; a missing or unreadable file gives the defaults.
pub fn load(path: &Path) -> Settings {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
        .unwrap_or_default()
        .sanitized()
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    write_atomic(path, &json)
}

/// Current settings shared by the whole app, saved on every change.
pub struct SettingsStore {
    path: Option<PathBuf>,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn open(path: Option<PathBuf>) -> Self {
        let current = path.as_deref().map(load).unwrap_or_default();
        Self { path, current: Mutex::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn replace(&self, settings: Settings) -> io::Result<()> {
        if let Some(path) = &self.path {
            save(path, &settings)?;
        }
        *self.current.lock().unwrap() = settings;
        Ok(())
    }

    /// Changes the settings while holding the lock, so two updates never undo each other.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> io::Result<Settings> {
        let mut current = self.current.lock().unwrap();
        let mut next = current.clone();
        change(&mut next);
        if let Some(path) = &self.path {
            save(path, &next)?;
        }
        *current = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mochi_name_and_color_are_kept_within_bounds() {
        let long = MochiSettings {
            name: "  Mochi-le-très-grand-ami  ".into(),
            palette: "rose".into(),
            kind: "fruit".into(),
            face: "angry".into(),
            ..MochiSettings::default()
        };
        let clean = long.sanitized();
        assert_eq!(clean.name.chars().count(), MAX_MOCHI_NAME);
        assert_eq!((clean.palette.as_str(), clean.kind.as_str(), clean.face.as_str()), ("lavande", "meche", "idle"));
        let blank = MochiSettings {
            name: " \u{7} ".into(),
            palette: "ciel".into(),
            animated: false,
            kind: "oreilles".into(),
            face: "star".into(),
        }
        .sanitized();
        assert_eq!((blank.name.as_str(), blank.palette.as_str(), blank.animated), ("Mochi", "ciel", false));
        assert_eq!((blank.kind.as_str(), blank.face.as_str()), ("oreilles", "star"));
    }

    #[test]
    fn older_themes_and_unknown_modes_fall_back_to_the_defaults() {
        let parsed: Settings = serde_json::from_str(r#"{"theme":"light","modes":{"music":"disque","relief":"wow"}}"#).unwrap();
        let clean = parsed.sanitized();
        assert_eq!(clean.theme, "fusion");
        assert_eq!((clean.modes.music.as_str(), clean.modes.relief.as_str()), ("disque", "3d"));
        assert_eq!(clean.modes.position, "libre");
        let neon: Settings = serde_json::from_str(r#"{"theme":"neon"}"#).unwrap();
        assert_eq!(neon.sanitized().theme, "neon");
    }

    #[test]
    fn the_chat_keeps_a_known_provider_and_a_plain_model_id() {
        let older: Settings = serde_json::from_str(r#"{"theme":"fusion"}"#).unwrap();
        assert_eq!(older.chat, ChatSettings::default());
        assert_eq!(older.chat.provider, "anthropic");
        for model in ["claude-sonnet-4-5", "gemini-2.5-flash", "llama3.2:3b", "google/gemma-3-12b", "qwen@q4_k_m"] {
            let chat = ChatSettings { provider: "lmstudio".into(), model: format!(" {model} ") }.sanitized();
            assert_eq!((chat.provider.as_str(), chat.model.as_str()), ("lmstudio", model));
        }
        for model in ["", "a b", "modèle", "x\"y", "a\nb", &"m".repeat(129)] {
            assert_eq!(ChatSettings { provider: "openai".into(), model: model.into() }.sanitized().model, "", "{model}");
        }
        assert_eq!(ChatSettings { provider: "evil".into(), model: String::new() }.sanitized().provider, "anthropic");
    }

    #[test]
    fn missing_file_gives_defaults() {
        let path = std::env::temp_dir().join("ilot-settings-missing").join("settings.json");
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn unknown_and_missing_fields_fall_back_to_defaults() {
        let parsed: Settings =
            serde_json::from_str(r#"{"theme":"aurore","futureOption":42}"#).unwrap();
        assert_eq!(parsed.theme, "aurore");
        assert!(parsed.hide_in_fullscreen);
        assert_eq!(parsed.shortcut, DEFAULT_SHORTCUT);
        assert_eq!(parsed.notifications, NotificationSettings::default());
    }

    #[test]
    fn invalid_json_gives_defaults() {
        let dir = std::env::temp_dir().join(format!("ilot-settings-bad-{}", std::process::id()));
        let path = dir.join("settings.json");
        write_atomic(&path, b"{ not json").unwrap();
        assert_eq!(load(&path), Settings::default());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn store_saves_every_change() {
        let dir = std::env::temp_dir().join(format!("ilot-settings-store-{}", std::process::id()));
        let path = dir.join("settings.json");
        let store = SettingsStore::open(Some(path.clone()));
        assert_eq!(store.get(), Settings::default());
        let quiet = Settings { do_not_disturb: true, ..Settings::default() };
        store.replace(quiet.clone()).unwrap();
        assert_eq!(SettingsStore::open(Some(path)).get(), quiet);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn update_changes_and_saves_in_one_step() {
        let dir = std::env::temp_dir().join(format!("ilot-settings-update-{}", std::process::id()));
        let path = dir.join("settings.json");
        let store = SettingsStore::open(Some(path.clone()));
        let saved = store.update(|s| s.mini_player = Some(WindowPosition { x: 5, y: 6 })).unwrap();
        assert_eq!(saved.mini_player, Some(WindowPosition { x: 5, y: 6 }));
        assert_eq!(SettingsStore::open(Some(path)).get(), saved);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn interface_settings_keep_the_window_positions_of_the_core() {
        let island = IslandPosition { monitor: r"\\.\DISPLAY1".into(), x: 0.5, y: 0.04 };
        let desk = DeskPlace { monitor: "DISPLAY1".into(), x: 0.7, y: 0.6 };
        let core = Settings {
            mini_player: Some(WindowPosition { x: 10, y: 20 }),
            island: Some(island.clone()),
            desk: Some(desk.clone()),
            ..Settings::default()
        };
        let from_interface = Settings { theme: "bonbon".into(), ..Settings::default() };
        let merged = from_interface.with_positions_of(&core);
        assert_eq!(merged.theme, "bonbon");
        assert_eq!(merged.mini_player, Some(WindowPosition { x: 10, y: 20 }));
        assert_eq!(merged.island, Some(island));
        assert_eq!(merged.desk, Some(desk), "Mochi's place on the desktop belongs to the core");
    }

    #[test]
    fn positions_are_saved_in_camel_case() {
        let settings = Settings { video_player: Some(WindowPosition { x: 1, y: 2 }), ..Settings::default() };
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["videoPlayer"]["x"], 1);
        assert!(json["miniPlayer"].is_null());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("ilot-settings-rt-{}", std::process::id()));
        let path = dir.join("settings.json");
        let settings = Settings {
            theme: "terminal".into(),
            modes: Modes { music: "egaliseur".into(), ..Modes::default() },
            hide_in_fullscreen: false,
            do_not_disturb: true,
            shortcut: "Ctrl+Shift+I".to_owned(),
            mini_player: Some(WindowPosition { x: -1600, y: 820 }),
            video_player: None,
            island: Some(IslandPosition { monitor: "DISPLAY2".into(), x: 0.25, y: 0.9 }),
            notifications: NotificationSettings {
                enabled: true,
                allowed: vec!["discord".to_owned()],
                ..NotificationSettings::default()
            },
            mochi: MochiSettings {
                name: "Pixel".into(),
                palette: "nuit".into(),
                animated: false,
                kind: "ecran".into(),
                face: "wink".into(),
            },
            announce_finished: false,
            launch_at_startup: true,
            sound: SoundSettings { volume: 0.25, muted: true },
            wardrobe: Wardrobe { head: "sorciere".into(), eyes: "rondes".into(), seasonal: false, ..Wardrobe::default() },
            display: DisplaySettings { follow_mouse: true, fade_when_idle: true },
            desk: Some(DeskPlace { monitor: "DISPLAY2".into(), x: 0.3, y: 0.8 }),
            shortcuts: ShortcutSettings {
                actions: [("go-to-alert".to_owned(), "Ctrl+Alt+A".to_owned()), ("mute".to_owned(), "Ctrl+Shift+M".to_owned())].into(),
            },
            voice: VoiceSettings { listen: true, speak: false },
            chat: ChatSettings { provider: "ollama".into(), model: "llama3.2:3b".into() },
            pastilles: PastilleSettings {
                tool: "vscode".into(),
                extras: vec![crate::pastilles::config::Extra { id: "ollama".into(), ..Default::default() }],
            },
            language: "en".into(),
            translate: TranslateSettings { target: "PT-BR".into() },
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), settings);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn older_files_get_the_new_settings_with_their_defaults() {
        let parsed: Settings = serde_json::from_str(r#"{"theme":"neon","modes":{"mochi":"suit"}}"#).unwrap();
        assert_eq!(parsed.sound, SoundSettings { volume: DEFAULT_SOUND_VOLUME, muted: false });
        assert_eq!(parsed.wardrobe, Wardrobe::default());
        assert!(parsed.wardrobe.seasonal, "season outfits are on until turned off");
        assert_eq!(parsed.display, DisplaySettings { follow_mouse: false, fade_when_idle: false });
        let renamed: Settings = serde_json::from_str(r#"{"display":{"hideWhenIdle":true}}"#).unwrap();
        assert!(renamed.display.fade_when_idle, "the old name still reads");
        assert_eq!(parsed.desk, None);
    }

    #[test]
    fn the_desk_mode_is_a_known_mode() {
        let parsed: Settings = serde_json::from_str(r#"{"modes":{"mochi":"bureau"}}"#).unwrap();
        assert_eq!(parsed.sanitized().modes.mochi, "bureau");
        let odd: Settings = serde_json::from_str(r#"{"modes":{"mochi":"plafond"}}"#).unwrap();
        assert_eq!(odd.sanitized().modes.mochi, "balade");
    }

    #[test]
    fn the_volume_stays_between_silent_and_full() {
        assert_eq!(SoundSettings { volume: 3.0, muted: false }.sanitized().volume, 1.0);
        assert_eq!(SoundSettings { volume: -1.0, muted: true }.sanitized(), SoundSettings { volume: 0.0, muted: true });
        assert_eq!(SoundSettings { volume: f64::NAN, muted: false }.sanitized().volume, DEFAULT_SOUND_VOLUME);
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["sound"], serde_json::json!({ "volume": 0.5, "muted": false }));
        assert_eq!(json["display"], serde_json::json!({ "followMouse": false, "fadeWhenIdle": false }));
    }

    #[test]
    fn a_piece_only_fits_its_own_slot() {
        let odd = Wardrobe {
            head: "soleil".into(),
            eyes: "rondes".into(),
            neck: "<script>".into(),
            ears: "casque".into(),
            seasonal: false,
        };
        let clean = odd.sanitized();
        assert_eq!((clean.head.as_str(), clean.eyes.as_str()), ("", "rondes"));
        assert_eq!((clean.neck.as_str(), clean.ears.as_str()), ("", "casque"));
        assert!(!clean.seasonal);
        for head in HEAD_PIECES {
            let worn = Wardrobe { head: head.into(), ..Wardrobe::default() }.sanitized();
            assert_eq!(worn.head, head);
        }
    }

    #[test]
    fn shortcuts_and_voice_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("ilot-settings-restart-{}", std::process::id()));
        let path = dir.join("settings.json");
        let store = SettingsStore::open(Some(path.clone()));
        let with_shortcut = crate::shortcuts::assign(&store.get(), "go-to-alert", "Ctrl+Alt+A").unwrap();
        store.replace(Settings { voice: VoiceSettings { listen: true, speak: true }, ..with_shortcut }).unwrap();
        // The file as it is on disk, read back by a new îlot.
        let written: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["shortcuts"]["actions"]["go-to-alert"], "Ctrl+Alt+A");
        assert_eq!(written["voice"]["listen"], true);
        let reopened = SettingsStore::open(Some(path)).get();
        assert_eq!(reopened.shortcuts.actions["go-to-alert"], "Ctrl+Alt+A");
        assert!(reopened.voice.listen);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_language_follows_windows_until_one_is_chosen() {
        let older: Settings = serde_json::from_str(r#"{"theme":"aurore"}"#).unwrap();
        assert_eq!(older.language, "auto");
        let english: Settings = serde_json::from_str(r#"{"language":"en"}"#).unwrap();
        assert_eq!(english.sanitized().language, "en");
        let odd: Settings = serde_json::from_str(r#"{"language":"klingon"}"#).unwrap();
        assert_eq!(odd.sanitized().language, "auto");
    }

    #[test]
    fn older_files_get_no_shortcut_and_no_listening() {
        let older: Settings = serde_json::from_str(r#"{"theme":"aurore","shortcut":"Ctrl+Shift+I"}"#).unwrap();
        assert!(older.shortcuts.actions.is_empty());
        assert!(!older.voice.listen, "« OK Mochi » stays off until the user turns it on");
    }

    #[test]
    fn interface_settings_keep_the_shortcuts_of_the_core() {
        let core = crate::shortcuts::assign(&Settings::default(), "mute", "Ctrl+Alt+M").unwrap();
        let from_interface = Settings { theme: "neon".into(), shortcut: "Ctrl+Alt+X".into(), ..Settings::default() };
        let merged = from_interface.with_shortcuts_of(&core);
        assert_eq!((merged.theme.as_str(), merged.shortcut.as_str()), ("neon", DEFAULT_SHORTCUT));
        assert_eq!(merged.shortcuts, core.shortcuts);
    }
}
