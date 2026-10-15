// Mochi's moves, ported from `MOVES` and `MV` in `reference/ilot-demo-finale.html`.
// Each move maps a time `t` (seconds) to frame parameters on the 24-unit grid,
// keeping the reference's formulas (and their order of operations) unchanged.
//
// îlot adds moves of its own after the reference's (`OWN_MOVES`, prompt 12): the launch
// greeting and the small emotions Mochi has at rest. They use the same parameters,
// faces and engine, and are not part of the reference's parity tests.
//
// Combining an expression with a move: the move drives the body (size, position, lean,
// sway, effects, gaze) and proposes a face at each instant. An imposed face replaces that
// proposal at every instant, the move's blinks included; otherwise big eyes (pointer over
// Mochi) replace it; otherwise the face at rest replaces the move's plain `idle`. A gaze
// replaces the move's own.

import type { Catalog } from "../i18n/fr";
import { texts } from "../i18n/live.svelte";
import type { FrameParams } from "./engine";
import type { FaceId } from "./faces";
import { EFFECTS } from "./palettes";

export type MoveId =
  | "still"
  | "breathe"
  | "sleep"
  | "bounce"
  | "watch"
  | "rock"
  | "roll"
  | "peek"
  | "shake"
  | "party"
  | "melt"
  | "fly"
  | "swallow"
  | "squish"
  | "dizzy"
  | "hello"
  | "yawn"
  | "stretch"
  | "glance"
  | "smile";

/** îlot's own moves, played once each. */
export type OwnMove = "hello" | "yawn" | "stretch" | "glance" | "smile";

export interface Move {
  /** French name, as in the reference. */
  readonly label: string;
  /** The move's own face, used for still frames (animations off or reduced motion). */
  readonly face: FaceId;
  /** Frame parameters at `t` seconds; the palette and the silhouette are set by the caller. */
  readonly pose: (t: number) => FrameParams;
}

const wave = (t: number, period = 1): number => Math.sin((t * Math.PI * 2) / period);

/** 0 before `from`, 1 after `to`, a smooth step between. */
const step = (t: number, from: number, to: number): number => {
  const p = Math.min(1, Math.max(0, (t - from) / (to - from)));
  return p * p * (3 - 2 * p);
};

/** 0 outside `[from, to]`, rising to 1 in its middle and back: one smooth bump. */
const bump = (t: number, from: number, to: number): number =>
  t <= from || t >= to ? 0 : Math.sin((Math.PI * (t - from)) / (to - from));

/** One cycle of each of îlot's own moves, in seconds. */
export const OWN_MOVE_S: Readonly<Record<OwnMove, number>> = {
  hello: 1.6,
  yawn: 2.4,
  stretch: 2.2,
  glance: 3.2,
  smile: 1.8,
};

export const MOVES: Readonly<Record<MoveId, Move>> = {
  still: {
    label: "Immobile",
    face: "idle",
    pose: (t) => ({ face: t % 4 < 0.12 ? "blink" : "idle" }),
  },
  breathe: {
    label: "Respire",
    face: "idle",
    pose: (t) => ({
      ry: 6.2 + 0.5 * wave(t, 2.4),
      rx: 8.2 - 0.4 * wave(t, 2.4),
      face: t % 4.2 < 0.15 ? "blink" : "idle",
      sway: wave(t, 2.4) > 0 ? 1 : 0,
    }),
  },
  sleep: {
    label: "Dort",
    face: "sleep",
    pose: (t) => {
      const z = Math.floor(t * 3) % 6;
      const fx: [number, number, string][] = [];
      if (z > 0) fx.push([17, 7 - Math.min(z, 4), EFFECTS.snore]);
      if (z > 2) fx.push([19, 4 - Math.min(z - 2, 3), EFFECTS.snoreFaint]);
      return { ry: 5.9 + 0.35 * wave(t, 3.2), rx: 8.4, face: "sleep", sway: -1, fx };
    },
  },
  bounce: {
    label: "Rebondit",
    face: "happy",
    pose: (t) => {
      const h = Math.abs(Math.sin(Math.PI * (t % 1)));
      const land = h < 0.18;
      return {
        bot: 21 - Math.round(h * 6),
        rx: land ? 9.2 : 7.8,
        ry: land ? 5.2 : 6.6,
        face: "happy",
        shadowK: 1 - h * 0.5,
        sway: h > 0.5 ? -1 : 1,
      };
    },
  },
  watch: {
    label: "Regarde",
    face: "idle",
    pose: (t) => {
      const p = t % 4;
      const look = p < 1 ? -1 : p < 2 ? 0 : p < 3 ? 1 : 0;
      return { face: p % 2 > 1.85 ? "blink" : "idle", look, sway: look };
    },
  },
  rock: {
    label: "Se balance",
    face: "happy",
    pose: (t) => ({ shear: 0.16 * wave(t, 1.4), face: "happy", sway: wave(t, 1.4) > 0 ? 1 : -1 }),
  },
  roll: {
    label: "Roule",
    face: "idle",
    pose: (t) => {
      const p = (t % 4) / 4;
      const dir = p < 0.5 ? 1 : -1;
      const cx = p < 0.5 ? 6 + p * 2 * 12 : 18 - (p - 0.5) * 2 * 12;
      return { cx, look: dir, face: "idle", shear: -0.06 * dir, sway: dir };
    },
  },
  peek: {
    label: "Coucou",
    face: "wow",
    pose: (t) => {
      const p = t % 3.2;
      const up = p < 0.6 ? p / 0.6 : p < 2.4 ? 1 : p < 3 ? 1 - (p - 2.4) / 0.6 : 0;
      return {
        bot: 21 + Math.round((1 - up) * 12),
        face: p > 0.6 && p < 2.4 ? "wow" : "idle",
        sway: p > 0.8 && p < 2.2 ? (Math.floor(p * 6) % 2 ? 1 : -1) : 0,
        shadow: up > 0.7,
      };
    },
  },
  shake: {
    label: "Tremble",
    face: "worry",
    pose: (t) => {
      const odd = Math.floor(t * 14) % 2 !== 0;
      return { cx: 12 + (odd ? 0.6 : -0.6), face: "worry", sway: odd ? 1 : -1 };
    },
  },
  party: {
    label: "Fait la fête",
    face: "star",
    pose: (t) => {
      const h = Math.abs(Math.sin((Math.PI * (t % 0.8)) / 0.8));
      const confetti = EFFECTS.confetti;
      const fx: [number, number, string][] = [];
      for (let i = 0; i < 7; i++) {
        const a = (t * 1.3 + i * 0.143) % 1;
        fx.push([(i * 7 + 3) % 24, Math.floor(a * 20), confetti[i % confetti.length]]);
      }
      return { bot: 21 - Math.round(h * 5), face: "star", fx, shadowK: 1 - h * 0.4, sway: h > 0.5 ? 1 : -1 };
    },
  },
  melt: {
    label: "Fond",
    face: "sleep",
    pose: (t) => {
      const p = t % 4;
      const k = p < 1.2 ? p / 1.2 : p < 2.4 ? 1 : p < 3 ? 1 - (p - 2.4) / 0.6 : 0;
      return { ry: 6.2 - k * 3.4, rx: 8.2 + k * 3, face: k > 0.6 ? "sleep" : "idle", sway: k > 0.5 ? -1 : 0 };
    },
  },
  fly: {
    label: "Vole",
    face: "happy",
    pose: (t) => ({
      bot: 17 + Math.round(2 * wave(t, 0.7)),
      shadowK: 0.45,
      sway: wave(t, 0.25) > 0 ? 1 : -1,
      face: "happy",
      shear: 0.05 * wave(t, 1.4),
    }),
  },
  swallow: {
    label: "Avale",
    face: "eat",
    pose: (t) => {
      const open = Math.floor(t * 3) % 2 !== 0;
      return { face: open ? "eat" : "happy", ry: 6.2 + (open ? 0.6 : -0.2), rx: 8.2 - (open ? 0.3 : 0) };
    },
  },
  squish: {
    label: "Clic",
    face: "annoyed",
    pose: (t) => {
      const p = t % 1.2;
      const k = p < 0.45 ? 1 - p / 0.45 : 0;
      return { rx: 8.2 + 2.2 * k, ry: 6.2 - 2.4 * k, bot: 21, face: k > 0 ? "annoyed" : "idle" };
    },
  },
  dizzy: {
    label: "Tournis",
    face: "dizzy",
    pose: (t) => ({
      face: "dizzy",
      shear: 0.14 * wave(t, 0.7),
      sway: wave(t, 0.7) > 0 ? 1 : -1,
      cx: 12 + Math.round(wave(t, 0.7)),
    }),
  },
  // ---------- îlot's own moves ----------
  hello: {
    label: "Salut",
    face: "happy",
    pose: (t) => {
      // A small hop, a landing squash, then he waves (Mèche raises his arm) and winks.
      const p = t % OWN_MOVE_S.hello;
      const hop = bump(p, 0, 0.34);
      const land = bump(p, 0.34, 0.5);
      const swing = p > 0.4 ? wave(p - 0.4, 0.8) : 0;
      return {
        bot: 21 - hop * 1.8,
        rx: 8.2 + land * 0.7 - hop * 0.3,
        ry: 6.2 - land * 0.6 + hop * 0.3,
        wave: p > 0.3,
        face: p < 0.3 ? "wow" : p < 1.25 ? "happy" : "wink",
        shear: 0.07 * swing,
        sway: swing > 0 ? 1 : -1,
        shadowK: 1 - hop * 0.25,
      };
    },
  },
  yawn: {
    label: "Bâille",
    face: "eat",
    pose: (t) => {
      // He grows a little taller, eyes shut and mouth wide, then sinks back with a blink.
      const p = t % OWN_MOVE_S.yawn;
      const open = bump(p, 0.25, 1.75);
      return {
        rx: 8.2 - open * 0.45,
        ry: 6.2 + open * 0.75,
        face: p < 0.3 ? "idle" : p < 1.65 ? "eat" : p < 1.85 ? "blink" : "idle",
        sway: open > 0.5 ? -1 : 0,
      };
    },
  },
  stretch: {
    label: "S'étire",
    face: "happy",
    pose: (t) => {
      // Up on his feet, long and thin, then a soft squash as he settles.
      const p = t % OWN_MOVE_S.stretch;
      const up = bump(p, 0.15, 1.45);
      const settle = bump(p, 1.4, 1.75);
      return {
        rx: 8.2 - up * 0.7 + settle * 0.7,
        ry: 6.2 + up * 1.0 - settle * 0.6,
        shear: 0.05 * up * wave(p, 1.3),
        face: up > 0.2 ? "happy" : "idle",
        sway: up > 0.5 ? 1 : 0,
      };
    },
  },
  glance: {
    label: "Regarde autour",
    face: "idle",
    pose: (t) => {
      // His eyes glide to the left, then to the right, then back: a look around.
      const p = t % OWN_MOVE_S.glance;
      const look = -step(p, 0.3, 0.75) + 2 * step(p, 1.35, 1.95) - step(p, 2.55, 3);
      return { look, face: p > 1.12 && p < 1.25 ? "blink" : "idle", sway: look > 0.3 ? 1 : look < -0.3 ? -1 : 0 };
    },
  },
  smile: {
    label: "Sourit",
    face: "happy",
    pose: (t) => {
      // A blink, a happy smile with a small bounce of joy, another blink.
      const p = t % OWN_MOVE_S.smile;
      const joy = bump(p, 0.15, 1.3);
      return {
        ry: 6.2 + 0.3 * joy,
        rx: 8.2 - 0.15 * joy,
        face: p < 0.15 || (p > 1.3 && p < 1.45) ? "blink" : p < 1.3 ? "happy" : "idle",
        sway: joy > 0.5 ? 1 : 0,
      };
    },
  },
};

/** îlot's own moves (prompt 12), not in the reference. */
export const OWN_MOVES: readonly OwnMove[] = ["hello", "yawn", "stretch", "glance", "smile"];

/** Every move of the reference: the ten animations of the onglet Mochi, then the island's
 * other moves (îlot's own are in `OWN_MOVES`). */
export const MOVE_IDS: readonly MoveId[] = [
  "still",
  "breathe",
  "bounce",
  "rock",
  "squish",
  "dizzy",
  "swallow",
  "watch",
  "fly",
  "shake",
  "sleep",
  "roll",
  "peek",
  "party",
  "melt",
];

type Animation = keyof Catalog["mochi"]["animations"];

/** An animation of the onglet Mochi, its chip label in the language of the moment. */
function animation(move: Animation): { readonly move: MoveId; readonly label: string } {
  return {
    move,
    get label() {
      return texts().mochi.animations[move];
    },
  };
}

/** The ten animations of the onglet Mochi (`ANIMS` in the reference), with their chip label. */
export const ANIMATIONS: readonly { readonly move: MoveId; readonly label: string }[] = [
  animation("still"),
  animation("breathe"),
  animation("bounce"),
  animation("rock"),
  animation("squish"),
  animation("dizzy"),
  animation("swallow"),
  animation("watch"),
  animation("fly"),
  animation("shake"),
];

/** What is laid over a move: see the header of this file for the order. */
export interface Expression {
  /** Imposed face, at every instant. */
  face?: FaceId;
  /** Big eyes, while the pointer is over Mochi. */
  big?: boolean;
  /** Face at rest, in place of the move's plain `idle`. */
  rest?: FaceId;
  /** Gaze, in place of the move's own: -1, 0 or 1. */
  look?: number;
}

/** Lays an expression over a frame. */
export function express(pose: FrameParams, { face, big, rest, look }: Expression = {}): FrameParams {
  const out = { ...pose };
  if (face !== undefined) out.face = face;
  else if (big) out.face = "big";
  else if (rest !== undefined && (out.face ?? "idle") === "idle") out.face = rest;
  if (look !== undefined) out.look = look;
  return out;
}

/** Frame parameters of `move` at `t` seconds, with an expression laid over. */
export function poseAt(move: MoveId, t: number, expression?: Expression): FrameParams {
  return express(MOVES[move].pose(t), expression);
}

/** Still frame for `move`: Mochi at rest, wearing the move's own face under the expression. */
export function stillPose(move: MoveId, expression?: Expression): FrameParams {
  return express({ face: MOVES[move].face }, expression);
}
