//! Dictation in the chat (prompt 12, lot 3), with the speech recognizer Windows already
//! has: SAPI 5.4, in process (`SpInprocRecognizer`), dictation grammar, engine of the
//! interface's language (`Language=40C`, « Microsoft Speech Recognizer 8.0 for Windows
//! (French) »; `Language=409`, its English one). The sound of the microphone turns into
//! text on this PC: nothing is downloaded, installed or sent.
//! SAPI keeps no audio (`SPAO_NONE`), and the engine's adaptation to the voice is off for
//! the session (`AdaptationOn` = 0), so nothing about it is written either.
//!
//! Not used: the dictation of `Windows.Media.SpeechRecognition` without a constraint and
//! Win + H, which both send the audio to Microsoft's online service.
//! https://learn.microsoft.com/en-us/windows/apps/develop/input/speech-recognition
//! https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee125077(v=vs.85)
//! https://learn.microsoft.com/en-us/dotnet/api/system.speech.recognition.speechrecognitionengine.updaterecognizersetting
//!
//! WSR, the engine behind SAPI dictation, is deprecated (no longer developed): the mic
//! only shows when the engine of the interface's language is installed
//! (`dictation_available`).

use crate::locale::{self, Lang, Text, tr};
use crate::window::WINDOW_LABEL;
use serde::Serialize;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree};
use windows::Win32::Media::Speech::{
    ISpMMSysAudio, ISpObjectToken, ISpRecoResult, ISpRecognizer, SPAO_NONE, SPCAT_RECOGNIZERS,
    SPEI_END_SR_STREAM, SPEI_HYPOTHESIS, SPEI_RECOGNITION, SPEI_RESERVED1, SPEI_RESERVED2, SPET_LPARAM_IS_OBJECT,
    SPET_LPARAM_IS_POINTER, SPET_LPARAM_IS_STRING, SPET_LPARAM_IS_TOKEN, SPEVENT, SPEVENTENUM, SPLO_STATIC, SPRS_ACTIVE,
    SPRS_INACTIVE, SPRST_INACTIVE_WITH_PURGE, SpInprocRecognizer, SpMMAudioIn,
};
use windows::core::{IUnknown, Interface, PCWSTR, PWSTR, w};

pub const EVENT_TEXT: &str = "dictation:text";
pub const EVENT_END: &str = "dictation:end";
/// A dictation stops by itself after this long.
const MAX_LISTEN: Duration = Duration::from_secs(60);
const POLL_MS: u32 = 100;
/// `SP_GETWHOLEPHRASE` (-1 as a ULONG): every element of the phrase.
const WHOLE_PHRASE: u32 = u32::MAX;

pub const NO_MICROPHONE: Text<'static> = Text::DictationNoMicrophone;

/// Recognized text: `final` once a phrase is settled, a guess while it is spoken.
#[derive(Clone, Serialize)]
pub struct Heard<'a> {
    pub text: &'a str,
    pub r#final: bool,
}

#[derive(Clone, Serialize)]
struct End {
    error: Option<String>,
}

/// The listening under way, stopped through its flag.
#[derive(Default)]
pub struct Dictation {
    running: Mutex<Option<Arc<AtomicBool>>>,
}

/// `SPFEI(event)` of sapi.h: the event's bit and the two flag-check bits.
fn interest(events: &[SPEVENTENUM]) -> u64 {
    let check = (1u64 << SPEI_RESERVED1.0) | (1u64 << SPEI_RESERVED2.0);
    events.iter().fold(check, |bits, event| bits | (1u64 << event.0))
}

/// The installed recognizer of `lang`, if any.
///
/// # Safety
/// COM must be joined on this thread.
unsafe fn recognizer(lang: Lang) -> windows::core::Result<Option<ISpObjectToken>> {
    unsafe { crate::voice::speech_token(SPCAT_RECOGNIZERS, lang).map(|found| found.map(|(token, _)| token)) }
}

/// Dictation said unavailable in the language of the interface.
fn unavailable(lang: Lang) -> String {
    tr(Text::DictationUnavailable { lang })
}

/// True when this PC has the recognizer of the interface's language (no microphone is
/// opened to know it).
pub fn available() -> bool {
    // SAFETY: COM joined in the multithreaded apartment, then read-only token queries.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        recognizer(locale::current()).ok().flatten().is_some()
    }
}

/// Frees what an event carries (sapi.h's `SpClearEvent`), handing a recognition result to
/// `on_result` first.
///
/// # Safety
/// `event` was just filled by `GetEvents`, which hands its parameter's ownership over.
unsafe fn take(event: &SPEVENT, on_result: impl FnOnce(ISpRecoResult)) {
    let kind = (event._bitfield >> 16) & 0xFFFF;
    let pointer = event.lParam.0 as *mut c_void;
    if pointer.is_null() {
        return;
    }
    unsafe {
        if kind == SPET_LPARAM_IS_OBJECT.0 {
            let object = IUnknown::from_raw(pointer);
            if let Ok(result) = object.cast::<ISpRecoResult>() {
                on_result(result);
            }
        } else if kind == SPET_LPARAM_IS_TOKEN.0 {
            drop(ISpObjectToken::from_raw(pointer));
        } else if kind == SPET_LPARAM_IS_STRING.0 || kind == SPET_LPARAM_IS_POINTER.0 {
            CoTaskMemFree(Some(pointer.cast_const()));
        }
    }
}

/// The text of a recognition result.
///
/// # Safety
/// `result` comes from a SAPI event.
unsafe fn text_of(result: &ISpRecoResult) -> Option<String> {
    let mut text = PWSTR::null();
    unsafe {
        result.GetText(WHOLE_PHRASE, WHOLE_PHRASE, true, &mut text, None).ok()?;
        if text.is_null() {
            return None;
        }
        let words = text.to_string().ok();
        CoTaskMemFree(Some(text.0.cast_const().cast()));
        words
    }
}

/// Plugs the default microphone of Windows into the recognizer (`SpMMAudioIn`, the
/// default input device).
fn microphone(recognizer: &ISpRecognizer) -> Result<(), String> {
    // SAFETY: COM is joined on this thread by `listen`, which calls this.
    unsafe {
        let microphone: ISpMMSysAudio =
            CoCreateInstance(&SpMMAudioIn, None, CLSCTX_INPROC_SERVER).map_err(|_| NO_MICROPHONE)?;
        recognizer.SetInput(&microphone, true).map_err(|_| NO_MICROPHONE.into())
    }
}

/// Listens in the interface's language until `stop`, the end of the sound, or one minute;
/// `plug` gives the recognizer its sound (the microphone). Hands every phrase to
/// `on_text`: guesses while it is spoken, then the settled phrase.
pub fn listen(
    plug: impl FnOnce(&ISpRecognizer) -> Result<(), String>,
    stop: &AtomicBool,
    mut on_text: impl FnMut(&str, bool),
) -> Result<(), String> {
    // SAFETY: COM joined in the multithreaded apartment on this thread (never one with a
    // window); then plain calls on the objects SAPI returns, and event parameters freed
    // as SpClearEvent does.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let lang = locale::current();
        let token = recognizer(lang).ok().flatten().ok_or_else(|| unavailable(lang))?;
        let failed = |_| unavailable(lang);
        let recognizer: ISpRecognizer = CoCreateInstance(&SpInprocRecognizer, None, CLSCTX_INPROC_SERVER).map_err(failed)?;
        recognizer.SetRecognizer(&token).map_err(failed)?;
        // Adaptation would update the voice profile on disk: off for this session only.
        let _ = recognizer.SetPropertyNum(w!("AdaptationOn"), 0);
        plug(&recognizer)?;
        let context = recognizer.CreateRecoContext().map_err(|_| NO_MICROPHONE)?;
        context.SetAudioOptions(SPAO_NONE, std::ptr::null(), std::ptr::null()).map_err(failed)?;
        context.SetNotifyWin32Event().map_err(failed)?;
        let wanted = interest(&[SPEI_RECOGNITION, SPEI_HYPOTHESIS, SPEI_END_SR_STREAM]);
        context.SetInterest(wanted, wanted).map_err(failed)?;
        let grammar = context.CreateGrammar(0).map_err(failed)?;
        grammar.LoadDictation(PCWSTR::null(), SPLO_STATIC).map_err(failed)?;
        grammar.SetDictationState(SPRS_ACTIVE).map_err(|_| NO_MICROPHONE)?;
        let began = Instant::now();
        let mut ended = false;
        while !ended && !stop.load(Ordering::Relaxed) && began.elapsed() < MAX_LISTEN {
            let _ = context.WaitForNotifyEvent(POLL_MS);
            loop {
                let mut event = SPEVENT::default();
                let mut fetched = 0u32;
                if context.GetEvents(1, &mut event, &mut fetched).is_err() || fetched == 0 {
                    break;
                }
                let id = event._bitfield & 0xFFFF;
                ended |= id == SPEI_END_SR_STREAM.0;
                take(&event, |result| {
                    if let Some(text) = text_of(&result) {
                        on_text(&text, id == SPEI_RECOGNITION.0);
                    }
                });
            }
        }
        let _ = grammar.SetDictationState(SPRS_INACTIVE);
        let _ = recognizer.SetRecoState(SPRST_INACTIVE_WITH_PURGE);
        Ok(())
    }
}

/// Whether the mic button shows next to the chat's field.
#[tauri::command]
pub async fn dictation_available() -> bool {
    tauri::async_runtime::spawn_blocking(available).await.unwrap_or(false)
}

/// Starts listening (the mic button): `dictation:text` while it hears, `dictation:end` when
/// it stops. A second press stops it (`dictation_stop`).
#[tauri::command]
pub fn dictation_start(app: AppHandle, dictation: State<'_, Dictation>) {
    let mut running = dictation.running.lock().unwrap();
    if running.is_some() {
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    *running = Some(stop.clone());
    std::thread::spawn(move || {
        let result = listen(microphone, &stop, |text, settled| {
            let _ = app.emit_to(WINDOW_LABEL, EVENT_TEXT, Heard { text, r#final: settled });
        });
        *app.state::<Dictation>().running.lock().unwrap() = None;
        let _ = app.emit_to(WINDOW_LABEL, EVENT_END, End { error: result.err() });
    });
}

#[tauri::command]
pub fn dictation_stop(dictation: State<'_, Dictation>) {
    if let Some(stop) = dictation.running.lock().unwrap().as_ref() {
        stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Media::Audio::{WAVE_FORMAT_PCM, WAVEFORMATEX};
    use windows::Win32::Media::Speech::{
        ISpStream, ISpVoice, SPCAT_VOICES, SPF_DEFAULT, SPFM_CREATE_ALWAYS, SPFM_OPEN_READONLY, SpStream, SpVoice,
    };
    use windows::core::GUID;

    /// `SPDFID_WaveFormatEx` of sapi.h: a stream described by a WAVEFORMATEX.
    const WAVE_FORMAT_EX: GUID = GUID::from_u128(0xc31adbae_527f_4ff5_a230_f62bb61ff70c);

    #[test]
    fn the_interest_bits_follow_sapi_h() {
        assert_eq!(interest(&[]), (1 << 30) | (1 << 33));
        assert_eq!(interest(&[SPEI_RECOGNITION]), (1 << 30) | (1 << 33) | (1 << 38));
    }

    /// Windows' French voice says a sentence into a WAV file, and the French recognizer
    /// writes it back, all on this PC. Skipped where either is missing.
    #[test]
    fn french_speech_turns_into_text_on_this_pc() {
        // SAFETY: COM joined in the multithreaded apartment; SAPI objects used as documented.
        let path = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            if recognizer(Lang::Fr).ok().flatten().is_none() {
                return;
            }
            let Some((voice_token, _)) = crate::voice::speech_token(SPCAT_VOICES, Lang::Fr).unwrap() else { return };
            let path = std::env::temp_dir().join(format!("ilot-dictee-{}.wav", std::process::id()));
            let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain([0]).collect();
            let format = WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_PCM as u16,
                nChannels: 1,
                nSamplesPerSec: 16_000,
                nAvgBytesPerSec: 32_000,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            let format_id = WAVE_FORMAT_EX;
            let stream: ISpStream = CoCreateInstance(&SpStream, None, CLSCTX_INPROC_SERVER).unwrap();
            stream
                .BindToFile(PCWSTR(wide.as_ptr()), SPFM_CREATE_ALWAYS, Some(&raw const format_id), Some(&raw const format), 0)
                .unwrap();
            let voice: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_INPROC_SERVER).unwrap();
            voice.SetVoice(&voice_token).unwrap();
            voice.SetOutput(&stream, true).unwrap();
            voice.Speak(w!("Bonjour. Je voudrais un résumé de ce document."), SPF_DEFAULT.0 as u32, None).unwrap();
            let _ = voice.WaitUntilDone(15_000);
            drop(voice);
            stream.Close().unwrap();
            path
        };
        let stop = AtomicBool::new(false);
        let mut heard = String::new();
        let mut guesses = 0;
        let file = |recognizer: &ISpRecognizer| -> Result<(), String> {
            // SAFETY: COM is joined by `listen`; the stream reads the WAV file just written.
            unsafe {
                let stream: ISpStream = CoCreateInstance(&SpStream, None, CLSCTX_INPROC_SERVER).unwrap();
                let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain([0]).collect();
                stream.BindToFile(PCWSTR(wide.as_ptr()), SPFM_OPEN_READONLY, None, None, 0).unwrap();
                recognizer.SetInput(&stream, true).map_err(|e| e.to_string())
            }
        };
        listen(file, &stop, |text, settled| {
            if settled {
                heard.push_str(text);
                heard.push(' ');
            } else {
                guesses += 1;
            }
        })
        .unwrap();
        std::fs::remove_file(&path).unwrap();
        let heard = heard.to_lowercase();
        assert!(heard.contains("bonjour") || heard.contains("résumé") || heard.contains("document"), "{heard:?}");
        assert!(guesses > 0, "words show while they are spoken");
    }

    #[test]
    fn a_missing_engine_is_said_in_the_language_of_the_moment() {
        assert_eq!(unavailable(Lang::En), "Dictée indisponible : la reconnaissance vocale anglaise de Windows n'est pas installée.");
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(unavailable(Lang::En), "Dictation unavailable: Windows' English speech recognition is not installed.");
        // SAFETY: COM joined in the multithreaded apartment, then read-only token queries.
        let english = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            recognizer(Lang::En).unwrap().is_some()
        };
        assert_eq!(available(), english, "the mic shows only with the engine of the interface's language");
    }
}
