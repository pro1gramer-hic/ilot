//! Mochi's voice: a voice installed with Windows in the language of the interface, through
//! SAPI (`ISpVoice`), on the PC. No text leaves it for a speech service.
//! https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee125024(v=vs.85) (ISpVoice::Speak)

use super::QUIET_AFTER_MS;
use super::listen::speech_token;
use crate::events::now_ms;
use crate::locale::{Lang, Text};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::Win32::Media::Speech::{
    ISpVoice, SPCAT_VOICES, SPF_ASYNC, SPF_IS_NOT_XML, SPF_PURGEBEFORESPEAK, SPRS_DONE, SPVOICESTATUS, SpVoice,
};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize};
use windows::core::{HSTRING, PCWSTR, PWSTR};

/// Characters Mochi says at most in one go.
pub const MAX_SPOKEN_CHARS: usize = 600;

enum Order {
    Say(String, Lang),
    Hush,
}

/// The thread that speaks, started on the first sentence, and the languages it found no
/// voice for on this PC.
#[derive(Default)]
pub struct Speaker {
    orders: Mutex<Option<Sender<Order>>>,
    voiceless: Arc<Mutex<Vec<Lang>>>,
}

/// What Mochi says, cleaned: control characters out, cut to its limit.
pub fn spoken(text: &str) -> String {
    let clean: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    clean.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MAX_SPOKEN_CHARS).collect()
}

impl Speaker {
    /// Says `text` aloud in `lang`, cutting off what was being said. On a PC without a
    /// voice of that language nothing is heard (the island shows the text anyway), and
    /// once known it is said so.
    pub fn say(&self, text: &str, lang: Lang, quiet_until: &Arc<AtomicU64>) -> Result<(), String> {
        let text = spoken(text);
        if text.is_empty() {
            return Ok(());
        }
        if self.voiceless.lock().unwrap().contains(&lang) {
            return Err(Text::NoVoice { lang }.into());
        }
        self.send(Order::Say(text, lang), quiet_until)
    }

    pub fn hush(&self, quiet_until: &Arc<AtomicU64>) {
        let _ = self.send(Order::Hush, quiet_until);
    }

    fn send(&self, order: Order, quiet_until: &Arc<AtomicU64>) -> Result<(), String> {
        let mut orders = self.orders.lock().unwrap();
        if orders.is_none() {
            let (send, receive) = mpsc::channel();
            let quiet = quiet_until.clone();
            let voiceless = self.voiceless.clone();
            std::thread::Builder::new()
                .name("ilot-voice-out".into())
                .spawn(move || speak(&receive, &quiet, &voiceless))
                .map_err(|e| e.to_string())?;
            *orders = Some(send);
        }
        let sent = orders.as_ref().map(|o| o.send(order));
        if !matches!(sent, Some(Ok(()))) {
            *orders = None;
            return Err(Text::VoiceSilent.into());
        }
        Ok(())
    }
}

fn speaking(voice: &ISpVoice) -> bool {
    let mut status = SPVOICESTATUS::default();
    // SAFETY: a valid status structure; the last bookmark is not asked for.
    let read = unsafe { voice.GetStatus(&mut status, std::ptr::null_mut::<PWSTR>()) };
    read.is_ok() && status.dwRunningState & SPRS_DONE.0 as u32 == 0
}

fn speak(orders: &mpsc::Receiver<Order>, quiet_until: &AtomicU64, voiceless: &Mutex<Vec<Lang>>) {
    // SAFETY: COM is set up for this thread, and the voice is used on it only.
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            return;
        }
        if let Ok(voice) = CoCreateInstance::<_, ISpVoice>(&SpVoice, None, CLSCTX_ALL) {
            run(&voice, orders, quiet_until, voiceless);
        }
        CoUninitialize();
    }
}

/// Gives `voice` the voice of `lang`; false when this PC has none.
fn speak_in(voice: &ISpVoice, lang: Lang) -> bool {
    // SAFETY: COM is joined by `speak` on this thread, where the voice lives.
    unsafe { matches!(speech_token(SPCAT_VOICES, lang), Ok(Some((token, _))) if voice.SetVoice(&token).is_ok()) }
}

fn run(voice: &ISpVoice, orders: &mpsc::Receiver<Order>, quiet_until: &AtomicU64, voiceless: &Mutex<Vec<Lang>>) {
    let purge = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0 | SPF_IS_NOT_XML.0) as u32;
    let mut busy = false;
    let mut speaking_in = None;
    loop {
        let order = if busy { orders.recv_timeout(Duration::from_millis(100)) } else { orders.recv().map_err(|_| RecvTimeoutError::Disconnected) };
        match order {
            Ok(Order::Say(_, lang)) if speaking_in != Some(lang) && !speak_in(voice, lang) => {
                voiceless.lock().unwrap().push(lang);
                continue;
            }
            // SAFETY (both): the voice lives on this thread; an empty text with the purge
            // flag only stops what is being said.
            Ok(Order::Say(text, lang)) => {
                speaking_in = Some(lang);
                busy = unsafe { voice.Speak(&HSTRING::from(text), purge, None) }.is_ok();
            }
            Ok(Order::Hush) => busy = unsafe { voice.Speak(PCWSTR::null(), purge, None) }.is_ok(),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if busy {
            busy = speaking(voice);
            // The microphone may hear Mochi: what it hears meanwhile is ignored.
            quiet_until.store(now_ms() + QUIET_AFTER_MS, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoken_text_is_one_clean_line_within_limits() {
        assert_eq!(spoken("  Bonjour\n\tle\u{7} monde  "), "Bonjour le monde");
        assert_eq!(spoken(&"a".repeat(2000)).chars().count(), MAX_SPOKEN_CHARS);
        assert_eq!(spoken("\n\t"), "");
    }

    #[test]
    fn a_missing_voice_is_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let speaker = Speaker::default();
        speaker.voiceless.lock().unwrap().push(Lang::En);
        let quiet = Arc::new(AtomicU64::new(0));
        assert_eq!(speaker.say("Hello", Lang::En, &quiet).unwrap_err(), "No English voice is installed on this PC.");
        assert_eq!(speaker.say("  ", Lang::En, &quiet), Ok(()), "nothing to say, nothing to refuse");
    }
}
