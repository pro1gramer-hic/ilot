//! The volume of Spotify alone, not of the whole PC: the master volume of its audio
//! sessions on the default output, the one Windows' volume mixer sets
//! (https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nn-audioclient-isimpleaudiovolume).
//! Sessions are listed by IAudioSessionManager2 and recognized by their process
//! (https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-getprocessid).
//! Windows keeps the level after a restart, as it does for the mixer. Spotify's Web API is
//! not used: it needs the network, an account and Premium.

use crate::locale::Text;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::{
    IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, eConsole,
    eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
use windows::core::{GUID, Interface, PWSTR};

/// Apps whose volume îlot may set, by the id the interface uses: Spotify only.
const APPS: [(&str, &str); 1] = [("spotify", "spotify.exe")];
/// Tags îlot's own changes in the session events (SetMasterVolume's event context).
const ILOT_CONTEXT: GUID = GUID::from_u128(0x6f1c_2a7e_9d3b_4c55_8e21_0a6b_7c3d_5e19);

const NOT_PLAYING: Text<'static> = Text::SpotifySilent;
const UNKNOWN_APP: Text<'static> = Text::VolumeSpotifyOnly;

fn exe_of(app: &str) -> Option<&'static str> {
    APPS.iter().find(|(id, _)| *id == app).map(|(_, exe)| *exe)
}

/// A level between 0 and 1, whatever comes in.
pub fn clamp_level(level: f32) -> f32 {
    if level.is_finite() { level.clamp(0.0, 1.0) } else { 0.0 }
}

fn process_exe(pid: u32) -> Option<String> {
    // SAFETY: plain queries; the handle is closed before returning.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 512];
        let mut len = buffer.len() as u32;
        let read = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        read.ok()?;
        String::from_utf16_lossy(&buffer[..len as usize]).rsplit('\\').next().map(str::to_lowercase)
    }
}

/// The volume controls of every audio session of this executable on the default output.
fn sessions_of(exe: &str) -> windows::core::Result<Vec<ISimpleAudioVolume>> {
    // SAFETY: COM calls on this thread, joined to the multithreaded apartment first (an
    // « already initialized » answer is fine); interfaces are released when dropped.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let manager: IAudioSessionManager2 = devices.GetDefaultAudioEndpoint(eRender, eConsole)?.Activate(CLSCTX_ALL, None)?;
        let list = manager.GetSessionEnumerator()?;
        let mut found = Vec::new();
        for index in 0..list.GetCount()? {
            let Ok(control) = list.GetSession(index) else { continue };
            // A session shared by several processes answers with the one that created it.
            let pid = control.cast::<IAudioSessionControl2>().and_then(|c| c.GetProcessId());
            if pid.ok().and_then(process_exe).as_deref() == Some(exe)
                && let Ok(volume) = control.cast::<ISimpleAudioVolume>()
            {
                found.push(volume);
            }
        }
        Ok(found)
    }
}

/// The app's level (0 to 1), or nothing when it has no audio session yet.
#[tauri::command]
pub async fn app_volume(app: String) -> Result<Option<f32>, String> {
    let exe = exe_of(&app).ok_or(UNKNOWN_APP)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sessions = sessions_of(exe).map_err(|e| e.to_string())?;
        // SAFETY: plain getter on a live interface.
        Ok(sessions.first().and_then(|volume| unsafe { volume.GetMasterVolume() }.ok()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sets the app's level (0 to 1) and returns it.
#[tauri::command]
pub async fn set_app_volume(app: String, level: f32) -> Result<f32, String> {
    let exe = exe_of(&app).ok_or(UNKNOWN_APP)?;
    let level = clamp_level(level);
    tauri::async_runtime::spawn_blocking(move || {
        let sessions = sessions_of(exe).map_err(|e| e.to_string())?;
        if sessions.is_empty() {
            return Err(NOT_PLAYING.into());
        }
        for volume in &sessions {
            // SAFETY: setter on a live interface; the context GUID lives in a constant.
            unsafe { volume.SetMasterVolume(level, &ILOT_CONTEXT) }.map_err(|e| e.to_string())?;
        }
        Ok(level)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_spotify_may_be_set() {
        assert_eq!(exe_of("spotify"), Some("spotify.exe"));
        assert_eq!(exe_of("explorer"), None);
        assert_eq!(exe_of("spotify.exe"), None);
    }

    #[test]
    fn levels_stay_between_zero_and_one() {
        assert_eq!(clamp_level(0.4), 0.4);
        assert_eq!(clamp_level(3.0), 1.0);
        assert_eq!(clamp_level(-1.0), 0.0);
        assert_eq!(clamp_level(f32::NAN), 0.0);
    }

    /// Lists the audio sessions of Spotify on this PC, without changing anything.
    #[test]
    #[ignore = "needs a Windows desktop with Spotify playing"]
    fn reads_spotify_sessions() {
        let sessions = sessions_of("spotify.exe").unwrap();
        // SAFETY: getter on live interfaces.
        let levels: Vec<f32> = sessions.iter().filter_map(|v| unsafe { v.GetMasterVolume() }.ok()).collect();
        println!("Spotify: {levels:?}");
    }

    #[test]
    fn other_apps_are_refused_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let error = tauri::async_runtime::block_on(app_volume("deezer".into())).unwrap_err();
        assert_eq!(error, "Volume can be set for Spotify only.");
    }
}
