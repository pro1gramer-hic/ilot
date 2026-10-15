//! « OK Mochi » (prompt 12, lot 5), off by default. On, the microphone listens on the PC
//! for the keyword and a short list of phrases (listen.rs); the interface turns what it
//! heard into an action of its own, waits for a click before anything leaves the PC, and
//! may send the words and a few counters to Claude with the user's key (claude.rs).
//! Mochi answers aloud with a voice of Windows in the language of the interface
//! (speak.rs). No audio is ever recorded, kept or sent.

mod claude;
mod listen;
mod speak;

use crate::events::now_ms;
use crate::locale::{self, Lang, Text};
use crate::settings::SettingsStore;
use crate::window::WINDOW_LABEL;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

pub const EVENT_HEARD: &str = "voice:heard";
pub const EVENT_STATE: &str = "voice:state";
/// What the microphone hears this long after Mochi spoke is taken for his own voice.
pub const QUIET_AFTER_MS: u64 = 600;
/// The grammar's limits: a few keywords, a short list of short phrases.
const MAX_KEYWORDS: usize = 4;
const MAX_PHRASES: usize = 60;
const MAX_PHRASE_CHARS: usize = 48;

/// « OK Mochi » in the settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VoiceSettings {
    /// The microphone listens for « OK Mochi » (off by default; a dot on Mochi shows it).
    pub listen: bool,
    /// Mochi answers aloud.
    pub speak: bool,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self { listen: false, speak: true }
    }
}

pub(crate) use listen::speech_token;

/// The words the recognizer may hear, as the interface lists them, and their language
/// (French when the interface does not say).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Grammar {
    /// « OK Mochi », and « OK » with Mochi's own name.
    pub wake: Vec<String>,
    pub phrases: Vec<String>,
    pub lang: Lang,
}

fn plain_words(text: &str) -> bool {
    let count = text.chars().count();
    (2..=MAX_PHRASE_CHARS).contains(&count)
        && text.chars().all(|c| c.is_alphabetic() || c == ' ' || c == '\'' || c == '’' || c == '-')
        && text.chars().any(char::is_alphabetic)
}

impl Grammar {
    /// A grammar of words only, within limits: the recognizer never gets anything else.
    pub fn checked(self) -> Result<Grammar, String> {
        let fits = (1..=MAX_KEYWORDS).contains(&self.wake.len())
            && self.phrases.len() <= MAX_PHRASES
            && self.wake.iter().chain(&self.phrases).all(|p| plain_words(p));
        if fits { Ok(self) } else { Err(Text::VoicePhrasesRefused.into()) }
    }
}

/// True while Mochi speaks (and a moment after).
pub fn quiet_now(quiet_until: &AtomicU64) -> bool {
    now_ms() < quiet_until.load(Ordering::Relaxed)
}

#[derive(Default)]
pub struct Voice {
    listener: Mutex<Option<listen::Listener>>,
    speaker: speak::Speaker,
    quiet_until: Arc<AtomicU64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeardView {
    wake: bool,
    phrase: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StateView {
    listening: bool,
}

/// The user's Claude key, the one saved in Réglages › Chat (the vault, never the
/// interface). None when there is none: Mochi says so and nothing is sent.
fn claude_key() -> Option<String> {
    crate::vault::read("chat:anthropic").ok().flatten()
}

/// Opens or closes the microphone. It only opens while « OK Mochi » is on in the
/// settings, whatever the interface asks. The engine takes a moment to start: off the
/// interface's thread.
#[tauri::command]
pub async fn voice_listen(on: bool, grammar: Option<Grammar>, app: AppHandle) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || listen_now(&app, on, grammar)).await.map_err(|e| e.to_string())?
}

fn listen_now(app: &AppHandle, on: bool, grammar: Option<Grammar>) -> Result<bool, String> {
    let voice = app.state::<Voice>();
    let mut listener = voice.listener.lock().unwrap();
    *listener = None;
    let allowed = app.state::<SettingsStore>().get().voice.listen;
    let result = if on && allowed {
        let grammar = grammar.unwrap_or_default().checked()?;
        let window = app.clone();
        let started = listen::Listener::start(grammar, voice.quiet_until.clone(), move |heard| {
            let view = HeardView { wake: heard.wake, phrase: heard.phrase };
            let _ = window.emit_to(WINDOW_LABEL, EVENT_HEARD, view);
        });
        started.map(|l| *listener = Some(l))
    } else {
        Ok(())
    };
    let listening = listener.is_some();
    let _ = app.emit_to(WINDOW_LABEL, EVENT_STATE, StateView { listening });
    result.map(|()| listening)
}

/// « OK Mochi » turned off in the settings: the microphone closes at once, whatever the
/// interface does next.
pub fn settings_changed(app: &AppHandle, settings: &VoiceSettings) {
    if settings.listen {
        return;
    }
    let voice = app.state::<Voice>();
    let mut listener = voice.listener.lock().unwrap();
    if listener.take().is_some() {
        let _ = app.emit_to(WINDOW_LABEL, EVENT_STATE, StateView { listening: false });
    }
}

/// Mochi says `text` aloud (when « Répondre à voix haute » is on), with a voice of the
/// interface's language.
#[tauri::command]
pub fn voice_say(text: String, app: AppHandle, voice: State<'_, Voice>) -> Result<(), String> {
    if !app.state::<SettingsStore>().get().voice.speak {
        return Ok(());
    }
    voice.speaker.say(&text, locale::current(), &voice.quiet_until)
}

#[tauri::command]
pub fn voice_hush(voice: State<'_, Voice>) {
    voice.speaker.hush(&voice.quiet_until);
}

/// Sends the question the user confirmed with a click to Claude; the answer or why there
/// is none, in the language of the interface.
#[tauri::command]
pub async fn voice_ask(question: String, context: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        claude::ask(&claude::WindowsHttp, claude_key().as_deref(), &question, &context).map_err(|e| e.message())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_mochi_is_off_by_default_and_mochi_speaks_once_on() {
        assert_eq!(VoiceSettings::default(), VoiceSettings { listen: false, speak: true });
        let older: VoiceSettings = serde_json::from_str("{}").unwrap();
        assert!(!older.listen);
    }

    #[test]
    fn the_grammar_holds_plain_words_only() {
        let words = |wake: &[&str], phrases: &[&str]| Grammar {
            wake: wake.iter().map(|s| (*s).to_owned()).collect(),
            phrases: phrases.iter().map(|s| (*s).to_owned()).collect(),
            lang: Lang::Fr,
        };
        assert!(words(&["OK Mochi"], &["coupe le son", "ouvre l'île", "va au terminal", "résume l’activité"]).checked().is_ok());
        assert!(words(&[], &["coupe le son"]).checked().is_err(), "a keyword is needed");
        assert!(words(&["OK Mochi"], &["<rule>"]).checked().is_err());
        assert!(words(&["OK Mochi"], &["a"]).checked().is_err());
        assert!(words(&["OK Mochi"], &[&"mot ".repeat(20)]).checked().is_err());
        let many: Vec<String> = (0..=MAX_PHRASES).map(|_| "ouvre le chat".to_owned()).collect();
        assert!(Grammar { wake: vec!["OK Mochi".into()], phrases: many, lang: Lang::Fr }.checked().is_err());
    }

    #[test]
    fn the_grammar_says_its_language_french_when_it_does_not() {
        let older: Grammar = serde_json::from_str(r#"{"wake":["OK Mochi"],"phrases":["coupe le son"]}"#).unwrap();
        assert_eq!(older.lang, Lang::Fr);
        let english: Grammar = serde_json::from_str(r#"{"wake":["OK Mochi"],"phrases":["mute"],"lang":"en"}"#).unwrap();
        assert_eq!(english.lang, Lang::En);
        assert!(serde_json::from_str::<Grammar>(r#"{"wake":["OK Mochi"],"lang":"klingon"}"#).is_err());
        crate::locale::speak(Lang::En);
        let refused = Grammar { wake: vec![], ..english }.checked().unwrap_err();
        assert_eq!(refused, "Phrase list refused.");
    }

    #[test]
    fn mochi_is_quiet_only_while_he_speaks() {
        let quiet = AtomicU64::new(0);
        assert!(!quiet_now(&quiet));
        quiet.store(now_ms() + 10_000, Ordering::Relaxed);
        assert!(quiet_now(&quiet));
    }

    /// The real capabilities, compiled by build.rs: the microphone, Claude, the shortcuts,
    /// the report and its file stay out of reach of every window but the island.
    mod access {
        use crate::window::WINDOW_LABEL;
        use tauri::ipc::{CallbackFn, InvokeBody};
        use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
        use tauri::webview::InvokeRequest;
        use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

        const LOCAL: &str = "http://tauri.localhost";
        const COMMANDS: [&str; 9] = [
            "voice_listen",
            "voice_say",
            "voice_hush",
            "voice_ask",
            "set_shortcut",
            "week_report",
            "take_monday_report",
            "clear_stats",
            "save_report_image",
        ];

        fn app() -> App<MockRuntime> {
            mock_builder()
                .manage(super::Voice::default())
                .invoke_handler(tauri::generate_handler![super::voice_hush])
                .build(tauri::generate_context!())
                .unwrap()
        }

        fn allowed(window: &WebviewWindow<MockRuntime>, cmd: &str) -> bool {
            let request = InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: LOCAL.parse().unwrap(),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            match get_ipc_response(window, request) {
                Ok(_) => true,
                Err(error) => {
                    assert!(error.to_string().contains("not allowed"), "{cmd}: {error}");
                    false
                }
            }
        }

        #[test]
        fn only_the_island_reaches_the_voice_the_shortcuts_and_the_report() {
            let app = app();
            let island = WebviewWindowBuilder::new(&app, WINDOW_LABEL, WebviewUrl::default()).build().unwrap();
            assert!(allowed(&island, "voice_hush"));
            for label in [crate::buddy::BUDDY_LABEL, crate::players::MINI_LABEL, crate::video::YOUTUBE_BAR] {
                let other = WebviewWindowBuilder::new(&app, label, WebviewUrl::default()).build().unwrap();
                for cmd in COMMANDS {
                    assert!(!allowed(&other, cmd), "{cmd} from {label}");
                }
            }
        }
    }
}
