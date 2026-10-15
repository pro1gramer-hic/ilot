// Mochi's small emotions at rest (prompt 12, lot 1): now and then, when nothing happens,
// he smiles, yawns, looks around or stretches. Rare (one every 2 to 5 minutes, drawn at
// random), short (one cycle of the move, 2 to 3 s), never twice the same in a row, and
// never while anything goes on, with reduced motion or with animations off. In the
// island he is frozen at rest (IslandMochi.svelte): an emotion wakes him just long
// enough to play, then he freezes again.

import { OWN_MOVE_S, type OwnMove } from "./moves";

export const EMOTIONS: readonly OwnMove[] = ["smile", "yawn", "glance", "stretch"];

/** Shortest and longest wait between two emotions. */
export const EMOTION_MIN_MS = 120_000;
export const EMOTION_MAX_MS = 300_000;

/** The wait before the next emotion, drawn evenly between 2 and 5 minutes. */
export function nextEmotionDelay(random: () => number): number {
  const r = Math.min(1, Math.max(0, random()));
  return Math.round(EMOTION_MIN_MS + r * (EMOTION_MAX_MS - EMOTION_MIN_MS));
}

/** An emotion drawn at random, never the one just played. */
export function pickEmotion(random: () => number, previous?: OwnMove): OwnMove {
  const choices = EMOTIONS.filter((e) => e !== previous);
  return choices[Math.min(choices.length - 1, Math.floor(Math.min(0.999999, Math.max(0, random())) * choices.length))];
}

/** How long an emotion plays, in milliseconds: one cycle of its move. */
export function emotionMs(emotion: OwnMove): number {
  return OWN_MOVE_S[emotion] * 1000;
}

/** What has to be calm for an emotion to play. */
export type Calm = {
  /** The island is folded (no opening, hub or panel). */
  folded: boolean;
  working: boolean;
  playing: boolean;
  /** The pointer is on or near the island, or something is being dragged. */
  busy: boolean;
  /** Mochi is out of the island, or the island is tucked away. */
  away: boolean;
  /** Animations on (settings) and no reduced motion. */
  animated: boolean;
};

export function canEmote(calm: Calm): boolean {
  return calm.folded && !calm.working && !calm.playing && !calm.busy && !calm.away && calm.animated;
}
