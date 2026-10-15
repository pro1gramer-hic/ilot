//! Which language îlot speaks, for the app and for `ilot-cli`: the one chosen in
//! îlot's settings, or with « auto » the first display language of Windows that îlot
//! speaks (the rule of src/lib/i18n/locale.ts).

/// `chosen` when it is one of `spoken`; otherwise (« auto ») the first of `system` whose
/// base (« fr » of « fr-CA ») is spoken; `fallback` when none is.
pub fn resolve<'a>(chosen: &str, system: &[String], spoken: &[&'a str], fallback: &'a str) -> &'a str {
    if let Some(code) = spoken.iter().find(|code| **code == chosen) {
        return code;
    }
    system
        .iter()
        .find_map(|tag| {
            let base = tag.to_lowercase();
            let base = base.split(['-', '_']).next().unwrap_or_default();
            spoken.iter().find(|code| **code == base).copied()
        })
        .unwrap_or(fallback)
}

/// The display languages of the user, preferred first (« fr-FR », « en-US »).
/// https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getuserpreferreduilanguages
pub fn system_languages() -> Vec<String> {
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    use windows::core::PWSTR;
    let mut count = 0u32;
    let mut size = 0u32;
    // SAFETY: a first call with no buffer asks for its size; the second fills a buffer of
    // exactly that size.
    unsafe {
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut size).is_err() || size == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u16; size as usize];
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, Some(PWSTR(buffer.as_mut_ptr())), &mut size).is_err() {
            return Vec::new();
        }
        multi_string(&buffer[..(size as usize).min(buffer.len())])
    }
}

/// The strings of a list ended by an empty one (`fr-FR\0en-US\0\0`).
fn multi_string(wide: &[u16]) -> Vec<String> {
    wide.split(|c| *c == 0).take_while(|part| !part.is_empty()).map(String::from_utf16_lossy).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOKEN: [&str; 2] = ["fr", "en"];

    fn list(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|t| (*t).to_owned()).collect()
    }

    #[test]
    fn a_chosen_language_wins_over_windows() {
        assert_eq!(resolve("fr", &list(&["en-US"]), &SPOKEN, "en"), "fr");
        assert_eq!(resolve("en", &list(&["fr-FR"]), &SPOKEN, "en"), "en");
    }

    #[test]
    fn auto_takes_the_first_language_of_windows_ilot_speaks() {
        assert_eq!(resolve("auto", &list(&["fr-FR", "en-US"]), &SPOKEN, "en"), "fr");
        assert_eq!(resolve("auto", &list(&["fr-CA"]), &SPOKEN, "en"), "fr");
        assert_eq!(resolve("auto", &list(&["de-DE", "en-GB", "fr-FR"]), &SPOKEN, "en"), "en");
        assert_eq!(resolve("auto", &list(&["de-DE", "FR-be"]), &SPOKEN, "en"), "fr");
        assert_eq!(resolve("auto", &list(&["de-DE", "ja-JP"]), &SPOKEN, "en"), "en", "English when îlot speaks none of them");
        assert_eq!(resolve("auto", &[], &SPOKEN, "en"), "en");
        assert_eq!(resolve("klingon", &list(&["fr-FR"]), &SPOKEN, "en"), "fr", "an unknown choice follows Windows");
    }

    #[test]
    fn windows_lists_its_languages_as_a_multi_string() {
        let wide: Vec<u16> = "fr-FR\0en-US\0\0".encode_utf16().collect();
        assert_eq!(multi_string(&wide), ["fr-FR", "en-US"]);
        assert!(multi_string(&[0, 0]).is_empty());
        let system = system_languages();
        assert!(!system.is_empty(), "Windows always has a display language");
        assert!(system.iter().all(|tag| tag.len() >= 2 && !tag.contains('\0')));
    }
}
