//! Listening for « OK Mochi » with the speech recognizer of Windows (SAPI 5.4, in-process,
//! engine of the grammar's language), on a command grammar only: the recognizer can hear
//! nothing but the keyword and the phrases the interface listed, and does it on the PC.
//! A language without its engine on the PC is said so: nothing goes online. No dictation
//! grammar is ever loaded (Windows' dictation and topic grammars send the audio to an
//! online service), and audio is not retained (no `SPAO_RETAIN_AUDIO`): only the words
//! recognized leave this module.
//! https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee125077(v=vs.85)
//! https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms718871(v=vs.85) (ISpRecognizer::SetInput)

use super::{Grammar, quiet_now};
use crate::locale::{Lang, Text, tr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use windows::Win32::Media::Speech::{
    ISpObjectToken, ISpObjectTokenCategory, ISpRecoContext, ISpRecoGrammar, ISpRecoResult, ISpRecognizer, SPCAT_AUDIOIN,
    SPCAT_RECOGNIZERS, SPEI_RECOGNITION, SPEI_RESERVED1, SPEI_RESERVED2, SPET_LPARAM_IS_OBJECT, SPET_LPARAM_IS_POINTER,
    SPET_LPARAM_IS_STRING, SPET_LPARAM_IS_TOKEN, SPEVENT, SPRAF_TopLevel, SPRS_ACTIVE, SPRS_INACTIVE, SPRST_ACTIVE,
    SPRST_INACTIVE_WITH_PURGE, SPSTATEHANDLE, SPWT_LEXICAL, SP_NORMAL_CONFIDENCE, SpInprocRecognizer, SpObjectToken,
    SpObjectTokenCategory,
};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize};
use windows::core::{HSTRING, IUnknown, Interface, PCWSTR, PWSTR, w};

/// After the keyword alone, the phrases are heard for this long.
const AWAKE: Duration = Duration::from_secs(8);
const WAKE_RULE: u32 = 1;
const COMMAND_RULE: u32 = 2;
/// `SP_GETWHOLEPHRASE` of sapi.h.
const WHOLE_PHRASE: u32 = u32::MAX;

/// What the recognizer heard: the keyword (said before the phrase, or alone), and the
/// phrase that followed it, empty when there was none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heard {
    pub wake: bool,
    pub phrase: String,
}

/// The listening thread; dropping it closes the microphone (within 200 ms, without
/// waiting for it: the caller may be the interface's thread).
pub struct Listener {
    stop: Arc<AtomicBool>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Splits what was recognized into the keyword and what followed: "OK Mochi coupe le son"
/// → (true, "coupe le son"). Case and spaces do not matter.
pub fn split_wake(text: &str, wakes: &[String]) -> Heard {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = text.to_lowercase();
    for wake in wakes {
        let wake = wake.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        if lower == wake {
            return Heard { wake: true, phrase: String::new() };
        }
        if let Some(rest) = lower.strip_prefix(&format!("{wake} ")) {
            return Heard { wake: true, phrase: rest.to_owned() };
        }
    }
    Heard { wake: false, phrase: lower }
}

/// The SAPI event interest mask for one event (`SPFEI` of sapi.h).
const fn interest(event: i32) -> u64 {
    (1 << event) | (1 << SPEI_RESERVED1.0) | (1 << SPEI_RESERVED2.0)
}

/// The first token of `category` (recognizers, voices) that speaks `lang`, and the
/// language id it was found by. SAPI tokens name their language as `Language=40C`.
///
/// # Safety
/// COM must be joined on this thread.
pub(crate) unsafe fn speech_token(category: PCWSTR, lang: Lang) -> windows::core::Result<Option<(ISpObjectToken, u16)>> {
    unsafe {
        let tokens: ISpObjectTokenCategory = CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)?;
        tokens.SetId(category, false)?;
        for lcid in lang.speech_lcids() {
            let list = tokens.EnumTokens(&HSTRING::from(format!("Language={lcid:X}")), PCWSTR::null())?;
            let mut count = 0;
            list.GetCount(&mut count)?;
            if count > 0 {
                return list.Item(0).map(|token| Some((token, *lcid)));
            }
        }
        Ok(None)
    }
}

/// The microphone Windows uses by default for speech.
unsafe fn default_microphone() -> windows::core::Result<ISpObjectToken> {
    unsafe {
        let inputs: ISpObjectTokenCategory = CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)?;
        inputs.SetId(SPCAT_AUDIOIN, false)?;
        let id = inputs.GetDefaultTokenId()?;
        let token: ISpObjectToken = CoCreateInstance(&SpObjectToken, None, CLSCTX_ALL)?;
        let set = token.SetId(PCWSTR::null(), PCWSTR(id.0), false);
        CoTaskMemFree(Some(id.0 as *const _));
        set.map(|()| token)
    }
}

/// The two rules: « wake » is the keyword, alone or followed by a phrase; « command » is a
/// phrase alone, heard only for a moment after the keyword alone.
unsafe fn build(grammar: &ISpRecoGrammar, words: &Grammar, lcid: u16) -> windows::core::Result<()> {
    unsafe {
        grammar.ResetGrammar(lcid)?;
        let end = SPSTATEHANDLE::default();
        let mut wake = SPSTATEHANDLE::default();
        grammar.GetRule(w!("wake"), WAKE_RULE, SPRAF_TopLevel.0 as u32, true, &mut wake)?;
        let mut after = SPSTATEHANDLE::default();
        grammar.CreateNewState(wake, &mut after)?;
        for keyword in &words.wake {
            grammar.AddWordTransition(wake, after, &HSTRING::from(keyword), w!(" "), SPWT_LEXICAL, 1.0, std::ptr::null())?;
        }
        grammar.AddWordTransition(after, end, PCWSTR::null(), PCWSTR::null(), SPWT_LEXICAL, 1.0, std::ptr::null())?;
        let mut command = SPSTATEHANDLE::default();
        grammar.GetRule(w!("command"), COMMAND_RULE, SPRAF_TopLevel.0 as u32, true, &mut command)?;
        for phrase in &words.phrases {
            let phrase = HSTRING::from(phrase);
            grammar.AddWordTransition(after, end, &phrase, w!(" "), SPWT_LEXICAL, 1.0, std::ptr::null())?;
            grammar.AddWordTransition(command, end, &phrase, w!(" "), SPWT_LEXICAL, 1.0, std::ptr::null())?;
        }
        grammar.Commit(0)?;
        grammar.SetRuleState(w!("wake"), std::ptr::null_mut(), SPRS_ACTIVE)?;
        grammar.SetRuleState(w!("command"), std::ptr::null_mut(), SPRS_INACTIVE)
    }
}

/// Everything the thread keeps alive while it listens (released in reverse order).
struct Session {
    grammar: ISpRecoGrammar,
    context: ISpRecoContext,
    recognizer: ISpRecognizer,
}

unsafe fn open(words: &Grammar) -> Result<Session, String> {
    unsafe {
        let failed = |e: windows::core::Error| tr(Text::VoiceFailedToStart { error: &e.message() });
        let recognizer: ISpRecognizer = CoCreateInstance(&SpInprocRecognizer, None, CLSCTX_ALL).map_err(failed)?;
        let (engine, lcid) =
            speech_token(SPCAT_RECOGNIZERS, words.lang).map_err(failed)?.ok_or(Text::VoiceNoRecognizer { lang: words.lang })?;
        recognizer.SetRecognizer(&engine).map_err(failed)?;
        let microphone = Text::VoiceNoMicrophone;
        let input = default_microphone().map_err(|_| microphone)?;
        recognizer.SetInput(&input, true).map_err(|_| microphone)?;
        let context = recognizer.CreateRecoContext().map_err(failed)?;
        context.SetNotifyWin32Event().map_err(failed)?;
        let wanted = interest(SPEI_RECOGNITION.0);
        context.SetInterest(wanted, wanted).map_err(failed)?;
        let grammar = context.CreateGrammar(1).map_err(failed)?;
        build(&grammar, words, lcid).map_err(failed)?;
        recognizer.SetRecoState(SPRST_ACTIVE).map_err(|_| microphone)?;
        Ok(Session { grammar, context, recognizer })
    }
}

/// The words of a recognition, its rule, and whether the engine is sure enough.
unsafe fn read(result: &ISpRecoResult) -> Option<(String, u32)> {
    unsafe {
        let mut text = PWSTR::null();
        result.GetText(WHOLE_PHRASE, WHOLE_PHRASE, true, &mut text, None).ok()?;
        let words = text.to_string().unwrap_or_default();
        CoTaskMemFree(Some(text.0 as *const _));
        let phrase = result.GetPhrase().ok()?;
        let rule = (*phrase).Base.Rule.ulId;
        let sure = i32::from((*phrase).Base.Rule.Confidence) >= SP_NORMAL_CONFIDENCE as i32;
        CoTaskMemFree(Some(phrase as *const _));
        sure.then_some((words, rule))
    }
}

/// Takes the waiting events out of the context, frees what they carry, and returns what
/// was recognized.
unsafe fn drain(context: &ISpRecoContext) -> Vec<(String, u32)> {
    let mut heard = Vec::new();
    loop {
        let mut event = SPEVENT::default();
        let mut fetched = 0;
        // SAFETY: one event slot, and the count of events written.
        if unsafe { context.GetEvents(1, &mut event, &mut fetched) }.is_err() || fetched == 0 {
            return heard;
        }
        let id = event._bitfield & 0xffff;
        let kind = (event._bitfield >> 16) & 0xffff;
        let param = event.lParam.0 as *mut core::ffi::c_void;
        if param.is_null() {
            continue;
        }
        if kind == SPET_LPARAM_IS_OBJECT.0 || kind == SPET_LPARAM_IS_TOKEN.0 {
            // SAFETY: the event owns one reference to this object; taking it releases it.
            let object = unsafe { IUnknown::from_raw(param) };
            if id == SPEI_RECOGNITION.0
                && let Ok(result) = object.cast::<ISpRecoResult>()
                && let Some(words) = unsafe { read(&result) }
            {
                heard.push(words);
            }
        } else if kind == SPET_LPARAM_IS_POINTER.0 || kind == SPET_LPARAM_IS_STRING.0 {
            // SAFETY: SAPI allocated it with CoTaskMemAlloc for us to free.
            unsafe { CoTaskMemFree(Some(param as *const _)) };
        }
    }
}

impl Listener {
    /// Opens the microphone and listens until dropped. `heard` gets each recognition;
    /// recognitions made while Mochi speaks (`quiet_until`) are his own voice: dropped.
    pub fn start(
        words: Grammar,
        quiet_until: Arc<AtomicU64>,
        heard: impl Fn(Heard) + Send + 'static,
    ) -> Result<Listener, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready, started) = mpsc::sync_channel(1);
        let halt = stop.clone();
        let thread = std::thread::Builder::new()
            .name("ilot-voice".into())
            .spawn(move || {
                // SAFETY: COM is set up for this thread and closed when it ends; every
                // interface below is used on this thread only.
                unsafe {
                    if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                        let _ = ready.send(Err(tr(Text::VoiceNotStarted)));
                        return;
                    }
                    match open(&words) {
                        Ok(session) => {
                            let _ = ready.send(Ok(()));
                            listen(&session, &words, &halt, &quiet_until, &heard);
                            let _ = session.recognizer.SetRecoState(SPRST_INACTIVE_WITH_PURGE);
                            drop(session);
                        }
                        Err(message) => {
                            let _ = ready.send(Err(message));
                        }
                    }
                    CoUninitialize();
                }
            })
            .map_err(|e| e.to_string())?;
        match started.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(Listener { stop }),
            Ok(Err(message)) => {
                let _ = thread.join();
                Err(message)
            }
            Err(_) => {
                stop.store(true, Ordering::Relaxed);
                Err(tr(Text::VoiceNotAnswering))
            }
        }
    }
}

unsafe fn listen(session: &Session, words: &Grammar, stop: &AtomicBool, quiet_until: &AtomicU64, heard: &impl Fn(Heard)) {
    let mut awake_until: Option<Instant> = None;
    let set_awake = |on: bool| {
        let state = if on { SPRS_ACTIVE } else { SPRS_INACTIVE };
        // SAFETY: the grammar lives as long as the session.
        let _ = unsafe { session.grammar.SetRuleState(w!("command"), std::ptr::null_mut(), state) };
    };
    while !stop.load(Ordering::Relaxed) {
        // SAFETY: the context lives as long as the session.
        let _ = unsafe { session.context.WaitForNotifyEvent(200) };
        if awake_until.is_some_and(|until| Instant::now() > until) {
            awake_until = None;
            set_awake(false);
        }
        // SAFETY: as above.
        for (text, rule) in unsafe { drain(&session.context) } {
            if quiet_now(quiet_until) {
                continue;
            }
            let mut words_heard = split_wake(&text, &words.wake);
            if rule == COMMAND_RULE {
                words_heard.wake = false;
            }
            let alone = rule == WAKE_RULE && words_heard.phrase.is_empty();
            awake_until = alone.then(|| Instant::now() + AWAKE);
            set_awake(alone);
            heard(words_heard);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keyword_is_split_from_the_phrase() {
        let wakes = vec!["OK Mochi".to_owned(), "OK Pixel".to_owned()];
        assert_eq!(split_wake("OK Mochi", &wakes), Heard { wake: true, phrase: String::new() });
        assert_eq!(split_wake("ok  mochi Coupe le son", &wakes), Heard { wake: true, phrase: "coupe le son".into() });
        assert_eq!(split_wake("OK Pixel ouvre le chat", &wakes), Heard { wake: true, phrase: "ouvre le chat".into() });
        assert_eq!(split_wake("Ouvre le chat", &wakes), Heard { wake: false, phrase: "ouvre le chat".into() });
        assert!(!split_wake("OK Mochiko", &wakes).wake, "a longer word is not the keyword");
    }

    #[test]
    fn events_are_asked_with_sapi_flag_bits() {
        assert_eq!(interest(SPEI_RECOGNITION.0), (1 << 38) | (1 << 30) | (1 << 33));
    }

    #[test]
    fn engines_are_looked_for_in_the_grammar_s_language() {
        // SAFETY: COM joined in the multithreaded apartment, then read-only token queries.
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, COINIT_MULTITHREADED);
            for lang in Lang::ALL {
                if let Some((_, lcid)) = speech_token(SPCAT_RECOGNIZERS, lang).unwrap() {
                    assert!(lang.speech_lcids().contains(&lcid), "{lang:?}: {lcid:X}");
                }
            }
        }
        crate::locale::speak(Lang::En);
        assert_eq!(tr(Text::VoiceNoRecognizer { lang: Lang::En }), "No English speech recognizer is installed on this PC.");
    }
}
