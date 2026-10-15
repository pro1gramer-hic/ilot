//! The language the core speaks to the user (prompt 12, lot 5): the one chosen in
//! Réglages › Général, or Windows' own with « auto », by the same rule as the interface
//! (src/lib/i18n/locale.ts). Texts are named in text.rs and said by each language in its
//! own file; adding a language = its file, its `Lang` and its line in `text_in`.
//! Texts sent to an agent (a refusal's reason, a hook's answer) are not the interface:
//! they stay as they are, whatever the language.

mod en;
mod fr;
mod text;

pub use text::Text;

use serde::Deserialize;
use std::sync::atomic::{AtomicU8, Ordering};

/// A language îlot speaks, by its code (`"fr"`, `"en"`); French when nothing says which.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Fr,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Fr, Lang::En];

    /// The code the settings and the interface use.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|lang| lang.code() == code)
    }

    /// The Windows language ids (LCID) of the speech engines and voices of this language,
    /// as SAPI names them in its tokens (`Language=409`), the most common first.
    pub fn speech_lcids(self) -> &'static [u16] {
        match self {
            // France only, as before (« Microsoft Speech Recognizer 8.0 for Windows (French) »).
            Lang::Fr => &[0x040C],
            // United States (MS-1033-…), United Kingdom, Australia, Canada, India.
            Lang::En => &[0x0409, 0x0809, 0x0C09, 0x1009, 0x4009],
        }
    }
}

/// `language` when chosen; with « auto », the first of Windows' display languages îlot
/// speaks, English when it speaks none of them.
pub fn resolve(language: &str, system: &[String]) -> Lang {
    let codes = Lang::ALL.map(Lang::code);
    Lang::from_code(ilot_bridge::lang::resolve(language, system, &codes, Lang::En.code())).unwrap_or(Lang::En)
}

/// The language of the moment, for every thread of the app.
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Follows the settings: the texts made from now on are in this language.
pub fn set(language: &str) -> Lang {
    let lang = match Lang::from_code(language) {
        Some(lang) => lang,
        None => resolve(language, &ilot_bridge::lang::system_languages()),
    };
    CURRENT.store(Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0) as u8, Ordering::Relaxed);
    lang
}

#[cfg(test)]
thread_local! {
    /// In the tests, each test thread speaks French unless it asks for another language:
    /// tests run side by side and never hear each other's choice.
    static SPOKEN: std::cell::Cell<Lang> = const { std::cell::Cell::new(Lang::Fr) };
}

/// The language the core speaks now.
#[cfg(not(test))]
pub fn current() -> Lang {
    Lang::ALL.get(CURRENT.load(Ordering::Relaxed) as usize).copied().unwrap_or(Lang::Fr)
}

#[cfg(test)]
pub fn current() -> Lang {
    SPOKEN.with(std::cell::Cell::get)
}

/// This test thread speaks `lang` from now on.
#[cfg(test)]
pub fn speak(lang: Lang) {
    SPOKEN.with(|spoken| spoken.set(lang));
}

/// `text` in `lang`.
pub fn text_in(lang: Lang, text: Text<'_>) -> String {
    match lang {
        Lang::Fr => fr::text(text),
        Lang::En => en::text(text),
    }
}

/// `text` in the language of the moment.
pub fn tr(text: Text<'_>) -> String {
    text_in(current(), text)
}

/// An error made of a text reads in the language of the moment (`ok_or(Text::…)?`).
impl From<Text<'_>> for String {
    fn from(text: Text<'_>) -> String {
        tr(text)
    }
}

/// A text as the tests compare it: what it says in the test thread's language.
#[cfg(test)]
impl PartialEq<Text<'_>> for String {
    fn eq(&self, text: &Text<'_>) -> bool {
        *self == tr(*text)
    }
}

#[cfg(test)]
impl PartialEq<Text<'_>> for &str {
    fn eq(&self, text: &Text<'_>) -> bool {
        *self == tr(*text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chosen_language_wins_over_windows() {
        let windows = vec!["en-US".to_owned()];
        assert_eq!(resolve("fr", &windows), Lang::Fr);
        assert_eq!(resolve("en", &["fr-FR".to_owned()]), Lang::En);
    }

    #[test]
    fn auto_takes_the_first_language_of_windows_ilot_speaks() {
        let list = |tags: &[&str]| tags.iter().map(|t| (*t).to_owned()).collect::<Vec<_>>();
        assert_eq!(resolve("auto", &list(&["fr-FR", "en-US"])), Lang::Fr);
        assert_eq!(resolve("auto", &list(&["fr-CA"])), Lang::Fr);
        assert_eq!(resolve("auto", &list(&["de-DE", "en-GB", "fr-FR"])), Lang::En);
        assert_eq!(resolve("auto", &list(&["de-DE", "FR-be"])), Lang::Fr);
        assert_eq!(resolve("auto", &list(&["de-DE", "ja-JP"])), Lang::En, "English when îlot speaks none of them");
        assert_eq!(resolve("auto", &[]), Lang::En);
        assert_eq!(resolve("klingon", &list(&["fr-FR"])), Lang::Fr, "an unknown choice follows Windows");
    }

    #[test]
    fn the_settings_set_the_language_auto_by_windows() {
        assert_eq!(set("en"), Lang::En);
        assert_eq!(set("fr"), Lang::Fr);
        assert_eq!(set("auto"), resolve("auto", &ilot_bridge::lang::system_languages()));
    }

    #[test]
    fn languages_have_their_codes_and_speech_engines() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
            assert!(!lang.speech_lcids().is_empty());
        }
        assert_eq!(Lang::En.speech_lcids()[0], 1033, "MS-1033-…: English (United States) first");
        assert_eq!(Lang::Fr.speech_lcids(), [0x040C]);
    }

    #[test]
    fn the_tests_speak_french_unless_they_ask() {
        assert_eq!(current(), Lang::Fr);
        speak(Lang::En);
        assert_eq!(current(), Lang::En);
        assert_eq!(tr(Text::TrayQuit), "Quit");
        speak(Lang::Fr);
        assert_eq!(tr(Text::TrayQuit), "Quitter");
        let error: String = Text::TrayQuit.into();
        assert_eq!(error, Text::TrayQuit);
    }

    #[test]
    fn each_language_makes_its_own_plurals() {
        let open = |lang, n| text_in(lang, Text::GithubOpenPrs { n });
        assert_eq!((open(Lang::Fr, 0), open(Lang::Fr, 1), open(Lang::Fr, 3)), ("0 PR ouverte".into(), "1 PR ouverte".into(), "3 PR ouvertes".into()));
        assert_eq!((open(Lang::En, 0), open(Lang::En, 1), open(Lang::En, 3)), ("0 open PRs".into(), "1 open PR".into(), "3 open PRs".into()));
        assert_eq!(text_in(Lang::En, Text::ClaudeAsks { count: 1 }), "Claude has a question for you");
        assert_eq!(text_in(Lang::Fr, Text::Money { sign: "", whole: 1234, cents: "50", symbol: "€" }), "1\u{202f}234,50\u{a0}€");
        assert_eq!(text_in(Lang::En, Text::Money { sign: "", whole: 1234, cents: "50", symbol: "€" }), "€1,234.50");
    }

    #[test]
    fn every_text_exists_in_every_language() {
        let samples = Text::samples();
        assert!(samples.len() > 300, "{}", samples.len());
        for text in samples {
            let said: Vec<String> = Lang::ALL.iter().map(|lang| text_in(*lang, text)).collect();
            for (lang, words) in Lang::ALL.iter().zip(&said) {
                assert!(!words.trim().is_empty(), "{text:?} is empty in {lang:?}");
                assert_eq!(words.trim(), words, "{text:?} in {lang:?}: no space around");
                assert!(!words.contains("  "), "{text:?} in {lang:?}: {words:?}");
                // Every language places the same names and numbers.
                for value in ["<value>", "7"] {
                    assert_eq!(words.contains(value), said[0].contains(value), "{text:?} in {lang:?}: {words:?}");
                }
            }
        }
    }

    #[test]
    fn english_reads_as_english() {
        for text in Text::samples() {
            let english = text_in(Lang::En, text);
            // French typography stays in French: no space before « : » or « ? », no « ».
            assert!(!english.contains(" :") && !english.contains(" ?") && !english.contains(" !"), "{text:?}: {english:?}");
            assert!(!english.contains('«') && !english.contains('»'), "{text:?}: {english:?}");
            assert!(!english.contains('\u{a0}') && !english.contains('\u{202f}'), "{text:?}: {english:?}");
        }
    }
}
