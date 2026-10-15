//! Little memory at rest: when nothing goes on in the island, its webview is asked to use
//! as little memory as it can (WebView2 `MemoryUsageTargetLevel`, low), and to come back
//! to normal on the first event. Scripts keep running; WebView2 frees what it can, as a
//! best effort, and may page some of it out: the interface asks for normal as soon as it
//! is used again
//! (https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_19).

use crate::window::WINDOW_LABEL;
use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_19,
};
use windows::core::Interface;

/// The level asked of WebView2.
pub fn level(low: bool) -> COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL {
    if low { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL }
}

/// The island at rest (`low`) or in use again. An older WebView2 without the setting, or
/// no island window, changes nothing.
#[tauri::command]
pub fn island_memory(low: bool, app: AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else { return };
    let _ = window.with_webview(move |webview| {
        // SAFETY: COM calls on the island's own webview, on the thread Tauri runs them on;
        // a failed query only leaves the level as it was.
        unsafe {
            if let Ok(core) = webview.controller().CoreWebView2()
                && let Ok(core) = core.cast::<ICoreWebView2_19>()
            {
                let _ = core.SetMemoryUsageTargetLevel(level(low));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_asks_for_little_memory_and_use_for_the_normal_level() {
        assert_eq!(level(true), COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
        assert_eq!(level(false), COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL);
    }
}
