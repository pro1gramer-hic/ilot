// îlot's sound synthesizer (prompt 12, lot 1): every sound is a few short voices, each a
// tone that glides from one pitch to another under a quick attack and an exponential
// decay. Pure arithmetic, no Web Audio: the same samples come out on every machine, the
// tests can listen to them as numbers, and the core plays the WAV this module writes
// (src-tauri/src/sounds.rs). Nothing here is recorded or copied: it is all made of sines.

/** Samples per second: plenty for short interface sounds, half the size of 44.1 kHz. */
export const SAMPLE_RATE = 22_050;

/** Loudest sample of a rendered sound, before the user's volume. */
export const PEAK = 0.8;

/** A wave's partials: their relative loudness (1 = the fundamental), and for a bell their
 * own frequency ratios, since a bell's overtones are not whole multiples. */
export type Timbre = { partials: readonly number[]; ratios?: readonly number[] };

export const TIMBRES = {
  /** A pure, round tone. */
  sine: { partials: [1] },
  /** A plucked bar, close to a kalimba: a few soft harmonics. */
  pluck: { partials: [1, 0.32, 0.12, 0.05] },
  /** Soft and hollow: odd harmonics only, falling fast. */
  hollow: { partials: [1, 0, 0.11, 0, 0.04] },
  /** A small bell: inharmonic overtones. */
  bell: { partials: [1, 0.42, 0.18, 0.08], ratios: [1, 2.76, 5.4, 8.93] },
} as const satisfies Record<string, Timbre>;

export type TimbreId = keyof typeof TIMBRES | "noise";

/** One tone of a sound. Times in seconds from the sound's start. */
export type Voice = {
  timbre: TimbreId;
  /** Pitch at the start and at the end (Hz); it glides exponentially between them. */
  from: number;
  to?: number;
  at: number;
  /** Length, attack included; the decay reaches silence at its end. */
  length: number;
  attack?: number;
  /** Loudness against the other voices. */
  gain?: number;
  /** A wobble of the pitch: cycles per second and depth in semitones. */
  vibrato?: { rate: number; depth: number };
  /** Noise only: a one-pole low-pass that softens it (Hz). */
  cutoff?: number;
};

export type Recipe = readonly Voice[];

const TAU = Math.PI * 2;
/** Default attack: fast enough to feel instant, slow enough not to click. */
const ATTACK_S = 0.006;
/** The last milliseconds fade to zero, so no voice ends on a click. */
const TAIL_S = 0.004;
/** The decay falls to a thousandth of the peak at the voice's end. */
const DECAY_DEPTH = Math.log(1000);

/** A small deterministic generator: the same noise on every render. */
function noiseSource(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return ((state >>> 0) / 0xffffffff) * 2 - 1;
  };
}

/** Loudness of a voice `t` seconds after it starts: a linear attack, an exponential decay,
 * and a short fade at the very end. */
export function envelope(voice: Voice, t: number): number {
  const attack = Math.min(voice.attack ?? ATTACK_S, voice.length / 2);
  if (t < 0 || t >= voice.length) return 0;
  if (t < attack) return t / attack;
  const decay = Math.exp((-DECAY_DEPTH * (t - attack)) / (voice.length - attack));
  const tail = Math.min(1, (voice.length - t) / TAIL_S);
  return decay * tail;
}

/** Pitch of a voice `t` seconds after it starts. */
export function pitch(voice: Voice, t: number): number {
  const to = voice.to ?? voice.from;
  const glide = voice.from * (to / voice.from) ** Math.min(1, Math.max(0, t / voice.length));
  if (!voice.vibrato) return glide;
  return glide * 2 ** ((voice.vibrato.depth * Math.sin(TAU * voice.vibrato.rate * t)) / 12);
}

/** Length of a sound, in seconds. */
export function lengthOf(recipe: Recipe): number {
  return recipe.reduce((end, voice) => Math.max(end, voice.at + voice.length), 0);
}

/** Renders a sound to mono samples, its loudest one at `PEAK`. */
export function render(recipe: Recipe, rate = SAMPLE_RATE): Float32Array {
  const out = new Float32Array(Math.ceil(lengthOf(recipe) * rate));
  recipe.forEach((voice, index) => {
    const first = Math.round(voice.at * rate);
    const count = Math.ceil(voice.length * rate);
    const gain = voice.gain ?? 1;
    if (voice.timbre === "noise") {
      const noise = noiseSource(index * 7919 + 17);
      const smoothing = voice.cutoff ? 1 - Math.exp((-TAU * voice.cutoff) / rate) : 1;
      let low = 0;
      for (let n = 0; n < count && first + n < out.length; n++) {
        low += (noise() - low) * smoothing;
        out[first + n] += low * envelope(voice, n / rate) * gain;
      }
      return;
    }
    const { partials, ratios } = TIMBRES[voice.timbre] as Timbre;
    const phases = partials.map(() => 0);
    for (let n = 0; n < count && first + n < out.length; n++) {
      const t = n / rate;
      const f = pitch(voice, t);
      let sample = 0;
      for (let k = 0; k < partials.length; k++) {
        if (partials[k] === 0) continue;
        const ratio = ratios?.[k] ?? k + 1;
        // Overtones above what the rate can hold are left out (no aliasing).
        if (f * ratio >= rate / 2) continue;
        phases[k] += (TAU * f * ratio) / rate;
        // Higher partials die faster, as on a real bar or bell.
        sample += partials[k] * Math.sin(phases[k]) * Math.exp(-t * 6 * k);
      }
      out[first + n] += sample * envelope(voice, t) * gain;
    }
  });
  let peak = 0;
  for (const sample of out) peak = Math.max(peak, Math.abs(sample));
  if (peak > 0) for (let n = 0; n < out.length; n++) out[n] = (out[n] / peak) * PEAK;
  return out;
}

/** Turns samples into a 16-bit mono WAV, scaled by `gain` (0 to 1). */
export function encodeWav(samples: Float32Array, rate = SAMPLE_RATE, gain = 1): Uint8Array {
  const bytes = new Uint8Array(44 + samples.length * 2);
  const view = new DataView(bytes.buffer);
  const text = (offset: number, value: string): void => {
    for (let i = 0; i < value.length; i++) bytes[offset + i] = value.charCodeAt(i);
  };
  text(0, "RIFF");
  view.setUint32(4, 36 + samples.length * 2, true);
  text(8, "WAVE");
  text(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  text(36, "data");
  view.setUint32(40, samples.length * 2, true);
  const k = Math.min(1, Math.max(0, gain));
  for (let n = 0; n < samples.length; n++) {
    const value = Math.max(-1, Math.min(1, samples[n] * k));
    view.setInt16(44 + n * 2, Math.round(value * 0x7fff), true);
  }
  return bytes;
}

/** One channel out of a decoded file's channels (their average), cut after `max` samples
 * and brought to `PEAK` like îlot's own sounds, so a replacement plays as loud. */
export function mixdown(channels: readonly Float32Array[], max: number): Float32Array {
  const length = Math.min(max, ...channels.map((c) => c.length));
  const out = new Float32Array(Math.max(0, length));
  for (const channel of channels) for (let n = 0; n < out.length; n++) out[n] += channel[n] / channels.length;
  let peak = 0;
  for (const sample of out) peak = Math.max(peak, Math.abs(sample));
  if (peak > 0) for (let n = 0; n < out.length; n++) out[n] = (out[n] / peak) * PEAK;
  return out;
}

/** The user's volume (0 to 1) as a gain: the ear hears loudness on a curve, so half the
 * slider sounds like half as loud. */
export function gainOf(volume: number): number {
  const v = Math.min(1, Math.max(0, Number.isFinite(volume) ? volume : 0));
  return v * v;
}
