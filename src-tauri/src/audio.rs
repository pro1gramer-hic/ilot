//! The equalizer's level: the peak of the default output, read with
//! `IAudioMeterInformation::GetPeakValue` (a number from 0 to 1 for the last device
//! period, https://learn.microsoft.com/en-us/windows/win32/api/endpointvolume/nf-endpointvolume-iaudiometerinformation-getpeakvalue)
//! about 30 times a second while the interface asks for it, and sent as `audio:level`
//! to the island only. îlot opens no capture stream: it never records the sound, nor
//! reads or analyses its samples, only this one number Windows already computes.

use crate::window::WINDOW_LABEL;
use std::sync::Mutex;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize};

pub const EVENT_LEVEL: &str = "audio:level";
/// About 30 readings a second.
const PERIOD: Duration = Duration::from_millis(33);
/// The default output can change (headphones plugged in): it is looked up again this
/// often, in readings (about every 2 s).
const REFRESH_EVERY: u32 = 60;
/// Below this, the output is silent.
const SILENCE: f32 = 0.001;

/// Where the peak comes from: Windows, or a fake in the tests.
pub trait PeakSource {
    fn peak(&mut self) -> Option<f32>;
}

/// What is sent: `None` while silence goes on (once silent, nothing more is sent until
/// sound comes back), else the peak, between 0 and 1, to three decimals.
pub fn level_to_send(previous: f32, peak: f32) -> Option<f32> {
    let peak = if peak.is_finite() { peak.clamp(0.0, 1.0) } else { 0.0 };
    if peak < SILENCE && previous < SILENCE {
        return None;
    }
    Some((peak * 1000.0).round() / 1000.0)
}

struct Running {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

/// The reading thread, started and stopped by `audio_meter`.
#[derive(Default)]
pub struct AudioMeter {
    running: Mutex<Option<Running>>,
}

impl AudioMeter {
    /// Starts reading, unless it already does. The source is opened on the reading
    /// thread (COM objects stay on the thread that made them).
    pub fn start<S, O, K>(&self, open: O, mut sink: K) -> std::io::Result<()>
    where
        S: PeakSource,
        O: FnOnce() -> S + Send + 'static,
        K: FnMut(f32) + Send + 'static,
    {
        let mut running = self.running.lock().unwrap();
        if running.as_ref().is_some_and(|r| !r.thread.is_finished()) {
            return Ok(());
        }
        let (stop, stopped) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new().name("ilot-audio-meter".into()).spawn(move || {
            let mut source = open();
            let mut previous = 0.0;
            // Any message, or the sender dropped, stops the thread at once.
            while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(PERIOD) {
                let peak = source.peak().unwrap_or(0.0);
                if let Some(level) = level_to_send(previous, peak) {
                    sink(level);
                }
                previous = peak;
            }
        })?;
        *running = Some(Running { stop, thread });
        Ok(())
    }

    /// Stops reading and waits for the thread: no level is sent once this returns.
    pub fn stop(&self) {
        let running = self.running.lock().unwrap().take();
        if let Some(Running { stop, thread }) = running {
            drop(stop);
            let _ = thread.join();
        }
    }
}

/// The meter of the default output (the one the volume wheel sets), found again every
/// `REFRESH_EVERY` readings.
pub struct WindowsPeak {
    meter: Option<IAudioMeterInformation>,
    readings: u32,
    // Last field: dropped after the meter, as COM asks.
    com: ComThread,
}

impl WindowsPeak {
    pub fn open() -> Self {
        Self { meter: None, readings: 0, com: ComThread::init() }
    }
}

impl PeakSource for WindowsPeak {
    fn peak(&mut self) -> Option<f32> {
        if self.readings.is_multiple_of(REFRESH_EVERY) {
            self.meter = self.com.default_meter().ok();
        }
        self.readings = self.readings.wrapping_add(1);
        // SAFETY: plain getter on a live interface, on the thread that created it.
        let peak = self.meter.as_ref().and_then(|meter| unsafe { meter.GetPeakValue() }.ok());
        if peak.is_none() {
            self.meter = None;
        }
        peak
    }
}

/// COM for the reading thread, released when the thread ends.
struct ComThread {
    initialized: bool,
}

impl ComThread {
    fn init() -> Self {
        // SAFETY: initializes COM for the current thread only; balanced in `drop`.
        let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        Self { initialized }
    }

    fn default_meter(&self) -> windows::core::Result<IAudioMeterInformation> {
        // SAFETY: COM is initialized on this thread (`init`); the interfaces live as long
        // as the returned value.
        unsafe {
            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            devices.GetDefaultAudioEndpoint(eRender, eConsole)?.Activate(CLSCTX_ALL, None)
        }
    }
}

impl Drop for ComThread {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: balances the successful `CoInitializeEx` of this thread.
            unsafe { CoUninitialize() };
        }
    }
}

/// `audio_meter(true)` starts sending `audio:level` `{ peak }` to the island,
/// `audio_meter(false)` stops it at once.
#[tauri::command]
pub async fn audio_meter(on: bool, app: AppHandle) -> Result<(), String> {
    let meter = app.state::<AudioMeter>();
    if !on {
        meter.stop();
        return Ok(());
    }
    let target = app.clone();
    let sink = move |peak: f32| {
        let _ = target.emit_to(WINDOW_LABEL, EVENT_LEVEL, serde_json::json!({ "peak": peak }));
    };
    meter.start(WindowsPeak::open, sink).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fake {
        peaks: Vec<f32>,
        reads: Arc<AtomicUsize>,
    }

    impl PeakSource for Fake {
        fn peak(&mut self) -> Option<f32> {
            let n = self.reads.fetch_add(1, Ordering::SeqCst);
            Some(self.peaks.get(n).copied().unwrap_or(0.5))
        }
    }

    /// Waits until `done` holds, 10 s at most: other tests may keep the machine busy.
    fn wait_for(done: impl Fn() -> bool) -> bool {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !done() {
            if std::time::Instant::now() > deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }

    #[test]
    fn silence_is_sent_once_then_nothing() {
        assert_eq!(level_to_send(0.0, 0.0), None);
        assert_eq!(level_to_send(0.4, 0.0), Some(0.0), "the drop to silence is sent");
        assert_eq!(level_to_send(0.0, 0.3141), Some(0.314));
        assert_eq!(level_to_send(0.0, 1.7), Some(1.0));
        assert_eq!(level_to_send(0.5, f32::NAN), Some(0.0));
        assert_eq!(level_to_send(0.0, -0.2), None);
    }

    #[test]
    fn the_meter_reads_until_stopped_and_only_once() {
        let meter = AudioMeter::default();
        let reads = Arc::new(AtomicUsize::new(0));
        let sent = Arc::new(Mutex::new(Vec::new()));
        let (r, s) = (reads.clone(), sent.clone());
        meter.start(move || Fake { peaks: vec![0.0, 0.0, 0.25], reads: r }, move |p| s.lock().unwrap().push(p)).unwrap();
        let other = Arc::new(AtomicUsize::new(0));
        let o = other.clone();
        meter.start(move || Fake { peaks: Vec::new(), reads: o }, |_| {}).unwrap();
        assert!(wait_for(|| reads.load(Ordering::SeqCst) >= 4), "it keeps reading");
        meter.stop();
        assert!(meter.running.lock().unwrap().is_none());
        let after_stop = reads.load(Ordering::SeqCst);
        assert_eq!(other.load(Ordering::SeqCst), 0, "a second start does not start a second thread");
        let sent = sent.lock().unwrap().clone();
        assert_eq!(sent[0], 0.25, "silence first: nothing sent");
        std::thread::sleep(PERIOD * 3);
        assert_eq!(reads.load(Ordering::SeqCst), after_stop, "nothing is read once stopped");
    }

    #[test]
    fn the_meter_can_start_again_after_a_stop() {
        let meter = AudioMeter::default();
        meter.stop();
        let reads = Arc::new(AtomicUsize::new(0));
        let r = reads.clone();
        meter.start(move || Fake { peaks: Vec::new(), reads: r }, |_| {}).unwrap();
        meter.stop();
        let r = reads.clone();
        meter.start(move || Fake { peaks: Vec::new(), reads: r }, |_| {}).unwrap();
        assert!(wait_for(|| reads.load(Ordering::SeqCst) >= 1), "reading again");
        meter.stop();
    }

    #[test]
    fn the_default_output_has_a_peak_level() {
        // Every Windows desktop has a default output; its peak reads between 0 and 1.
        let mut source = WindowsPeak::open();
        let peak = source.peak().expect("default output meter");
        assert!((0.0..=1.0).contains(&peak));
    }
}
