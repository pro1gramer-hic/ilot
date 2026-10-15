//! Puts text on the Windows clipboard: the reply to a message (for the app where the
//! user pastes it) and the path of a dropped file. Unicode text only, written with
//! `OpenClipboard`, `EmptyClipboard` and `SetClipboardData(CF_UNICODETEXT)`
//! (https://learn.microsoft.com/en-us/windows/win32/dataxchg/using-the-clipboard).
//! îlot reads the clipboard at one moment only: when the user starts « Traduire le
//! presse-papiers » (shortcut, « OK Mochi »), and then its text alone, bounded; nothing
//! leaves before the island's « Traduire » (translate.rs). Never in the background.

use crate::locale::Text;
use serde::Serialize;
use std::time::Duration;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};

/// Standard clipboard format of UTF-16 text
/// (https://learn.microsoft.com/en-us/windows/win32/dataxchg/standard-clipboard-formats).
const CF_UNICODETEXT: u32 = 13;
/// Longer than any reply or path; more is refused rather than cut.
pub const MAX_CHARS: usize = 100_000;
/// Another app may hold the clipboard open for a moment.
const ATTEMPTS: u32 = 5;
const RETRY: Duration = Duration::from_millis(10);

const TOO_LONG: Text<'static> = Text::ClipboardTooLong;
const INVALID: Text<'static> = Text::ClipboardInvalid;
const BUSY: Text<'static> = Text::ClipboardBusy;
const FAILED: Text<'static> = Text::ClipboardFailed;

/// The text as Windows wants it: UTF-16, lines ending in CR LF, a final NUL. A NUL inside
/// would cut the text silently: refused.
pub fn clipboard_text(text: &str) -> Result<Vec<u16>, Text<'static>> {
    if text.chars().count() > MAX_CHARS {
        return Err(TOO_LONG);
    }
    if text.contains('\0') {
        return Err(INVALID);
    }
    let mut wide = Vec::with_capacity(text.len() + 1);
    let mut previous = '\0';
    for c in text.chars() {
        if c == '\n' && previous != '\r' {
            wide.push(u16::from(b'\r'));
        }
        let mut units = [0u16; 2];
        wide.extend_from_slice(c.encode_utf16(&mut units));
        previous = c;
    }
    wide.push(0);
    Ok(wide)
}

/// Replaces the clipboard's content with `text`. The window becomes the clipboard owner:
/// without an owner, `SetClipboardData` fails (OpenClipboard documentation).
pub fn write(owner: HWND, text: &str) -> Result<(), String> {
    let wide = clipboard_text(text)?;
    let mut opened = false;
    for attempt in 0..ATTEMPTS {
        // SAFETY: `owner` is a live window of this process.
        if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
            opened = true;
            break;
        }
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(RETRY);
        }
    }
    if !opened {
        return Err(BUSY.into());
    }
    let result = fill(&wide);
    // SAFETY: the clipboard was opened above by this thread.
    let _ = unsafe { CloseClipboard() };
    result.map_err(|_| FAILED.into())
}

/// Empties the open clipboard and gives it a copy of the text. Once `SetClipboardData`
/// succeeds, the memory belongs to the system; before that, it is freed here.
fn fill(wide: &[u16]) -> windows::core::Result<()> {
    // SAFETY: the clipboard is open (`write`); the global memory block is allocated for
    // exactly `wide.len()` units, locked while it is written, then handed to the system.
    unsafe {
        EmptyClipboard()?;
        let memory = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of_val(wide))?;
        let target = GlobalLock(memory).cast::<u16>();
        if target.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err(windows::core::Error::from_thread());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
        // Reports an "error" once the lock count reaches zero: expected here.
        let _ = GlobalUnlock(memory);
        if let Err(error) = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(memory.0))) {
            let _ = GlobalFree(Some(memory));
            return Err(error);
        }
    }
    Ok(())
}

/// Copies a reply or a path; the calling window owns the clipboard.
#[tauri::command]
pub fn copy_text(text: String, window: WebviewWindow) -> Result<(), String> {
    let owner = window.hwnd().map_err(|e| e.to_string())?;
    write(HWND(owner.0), &text)
}

/// Why the clipboard gave no text; the interface says it in its language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadFailure {
    /// No text in it (an image, files, nothing), or only spaces.
    Empty,
    /// More than `MAX_CHARS`: refused rather than cut.
    TooLong,
    /// Another app holds it open.
    Busy,
}

/// The text of `CF_UNICODETEXT` memory: up to its first NUL (or the end of the block),
/// lines back to `\n`. Longer than `MAX_CHARS` is refused; blank is « empty ».
pub fn text_of_units(units: &[u16]) -> Result<String, ReadFailure> {
    let end = units.iter().position(|unit| *unit == 0).unwrap_or(units.len());
    let units = &units[..end];
    // Each character takes one or two units: past twice the limit, no need to decode.
    if units.len() > MAX_CHARS * 2 {
        return Err(ReadFailure::TooLong);
    }
    let text = String::from_utf16_lossy(units).replace("\r\n", "\n");
    if text.chars().count() > MAX_CHARS {
        Err(ReadFailure::TooLong)
    } else if text.trim().is_empty() {
        Err(ReadFailure::Empty)
    } else {
        Ok(text)
    }
}

/// Reads the clipboard's text, the way Microsoft shows it (« Pasting Information from the
/// Clipboard »): format available, open, `GetClipboardData`, lock, copy, unlock, close.
/// The memory stays the system's: it is neither freed nor left locked.
pub fn read(owner: HWND) -> Result<String, ReadFailure> {
    // SAFETY: a plain query of the clipboard's formats.
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_err() {
        return Err(ReadFailure::Empty);
    }
    let mut opened = false;
    for attempt in 0..ATTEMPTS {
        // SAFETY: `owner` is a live window of this process.
        if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
            opened = true;
            break;
        }
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(RETRY);
        }
    }
    if !opened {
        return Err(ReadFailure::Busy);
    }
    let result = copy_out();
    // SAFETY: the clipboard was opened above by this thread.
    let _ = unsafe { CloseClipboard() };
    result
}

/// Copies the text out of the open clipboard's `CF_UNICODETEXT` block.
fn copy_out() -> Result<String, ReadFailure> {
    // SAFETY: the clipboard is open (`read`); the handle belongs to the system, which keeps
    // it valid until the clipboard closes. The block is locked while its units are copied
    // (never past `GlobalSize`, nor past what a text of `MAX_CHARS` can take), then unlocked.
    unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT).map_err(|_| ReadFailure::Empty)?;
        let memory = HGLOBAL(handle.0);
        let units = (GlobalSize(memory) / std::mem::size_of::<u16>()).min(MAX_CHARS * 2 + 1);
        let source = GlobalLock(memory).cast::<u16>();
        if source.is_null() {
            return Err(ReadFailure::Empty);
        }
        let copied = std::slice::from_raw_parts(source, units).to_vec();
        // Reports an "error" once the lock count reaches zero: expected here.
        let _ = GlobalUnlock(memory);
        text_of_units(&copied)
    }
}

/// « Traduire le presse-papiers »: the clipboard's text, read now and only now, for the
/// island to show before anything is sent.
#[tauri::command]
pub fn read_clipboard_text(window: WebviewWindow) -> Result<String, ReadFailure> {
    let owner = window.hwnd().map_err(|_| ReadFailure::Busy)?;
    read(HWND(owner.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    #[test]
    fn text_goes_in_as_utf16_with_windows_line_ends() {
        assert_eq!(clipboard_text("Oui, j'arrive !").unwrap(), units("Oui, j'arrive !"));
        assert_eq!(clipboard_text("a\nb\r\nc").unwrap(), units("a\r\nb\r\nc"));
        assert_eq!(clipboard_text("").unwrap(), [0]);
        assert_eq!(clipboard_text("Léa 🎧").unwrap(), units("Léa 🎧"), "accents and emoji");
        let path = r"C:\Users\HP 240\Downloads\notes.pdf";
        assert_eq!(clipboard_text(path).unwrap(), units(path), "a dropped file's path, as is");
    }

    #[test]
    fn clipboard_text_is_read_up_to_its_end_with_plain_line_ends() {
        let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
        assert_eq!(text_of_units(&wide("Hello,\r\nworld 🎧")), Ok("Hello,\nworld 🎧".to_owned()));
        let mut after_nul = wide("Bonjour");
        after_nul.extend("rest".encode_utf16());
        assert_eq!(text_of_units(&after_nul), Ok("Bonjour".to_owned()), "stops at the first NUL");
        assert_eq!(text_of_units(&"no end".encode_utf16().collect::<Vec<u16>>()), Ok("no end".to_owned()));
        assert_eq!(text_of_units(&wide("  \r\n\t")), Err(ReadFailure::Empty));
        assert_eq!(text_of_units(&[]), Err(ReadFailure::Empty));
        assert_eq!(text_of_units(&wide(&"x".repeat(MAX_CHARS + 1))), Err(ReadFailure::TooLong));
        assert!(text_of_units(&wide(&"x".repeat(MAX_CHARS))).is_ok());
        assert_eq!(serde_json::to_value(ReadFailure::TooLong).unwrap(), serde_json::json!("tooLong"));
    }

    #[test]
    fn hidden_cuts_and_huge_texts_are_refused() {
        assert_eq!(clipboard_text("a\0b"), Err(INVALID));
        assert_eq!(clipboard_text(&"x".repeat(MAX_CHARS + 1)), Err(TOO_LONG));
        assert!(clipboard_text(&"x".repeat(MAX_CHARS)).is_ok());
    }

    #[test]
    fn refusals_are_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(String::from(clipboard_text("a\0b").unwrap_err()), "This text can't go in the clipboard.");
        assert_eq!(String::from(clipboard_text(&"x".repeat(MAX_CHARS + 1)).unwrap_err()), "Text too long for the clipboard.");
    }
}
