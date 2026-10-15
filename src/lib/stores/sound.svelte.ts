// îlot's sounds at play (prompt 12, lot 1): `sound.play("finish")` where something
// happens. The rules (catalog.ts) decide whether it may sound now; the samples come from
// the synthesizer, or from the user's replacement file once decoded; the core plays the
// WAV (src-tauri/src/sounds.rs), so no sound waits for a click in the page. The browser
// preview plays them with Web Audio instead.

import { prefersReducedMotion } from "svelte/motion";
import { type SoundFile, inTauri, openSoundsFolder, playSound, soundFile, soundOverrides } from "../ipc";
import { RECIPES, REPEAT_MS, type SoundId, mayPlay } from "../sound/catalog";
import { SAMPLE_RATE, encodeWav, gainOf, mixdown, render } from "../sound/synth";
import { settings } from "./settings.svelte";

/** A replacement file plays at most this long. */
const MAX_REPLACEMENT_S = 3;
/** Sounds remembered for the preview's tests (memory only). */
const HISTORY = 20;

class SoundStore {
  /** Replacement files found in the sounds folder. */
  files = $state<SoundFile[]>([]);
  /** Replacements that could not be decoded: their own sound plays instead. */
  broken = $state<SoundId[]>([]);
  /** The last sounds played, oldest first. */
  played = $state<SoundId[]>([]);
  error = $state<string | null>(null);
  #samples = new Map<string, Promise<Float32Array>>();
  #last = new Map<SoundId, number>();
  #context: AudioContext | null = null;

  /** Reads the sounds folder again (Réglages › Son, and at launch). */
  async refresh(): Promise<void> {
    try {
      this.files = await soundOverrides();
      this.broken = [];
      this.#samples.clear();
    } catch (error) {
      this.error = String(error);
    }
  }

  /** Plays a sound if the rules allow it now. `preview`: Écouter in Réglages. */
  play(id: SoundId, { preview = false }: { preview?: boolean } = {}): void {
    const current = settings.value;
    const conditions = {
      muted: current.sound.muted,
      volume: current.sound.volume,
      doNotDisturb: current.doNotDisturb,
      reducedMotion: prefersReducedMotion.current,
    };
    if (!mayPlay(id, conditions, preview)) return;
    const now = performance.now();
    if (!preview && now - (this.#last.get(id) ?? -Infinity) < REPEAT_MS) return;
    this.#last.set(id, now);
    this.played = [...this.played.slice(1 - HISTORY), id];
    void this.#output(id, gainOf(current.sound.volume));
  }

  async openFolder(): Promise<void> {
    try {
      await openSoundsFolder();
      this.error = null;
    } catch (error) {
      this.error = String(error);
    }
  }

  /** The replacement of a sound, when the folder holds one. */
  fileOf(id: SoundId): SoundFile | undefined {
    return this.files.find((f) => f.id === id);
  }

  #samplesOf(id: SoundId): Promise<Float32Array> {
    const file = this.files.find((f) => f.id === id && f.usable);
    const replaced = file !== undefined && !this.broken.includes(id);
    const key = replaced ? `${id}:${file.file}` : id;
    let samples = this.#samples.get(key);
    if (!samples) {
      samples = replaced ? this.#decode(id) : Promise.resolve(render(RECIPES[id]));
      this.#samples.set(key, samples);
    }
    return samples;
  }

  /** Decodes a replacement (wav, mp3 or ogg) to îlot's rate; its own sound if it fails. */
  async #decode(id: SoundId): Promise<Float32Array> {
    try {
      const bytes = await soundFile(id);
      const buffer = await new OfflineAudioContext(1, 1, SAMPLE_RATE).decodeAudioData(bytes);
      const channels = Array.from({ length: buffer.numberOfChannels }, (_, i) => buffer.getChannelData(i));
      return mixdown(channels, MAX_REPLACEMENT_S * SAMPLE_RATE);
    } catch {
      this.broken = [...this.broken, id];
      return render(RECIPES[id]);
    }
  }

  async #output(id: SoundId, gain: number): Promise<void> {
    const samples = await this.#samplesOf(id);
    try {
      if (inTauri) {
        await playSound(encodeWav(samples, SAMPLE_RATE, gain));
      } else {
        this.#context ??= new AudioContext();
        const buffer = this.#context.createBuffer(1, samples.length, SAMPLE_RATE);
        buffer.copyToChannel(samples.map((v) => v * gain), 0);
        const source = this.#context.createBufferSource();
        source.buffer = buffer;
        source.connect(this.#context.destination);
        source.start();
      }
      this.error = null;
    } catch (error) {
      this.error = String(error);
    }
  }
}

export const sound = new SoundStore();
