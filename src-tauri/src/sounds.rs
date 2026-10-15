//! îlot's sounds (prompt 12, lot 1). The interface makes each sound (src/lib/sound): it
//! synthesizes îlot's own, or decodes the user's replacement file, then hands the core a
//! short WAV to play. The core plays it with PlaySound, so a sound never waits for a
//! click in the page (WebView2 keeps a Web Audio context suspended until the user
//! touches the page: https://github.com/tauri-apps/tauri/issues/9968).
//!
//! Replacing a sound: the user drops `finish.wav`, `approval.mp3`… in one fixed folder,
//! `%APPDATA%\ilot\sons`, that « Ouvrir le dossier des sons » creates and opens. The core
//! only looks there, only for the names of `SOUND_IDS` with a `wav`, `mp3` or `ogg`
//! extension, only regular files of 2 MB at most, and gives their bytes to the interface
//! through the IPC (no file access for the page, the CSP stays as it is).

use crate::locale::{Text, tr};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FLAGS, SND_MEMORY, SND_NODEFAULT};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// Every sound, by the name its replacement file takes (same list as src/lib/sound/catalog.ts).
pub const SOUND_IDS: [&str; 20] = [
    "finish", "fail", "approval", "allow", "deny", "message", "masked", "track", "toast", "jump", "swallow", "click",
    "dizzy", "hello", "yawn", "leave", "back", "detach", "suspend", "wardrobe",
];
/// Extensions read, in order of preference when two files share a name.
pub const EXTENSIONS: [&str; 3] = ["wav", "mp3", "ogg"];
/// A replacement file larger than this is left aside.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Longest WAV the interface may ask to play (a few seconds of 16-bit sound).
const MAX_WAV_BYTES: usize = 4 * 1024 * 1024;
const WAV_HEADER: usize = 44;

/// `%APPDATA%\ilot\sons`, next to the settings.
pub fn sounds_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("ilot").join("sons"))
}

/// A replacement file found in the folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundFile {
    pub id: String,
    /// Its name on disk, as shown to the user.
    pub file: String,
    /// False when it is too large to be used.
    pub usable: bool,
}

/// One entry of the folder: its name, its size, and whether it is a plain file (not a
/// folder nor a link).
pub struct Entry {
    pub name: String,
    pub bytes: u64,
    pub plain: bool,
}

/// The sound id and extension rank of a file name (`Finish.WAV` counts as `finish.wav`).
fn parse(name: &str) -> Option<(&'static str, usize)> {
    let (stem, extension) = name.rsplit_once('.')?;
    let id = SOUND_IDS.iter().find(|id| id.eq_ignore_ascii_case(stem))?;
    let rank = EXTENSIONS.iter().position(|e| e.eq_ignore_ascii_case(extension))?;
    Some((id, rank))
}

/// The replacement of each sound, if any: plain files named after a sound, with a known
/// extension; a usable file wins over a too large one, then `wav` over `mp3` over `ogg`.
pub fn pick(entries: &[Entry]) -> Vec<SoundFile> {
    let mut best: Vec<(&str, bool, usize, &str)> = Vec::new();
    for entry in entries.iter().filter(|e| e.plain && e.bytes > 0) {
        let Some((id, rank)) = parse(&entry.name) else { continue };
        let usable = entry.bytes <= MAX_FILE_BYTES;
        let candidate = (id, usable, rank, entry.name.as_str());
        match best.iter_mut().find(|(other, ..)| *other == id) {
            Some(current) if (usable, std::cmp::Reverse(rank)) > (current.1, std::cmp::Reverse(current.2)) => *current = candidate,
            Some(_) => {}
            None => best.push(candidate),
        }
    }
    SOUND_IDS
        .iter()
        .filter_map(|id| best.iter().find(|(other, ..)| other == id))
        .map(|(id, usable, _, file)| SoundFile { id: (*id).to_owned(), file: (*file).to_owned(), usable: *usable })
        .collect()
}

/// The folder's entries; a missing folder has none.
fn entries(dir: &Path) -> Vec<Entry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    read.filter_map(Result::ok)
        .filter_map(|entry| {
            // The link itself, not its target: a link is never followed.
            let meta = std::fs::symlink_metadata(entry.path()).ok()?;
            Some(Entry { name: entry.file_name().to_string_lossy().into_owned(), bytes: meta.len(), plain: meta.is_file() })
        })
        .collect()
}

/// The bytes of the replacement of `id`, read from `dir` (at most `MAX_FILE_BYTES`).
pub fn read_override(dir: &Path, id: &str) -> Result<Vec<u8>, String> {
    if !SOUND_IDS.contains(&id) {
        return Err(Text::SoundUnknown.into());
    }
    let found = pick(&entries(dir)).into_iter().find(|f| f.id == id && f.usable).ok_or(Text::SoundNoFile)?;
    let path = dir.join(&found.file);
    let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(Text::SoundNoFile.into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(Text::SoundTooHeavy.into());
    }
    Ok(bytes)
}

/// A RIFF/WAVE file of 16-bit PCM, mono or stereo, small enough: the only thing the core
/// accepts to play.
pub fn is_playable_wav(bytes: &[u8]) -> bool {
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    bytes.len() > WAV_HEADER
        && bytes.len() <= MAX_WAV_BYTES
        && &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WAVE"
        && &bytes[12..16] == b"fmt "
        && u16_at(20) == 1
        && matches!(u16_at(22), 1 | 2)
        && u16_at(34) == 16
}

/// The sound playing now: PlaySound reads it from memory until it ends.
static PLAYING: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Plays a WAV at once, cutting the previous sound (UI sounds are short).
/// https://learn.microsoft.com/en-us/previous-versions/dd743680(v=vs.85)
fn play(wav: Vec<u8>) -> Result<(), String> {
    if !is_playable_wav(&wav) {
        return Err(Text::SoundUnreadable.into());
    }
    let mut playing = PLAYING.lock().map_err(|_| Text::SoundBusy)?;
    // SAFETY: a null name stops the sound playing, which may still read the old buffer;
    // only then is that buffer replaced. The new one lives in the static until the next
    // call stops it, as SND_MEMORY with SND_ASYNC requires.
    unsafe {
        let _ = PlaySoundW(PCWSTR::null(), None, SND_FLAGS(0));
        *playing = wav;
        PlaySoundW(PCWSTR(playing.as_ptr().cast()), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT)
            .ok()
            .map_err(|e| e.to_string())
    }
}

/// Plays a sound the interface made (raw WAV bytes in the request's body).
#[tauri::command]
pub fn play_sound(request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err(Text::SoundUnreadable.into()) };
    play(bytes.clone())
}

/// The replacement files found in the sounds folder.
#[tauri::command]
pub fn sound_overrides() -> Vec<SoundFile> {
    sounds_dir().map(|dir| pick(&entries(&dir))).unwrap_or_default()
}

/// The bytes of a replacement file, for the interface to decode.
#[tauri::command]
pub async fn sound_file(id: String) -> Result<tauri::ipc::Response, String> {
    let dir = sounds_dir().ok_or(Text::SoundsFolderMissing)?;
    tauri::async_runtime::spawn_blocking(move || read_override(&dir, &id))
        .await
        .map_err(|e| e.to_string())?
        .map(tauri::ipc::Response::new)
}

/// Creates the sounds folder if needed and opens it in the Explorer (a fixed path, never
/// one received from the interface).
#[tauri::command]
pub fn open_sounds_folder() -> Result<(), String> {
    let dir = sounds_dir().ok_or(Text::SoundsFolderMissing)?;
    std::fs::create_dir_all(&dir).map_err(|e| tr(Text::SoundsFolderFailed { error: &e.to_string() }))?;
    let path = HSTRING::from(dir.as_os_str());
    // SAFETY: constant verb and a null-terminated path; ShellExecuteW returns a value
    // above 32 on success. « explore » only ever opens a folder.
    let result = unsafe { ShellExecuteW(None, w!("explore"), &path, None, None, SW_SHOWNORMAL) };
    if result.0 as isize > 32 { Ok(()) } else { Err(Text::SoundsFolderCannotOpen.into()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, bytes: u64) -> Entry {
        Entry { name: name.into(), bytes, plain: true }
    }

    #[test]
    fn only_known_names_and_extensions_count() {
        let found = pick(&[
            entry("finish.wav", 10),
            entry("approval.mp3", 10),
            entry("message.ogg", 10),
            entry("finish.flac", 10),
            entry("bonjour.wav", 10),
            entry("..\\finish.wav", 10),
            entry("deny.wav.exe", 10),
            entry("click", 10),
        ]);
        let ids: Vec<(&str, &str)> = found.iter().map(|f| (f.id.as_str(), f.file.as_str())).collect();
        assert_eq!(ids, vec![("finish", "finish.wav"), ("approval", "approval.mp3"), ("message", "message.ogg")]);
    }

    #[test]
    fn names_are_read_without_case_and_wav_wins() {
        let found = pick(&[entry("Hello.OGG", 10), entry("hello.Mp3", 10), entry("HELLO.wav", 10)]);
        assert_eq!(found, vec![SoundFile { id: "hello".into(), file: "HELLO.wav".into(), usable: true }]);
    }

    #[test]
    fn large_empty_and_odd_entries_are_left_aside() {
        let big = MAX_FILE_BYTES + 1;
        let found = pick(&[
            entry("fail.wav", big),
            entry("fail.ogg", 100),
            entry("toast.wav", big),
            entry("jump.wav", 0),
            Entry { name: "yawn.wav".into(), bytes: 10, plain: false },
        ]);
        assert_eq!(
            found,
            vec![
                SoundFile { id: "fail".into(), file: "fail.ogg".into(), usable: true },
                SoundFile { id: "toast".into(), file: "toast.wav".into(), usable: false },
            ]
        );
    }

    #[test]
    fn a_replacement_is_read_from_the_folder_only_for_a_known_sound() {
        let dir = std::env::temp_dir().join(format!("ilot-sons-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("click.wav"), b"RIFF-click").unwrap();
        std::fs::write(dir.join("other.wav"), b"RIFF-other").unwrap();
        assert_eq!(read_override(&dir, "click").unwrap(), b"RIFF-click");
        assert_eq!(read_override(&dir, "jump").unwrap_err(), "Aucun fichier pour ce son");
        assert_eq!(read_override(&dir, "../other").unwrap_err(), "Son inconnu");
        assert_eq!(read_override(&dir, "other").unwrap_err(), "Son inconnu");
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(read_override(&dir, "click").is_err(), "a missing folder gives nothing");
    }

    /// A 16-bit mono WAV header followed by `samples` silent samples.
    fn wav(channels: u16, bits: u16, samples: usize) -> Vec<u8> {
        let data = samples * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&((36 + data) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&22_050u32.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&bits.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data as u32).to_le_bytes());
        bytes.resize(44 + data, 0);
        bytes
    }

    #[test]
    fn only_small_16_bit_pcm_wav_is_played() {
        assert!(is_playable_wav(&wav(1, 16, 100)));
        assert!(is_playable_wav(&wav(2, 16, 100)));
        assert!(!is_playable_wav(&wav(1, 8, 100)), "8-bit");
        assert!(!is_playable_wav(&wav(3, 16, 100)));
        assert!(!is_playable_wav(&wav(1, 16, 0)), "a header alone");
        assert!(!is_playable_wav(&wav(1, 16, MAX_WAV_BYTES)), "too long");
        let mut mp3 = wav(1, 16, 100);
        mp3[0..4].copy_from_slice(b"ID3\x04");
        assert!(!is_playable_wav(&mp3));
        assert!(!is_playable_wav(b"RIFF"));
    }

    #[test]
    fn every_sound_of_the_interface_is_known_here() {
        let catalog = include_str!("../../src/lib/sound/catalog.ts");
        for id in SOUND_IDS {
            assert!(catalog.contains(&format!("\"{id}\"")), "{id} missing from catalog.ts");
        }
        assert_eq!(SOUND_IDS.len(), 20);
    }

    #[test]
    fn refusals_are_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let dir = std::env::temp_dir().join(format!("ilot-sounds-en-{}", std::process::id()));
        assert_eq!(read_override(&dir, "other").unwrap_err(), "Unknown sound");
        assert_eq!(read_override(&dir, "jump").unwrap_err(), "No file for this sound");
    }
}
