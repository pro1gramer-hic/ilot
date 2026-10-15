//! Saving the report image (1080 × 1920 PNG drawn by the interface) where the user
//! chooses, in Windows' own « Enregistrer sous » dialog: nothing is written before the
//! user picked a file, only a PNG of that size is accepted, and nothing goes on the network.
//! https://learn.microsoft.com/en-us/windows/win32/shell/common-file-dialog

use crate::atomic::write_atomic;
use crate::locale::{Text, tr};
use std::path::PathBuf;
use tauri::ipc::{InvokeBody, Request};
use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_STRICTFILETYPES, FileSaveDialog, IFileSaveDialog, SIGDN_FILESYSPATH};
use windows::core::{HSTRING, w};

pub const WIDTH: u32 = 1080;
pub const HEIGHT: u32 = 1920;
/// A larger file is not a report image.
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// The suggested file name travels in this header of the request.
const NAME_HEADER: &str = "x-ilot-name";

/// True for a PNG of the report's size (its IHDR chunk comes first).
pub fn is_report_png(bytes: &[u8]) -> bool {
    let size = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    bytes.len() <= MAX_BYTES
        && bytes.starts_with(&PNG_SIGNATURE)
        && bytes.get(12..16) == Some(b"IHDR")
        && size(16) == Some(WIDTH)
        && size(20) == Some(HEIGHT)
}

/// A file name made of letters, digits, spaces and dashes, ending in `.png`.
pub fn file_name(asked: &str) -> String {
    let clean: String = asked.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-').take(60).collect();
    let clean = clean.trim();
    if clean.is_empty() { format!("{}.png", tr(Text::ReportFileName)) } else { format!("{clean}.png") }
}

/// Shows the dialog on a thread of its own (it needs a single-threaded apartment) and
/// returns the chosen path, none when the user cancelled.
fn choose(owner: isize, name: String) -> Result<Option<PathBuf>, String> {
    let thread = std::thread::spawn(move || -> windows::core::Result<Option<PathBuf>> {
        // SAFETY: COM is set up for this thread and closed before it ends; the dialog and
        // the strings it returns are used on it only.
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            let chosen = (|| {
                let dialog: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_ALL)?;
                let filter = HSTRING::from(tr(Text::ReportImageFilter));
                dialog.SetFileTypes(&[COMDLG_FILTERSPEC { pszName: windows::core::PCWSTR(filter.as_ptr()), pszSpec: w!("*.png") }])?;
                dialog.SetDefaultExtension(w!("png"))?;
                dialog.SetFileName(&HSTRING::from(name))?;
                dialog.SetTitle(&HSTRING::from(tr(Text::ReportSaveTitle)))?;
                dialog.SetOptions(dialog.GetOptions()? | FOS_OVERWRITEPROMPT | FOS_FORCEFILESYSTEM | FOS_STRICTFILETYPES)?;
                if dialog.Show(Some(HWND(owner as *mut _))).is_err() {
                    return Ok(None);
                }
                let path = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
                let text = path.to_string();
                CoTaskMemFree(Some(path.0 as *const _));
                Ok(text.ok().map(PathBuf::from))
            })();
            CoUninitialize();
            chosen
        }
    });
    thread.join().map_err(|_| tr(Text::ReportDialogClosed))?.map_err(|e| e.message())
}

/// « Exporter l'image »: the PNG comes as the raw body of the request. True once saved,
/// false when the user cancelled.
#[tauri::command]
pub async fn save_report_image(request: Request<'_>, window: WebviewWindow) -> Result<bool, String> {
    let InvokeBody::Raw(bytes) = request.body() else { return Err(Text::ReportImageMissing.into()) };
    if !is_report_png(bytes) {
        return Err(Text::ReportImageRefused.into());
    }
    let name = request.headers().get(NAME_HEADER).and_then(|v| v.to_str().ok()).map(file_name).unwrap_or_else(|| file_name(""));
    let owner = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let bytes = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = choose(owner, name)? else { return Ok(false) };
        write_atomic(&path, &bytes).map_err(|e| tr(Text::SaveFailed { error: &e.to_string() }))?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    #[test]
    fn only_a_png_of_the_report_size_is_saved() {
        assert!(is_report_png(&png(1080, 1920)));
        assert!(!is_report_png(&png(1920, 1080)));
        assert!(!is_report_png(b"GIF89a"));
        assert!(!is_report_png(&[]));
        let mut huge = png(1080, 1920);
        huge.resize(MAX_BYTES + 1, 0);
        assert!(!is_report_png(&huge));
    }

    #[test]
    fn the_suggested_name_is_tamed() {
        assert_eq!(file_name("Bilan 2026-10-05"), "Bilan 2026-10-05.png");
        assert_eq!(file_name("..\\..\\Windows\\evil"), "Windowsevil.png", "no folder, only a name");
        assert_eq!(file_name("   "), "Bilan îlot.png");
    }

    #[test]
    fn the_default_name_follows_the_language() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(file_name("   "), "îlot report.png");
        assert_eq!(file_name("Report 2026-10-05"), "Report 2026-10-05.png");
    }
}
