// The output's peak level, shared by every equalizer on screen (docs/09-DEMO-FINALE.md,
// section 10). The core reads only the peak of the default output (IAudioMeterInformation),
// never the sound itself, and only while an equalizer asks for it: the first one to show
// turns the meter on, the last one to hide turns it off. The browser preview has no
// sound: a made-up level stands in, so the equalizer can be seen and tested.

import { audioMeter, inTauri, onCoreEvent } from "../../lib/ipc";

/** The made-up level of the preview: a beat about twice a second over a slow swell. */
export function previewPeak(ms: number): number {
  const beat = Math.max(0, Math.cos((ms / 1000) * Math.PI * 4.2)) ** 6;
  const swell = 0.5 + 0.5 * Math.sin((ms / 1000) * 0.9);
  return 0.05 + 0.3 * swell + 0.6 * beat;
}

class Meter {
  /** Latest peak, 0 to 1 (linear). A plain field: the equalizers' frame loops read it. */
  peak = 0;
  #listeners = 0;
  #stop: (() => void) | undefined;

  /** Starts reading while at least one equalizer listens; returns the function that stops. */
  listen(): () => void {
    if (this.#listeners++ === 0) this.#start();
    let done = false;
    return () => {
      if (done) return;
      done = true;
      if (--this.#listeners === 0) this.#end();
    };
  }

  #start(): void {
    if (!inTauri) {
      const timer = setInterval(() => (this.peak = previewPeak(performance.now())), 33);
      this.#stop = () => clearInterval(timer);
      return;
    }
    const unlisten = onCoreEvent("audio:level", ({ peak }) => (this.peak = peak));
    void audioMeter(true);
    this.#stop = () => {
      unlisten();
      void audioMeter(false);
    };
  }

  #end(): void {
    this.#stop?.();
    this.#stop = undefined;
    this.peak = 0;
  }
}

export const meter = new Meter();
