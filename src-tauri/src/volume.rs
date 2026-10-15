//! The wheel on the pill sets the system volume: the master level of the default output
//! (https://learn.microsoft.com/en-us/windows/win32/api/endpointvolume/nn-endpointvolume-iaudioendpointvolume).

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};

/// One wheel notch moves the volume by this much (Windows' own keys move it by 2 %).
pub const STEP: f32 = 0.02;

fn endpoint() -> windows::core::Result<IAudioEndpointVolume> {
    // SAFETY: COM calls on the current thread, initialized first (an "already initialized"
    // answer is fine); the interfaces live as long as the returned value.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        devices.GetDefaultAudioEndpoint(eRender, eConsole)?.Activate(CLSCTX_ALL, None)
    }
}

/// The level after `notches` wheel notches up (positive) or down, between 0 and 1.
pub fn stepped(level: f32, notches: i32) -> f32 {
    (level + notches as f32 * STEP).clamp(0.0, 1.0)
}

/// Current level, between 0 and 1.
pub fn level() -> Result<f32, String> {
    // SAFETY: plain getter on a live interface.
    unsafe { endpoint().and_then(|volume| volume.GetMasterVolumeLevelScalar()) }.map_err(|e| e.to_string())
}

/// Moves the level by whole notches and returns the new level.
pub fn change(notches: i32) -> Result<f32, String> {
    let volume = endpoint().map_err(|e| e.to_string())?;
    // SAFETY: getter and setter on a live interface; no event context GUID.
    unsafe {
        let next = stepped(volume.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?, notches);
        volume.SetMasterVolumeLevelScalar(next, std::ptr::null()).map_err(|e| e.to_string())?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notches_move_by_two_percent_and_stay_in_bounds() {
        assert!((stepped(0.5, 3) - 0.56).abs() < 1e-6);
        assert!((stepped(0.5, -1) - 0.48).abs() < 1e-6);
        assert_eq!(stepped(0.99, 5), 1.0);
        assert_eq!(stepped(0.01, -5), 0.0);
    }

    #[test]
    fn the_default_output_has_a_level() {
        // Every Windows desktop has a default output; its level reads between 0 and 1.
        let level = level().unwrap();
        assert!((0.0..=1.0).contains(&level));
    }
}
