// îlot's twenty sounds (prompt 12, lot 1), made for îlot from the synthesizer in
// synth.ts: their names (also the names of the replacement files, see
// src-tauri/src/sounds.rs), what they mean, when they may play, and how they sound.
//
// When a sound plays: never while sounds are cut, at volume 0 or under Ne pas déranger;
// with reduced motion, only the sounds that answer the user's own gesture (a click on
// Mochi, an answer, Écouter in Réglages), never one that comes on its own.

import { texts } from "../i18n/live.svelte";
import type { Recipe } from "./synth";

/** The sounds, in the order Réglages › Son lists them. */
export const SOUND_IDS = [
  "finish",
  "fail",
  "approval",
  "allow",
  "deny",
  "message",
  "masked",
  "track",
  "toast",
  "jump",
  "swallow",
  "click",
  "dizzy",
  "hello",
  "yawn",
  "leave",
  "back",
  "detach",
  "suspend",
  "wardrobe",
] as const;

export type SoundId = (typeof SOUND_IDS)[number];

/** `auto`: îlot plays it on its own (an agent finished, a message came). `user`: it
 * answers what the user just did. */
export type Origin = "auto" | "user";

/** A sound: its name in the language of the moment (Réglages › Son), and where it comes from. */
function named(id: SoundId, origin: Origin): { label: string; origin: Origin } {
  return {
    get label() {
      return texts().sounds[id];
    },
    origin,
  };
}

export const SOUNDS: Readonly<Record<SoundId, { label: string; origin: Origin }>> = {
  finish: named("finish", "auto"),
  fail: named("fail", "auto"),
  approval: named("approval", "auto"),
  allow: named("allow", "user"),
  deny: named("deny", "user"),
  message: named("message", "auto"),
  masked: named("masked", "auto"),
  track: named("track", "auto"),
  toast: named("toast", "user"),
  jump: named("jump", "auto"),
  swallow: named("swallow", "user"),
  click: named("click", "user"),
  dizzy: named("dizzy", "user"),
  hello: named("hello", "auto"),
  yawn: named("yawn", "auto"),
  leave: named("leave", "user"),
  back: named("back", "auto"),
  detach: named("detach", "user"),
  suspend: named("suspend", "user"),
  wardrobe: named("wardrobe", "user"),
};

/** What decides whether a sound may play now. */
export type Conditions = { muted: boolean; volume: number; doNotDisturb: boolean; reducedMotion: boolean };

/** `preview`: the user pressed Écouter in Réglages; it plays even while sounds are cut. */
export function mayPlay(id: SoundId, conditions: Conditions, preview = false): boolean {
  if (conditions.volume <= 0) return false;
  if (preview) return true;
  if (conditions.muted || conditions.doNotDisturb) return false;
  return !(conditions.reducedMotion && SOUNDS[id].origin === "auto");
}

/** The same sound twice within this time plays once (two events in the same instant). */
export const REPEAT_MS = 150;

// Notes of the pentatonic scale îlot's sounds are made of (Hz).
const A3 = 220;
const C4 = 261.63;
const D5 = 587.33;
const E5 = 659.25;
const G5 = 783.99;
const A5 = 880;
const C5 = 523.25;
const C6 = 1046.5;
const E6 = 1318.51;
const G6 = 1567.98;
const C7 = 2093;

export const RECIPES: Readonly<Record<SoundId, Recipe>> = {
  // A little rising arpeggio, the last note left to ring.
  finish: [
    { timbre: "pluck", from: C5, at: 0, length: 0.16 },
    { timbre: "pluck", from: E5, at: 0.07, length: 0.16 },
    { timbre: "pluck", from: G5, at: 0.14, length: 0.18 },
    { timbre: "pluck", from: C6, at: 0.21, length: 0.42 },
  ],
  // Two soft notes going down, the second sagging a little: not an alarm.
  fail: [
    { timbre: "hollow", from: E5, at: 0, length: 0.2, gain: 0.9 },
    { timbre: "hollow", from: C5, to: C5 * 0.94, at: 0.16, length: 0.38 },
  ],
  // Two small bells: someone is waiting for an answer.
  approval: [
    { timbre: "bell", from: A5, at: 0, length: 0.32 },
    { timbre: "bell", from: A5, at: 0.14, length: 0.42, gain: 0.85 },
  ],
  allow: [
    { timbre: "sine", from: G5, to: C6, at: 0, length: 0.12 },
    { timbre: "pluck", from: C6, at: 0.09, length: 0.22, gain: 0.7 },
  ],
  // A low, muffled knock.
  deny: [
    { timbre: "sine", from: C4, to: A3, at: 0, length: 0.2 },
    { timbre: "noise", from: 1, at: 0, length: 0.05, cutoff: 900, gain: 0.35 },
  ],
  // Two bubbles popping.
  message: [
    { timbre: "sine", from: 600, to: 1100, at: 0, length: 0.07 },
    { timbre: "sine", from: 800, to: 1400, at: 0.08, length: 0.08 },
  ],
  masked: [{ timbre: "hollow", from: 900, at: 0, length: 0.06, gain: 0.6 }],
  track: [
    { timbre: "bell", from: E6, at: 0, length: 0.36, gain: 0.8 },
    { timbre: "bell", from: C6, at: 0.06, length: 0.44 },
  ],
  toast: [
    { timbre: "pluck", from: G5, at: 0, length: 0.07 },
    { timbre: "pluck", from: C6, at: 0.06, length: 0.12 },
  ],
  // A springy hop upward.
  jump: [{ timbre: "sine", from: 300, to: 720, at: 0, length: 0.15, vibrato: { rate: 18, depth: 0.4 } }],
  // A gulp, then a smaller one.
  swallow: [
    { timbre: "sine", from: 520, to: 160, at: 0, length: 0.16 },
    { timbre: "sine", from: 380, to: 140, at: 0.13, length: 0.12, gain: 0.6 },
  ],
  // A soft squish.
  click: [
    { timbre: "hollow", from: 440, to: 260, at: 0, length: 0.08 },
    { timbre: "pluck", from: E5, at: 0.03, length: 0.09, gain: 0.4 },
  ],
  // A wobbling fall.
  dizzy: [{ timbre: "sine", from: 720, to: 340, at: 0, length: 0.62, vibrato: { rate: 9, depth: 1.6 } }],
  // A small tune that waves: two quick notes, then a held one.
  hello: [
    { timbre: "pluck", from: G5, at: 0, length: 0.12 },
    { timbre: "pluck", from: C6, at: 0.09, length: 0.12 },
    { timbre: "pluck", from: E6, at: 0.18, length: 0.46, vibrato: { rate: 6, depth: 0.25 } },
  ],
  // A slow vowel that opens and sinks.
  yawn: [
    { timbre: "hollow", from: 330, to: 520, at: 0, length: 0.34, attack: 0.12, gain: 0.7 },
    { timbre: "hollow", from: 520, to: 290, at: 0.3, length: 0.5, attack: 0.04, gain: 0.6 },
  ],
  // A pop, then a slide away.
  leave: [
    { timbre: "noise", from: 1, at: 0, length: 0.03, cutoff: 2400, gain: 0.4 },
    { timbre: "sine", from: 900, to: 340, at: 0.02, length: 0.26 },
  ],
  // A slide back in, then a plop.
  back: [
    { timbre: "sine", from: 340, to: 900, at: 0, length: 0.2 },
    { timbre: "pluck", from: C6, at: 0.2, length: 0.2 },
  ],
  // Liquid bloops, like the bubble coming off the island.
  detach: [
    { timbre: "sine", from: 250, to: 600, at: 0, length: 0.12, vibrato: { rate: 30, depth: 0.6 } },
    { timbre: "sine", from: 320, to: 820, at: 0.1, length: 0.14, gain: 0.8 },
  ],
  // Two slow notes settling down.
  suspend: [
    { timbre: "hollow", from: G5, at: 0, length: 0.24, attack: 0.03 },
    { timbre: "hollow", from: D5, at: 0.18, length: 0.4, attack: 0.03 },
  ],
  // A sparkle.
  wardrobe: [
    { timbre: "bell", from: E6, at: 0, length: 0.14, gain: 0.7 },
    { timbre: "bell", from: G6, at: 0.045, length: 0.14, gain: 0.7 },
    { timbre: "bell", from: C7, at: 0.09, length: 0.3, gain: 0.8 },
  ],
};
