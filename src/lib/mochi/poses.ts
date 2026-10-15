// Two poses added for the chat and the dropped files (prompt 12, lot 3), on the same
// 24-unit grid and with the same palettes as `drawMochiV` (engine.ts stays the reference's
// port, untouched):
// - « Réfléchit »: Mochi looks up at three dots lighting one after the other while a model
//   answers: the engine draws him, the dots go on top (`drawThought`).
// - « Boîte »: Mochi turns into a box of his own matter, opens its flaps, a sheet falls in,
//   the flaps close and the box gulps; then it rests, closed, breathing. Drawn here, over
//   the engine's ground shadow and feet, for the three silhouettes: Mèche keeps his lock,
//   Écran his visor, Oreilles their ears.

import { GRID_SIZE, bodyOf, type FrameParams } from "./engine";
import { drawFace, ROUND, type FaceId } from "./faces";
import { DEFAULT_KIND } from "./kinds";
import { DEFAULT_PALETTE, EFFECTS, INK, PALETTES, SCREEN, rgba } from "./palettes";
import { express, poseAt, stillPose, type Expression, type MoveId } from "./moves";

export type PoseId = "think" | "box";
/** Any of Mochi's moves: the reference's, or the two poses of this file. */
export type MochiMove = MoveId | PoseId;

/** Where the box stands in its story, each from 0 to 1. */
export interface BoxFrame {
  /** From Mochi's body to the box. */
  morph: number;
  /** Flaps: 0 closed on top, 1 wide open. */
  open: number;
  /** The sheet falling in: 0 above the box, 1 inside it; negative when there is none. */
  sheet: number;
  /** Vertical squash of the gulp (1 at rest). */
  squash: number;
}

/** `box`: the box of « Boîte »; `thought`: how many of the three dots are lit. */
export type PoseFrame = FrameParams & { box?: BoxFrame; thought?: number };

const wave = (t: number, period: number): number => Math.sin((t * Math.PI * 2) / period);
const clamp01 = (x: number): number => Math.min(1, Math.max(0, x));
/** Exponential ease-out, the island's curve. */
const out = (x: number): number => (x >= 1 ? 1 : 1 - 2 ** (-10 * clamp01(x)));
const span = (t: number, from: number, to: number): number => clamp01((t - from) / (to - from));

/** The swallow's beats, in seconds. */
export const BOX_BEATS = { morph: 0.26, open: 0.56, fall: 1.0, close: 1.24, gulp: 1.56 } as const;

/** The box at `t` seconds: the swallow plays once, then the closed box breathes. */
export function boxAt(t: number): { box: BoxFrame; face: FaceId } {
  const b = BOX_BEATS;
  const morph = out(span(t, 0, b.morph));
  const opening = out(span(t, b.morph, b.open));
  const closing = out(span(t, b.fall, b.close));
  const fall = span(t, b.open, b.fall);
  const gulp = span(t, b.close, b.gulp);
  // Down, up a little past rest, settled: the gulp of a full box.
  const squash = t < b.close ? 1 : t < b.gulp ? 1 - 0.14 * Math.sin(Math.PI * gulp) + 0.04 * Math.sin(2 * Math.PI * gulp) : 1 + 0.015 * wave(t - b.gulp, 2.4);
  const face: FaceId = t < b.morph ? "wow" : t < b.fall ? "eat" : t < b.gulp ? "happy" : (t - b.gulp) % 3.2 < 0.14 ? "blink" : "happy";
  return { box: { morph, open: opening * (1 - closing), sheet: t < b.open ? -1 : fall * fall, squash }, face };
}

/** Dots lit at `t`: one, two, three, then none for a beat. */
function lit(t: number): number {
  return Math.floor(t * 2.6) % 4;
}

/**
 * Three round dots up right of the head, on top of a frame the engine just drew (the
 * context still maps the 24-unit grid): `count` of them lit, the newest brightest. Larger
 * than the engine's effects, so that they read on a 48 px Mochi.
 */
export function drawThought(c: CanvasRenderingContext2D, count: number): void {
  c.save();
  for (let i = 0; i < count; i++) {
    c.beginPath();
    c.arc(16.9 + i * 2.1, 5.2 - i * 1.25, 0.62 + i * 0.12, 0, ROUND);
    c.fillStyle = i === count - 1 ? INK.glint : EFFECTS.snoreFaint;
    c.fill();
  }
  c.restore();
}

export const POSES: Readonly<Record<PoseId, { label: string; face: FaceId; pose: (t: number) => PoseFrame }>> = {
  think: {
    label: "Réfléchit",
    face: "idle",
    pose: (t) => ({
      look: 1,
      ly: -0.55,
      shear: -0.04 + 0.02 * wave(t, 3.2),
      ry: 6.2 + 0.25 * wave(t, 2.6),
      face: t % 3.8 < 0.14 ? "blink" : "idle",
      sway: wave(t, 2.6) > 0 ? 1 : 0,
      thought: lit(t),
    }),
  },
  box: {
    label: "Boîte",
    face: "happy",
    pose: (t) => {
      const { box, face } = boxAt(t);
      return { box, face };
    },
  },
};

export function isPose(move: MochiMove): move is PoseId {
  return move === "think" || move === "box";
}

/** Frame parameters of any move at `t` seconds, with an expression laid over. */
export function framePoseAt(move: MochiMove, t: number, expression?: Expression): PoseFrame {
  if (!isPose(move)) return poseAt(move, t, expression);
  const frame = POSES[move].pose(t);
  // The box keeps its own faces: an imposed face only reaches the thinking Mochi.
  return move === "box" ? frame : express(frame, expression);
}

/** Still frame of any move (animations off, reduced motion): the box rests closed. */
export function stillFramePose(move: MochiMove, expression?: Expression): PoseFrame {
  if (!isPose(move)) return stillPose(move, expression);
  if (move === "box") return { box: { morph: 1, open: 0, sheet: -1, squash: 1 }, face: POSES.box.face };
  return { ...express({ face: POSES.think.face, look: 1, ly: -0.55 }, expression), thought: 3 };
}

/**
 * Draws the box over the whole `size` × `size` canvas behind `c`: the ground shadow and
 * the feet of the engine's Mochi, then a box of his matter with his face on its front.
 */
export function drawBox(c: CanvasRenderingContext2D, size: number, params: PoseFrame): void {
  const box = params.box ?? { morph: 1, open: 0, sheet: -1, squash: 1 };
  const kind = params.kind ?? DEFAULT_KIND;
  const { tiers, outline } = PALETTES[params.palette ?? DEFAULT_PALETTE];
  const body = bodyOf({ kind });
  const ground = 21 - 0.6;
  // The body's own box, eased into a square one: 15 × 11 units.
  const width = body.rx * 2 + (15 - body.rx * 2) * box.morph;
  const height = (body.ry * 2 + (11 - body.ry * 2) * box.morph) * box.squash;
  const radius = 6 - 4.6 * box.morph;
  const left = 12 - width / 2;
  const right = 12 + width / 2;
  const top = ground - height;

  c.setTransform(size / GRID_SIZE, 0, 0, size / GRID_SIZE, 0, 0);
  c.clearRect(0, 0, GRID_SIZE, GRID_SIZE);
  c.save();

  // Ground shadow and feet, as the engine draws them.
  const g = c.createRadialGradient(12, 23, 0, 12, 23, (width / 2) * 0.82);
  g.addColorStop(0, INK.groundShadow);
  g.addColorStop(1, INK.noShadow);
  c.save();
  c.translate(12, 23);
  c.scale(1, 0.22);
  c.translate(-12, -23);
  c.fillStyle = g;
  c.beginPath();
  c.arc(12, 23, (width / 2) * 0.82, 0, ROUND);
  c.fill();
  c.restore();
  for (const s of [-1, 1]) {
    c.beginPath();
    c.ellipse(12 + s * 3.4, ground + 0.35, 1.7, 0.85, 0, 0, ROUND);
    c.fillStyle = tiers[3];
    c.fill();
  }

  // What shows above the box: the ears of Oreilles, the lock of Mèche.
  if (kind === "oreilles") {
    for (const s of [-1, 1]) {
      const bx = 12 + s * (width / 2 - 1.6);
      c.beginPath();
      c.moveTo(bx - 1.5, top + 0.6);
      c.quadraticCurveTo(bx - s * 0.4, top - 3.4, bx + s * 0.6, top - 3.6);
      c.quadraticCurveTo(bx + s * 1.2, top - 1.2, bx + 1.5, top + 0.6);
      c.closePath();
      c.fillStyle = tiers[2];
      c.fill();
      c.beginPath();
      c.moveTo(bx - 0.7, top + 0.4);
      c.quadraticCurveTo(bx, top - 2.2, bx + s * 0.4, top - 2.3);
      c.quadraticCurveTo(bx + s * 0.7, top - 0.8, bx + 0.7, top + 0.4);
      c.closePath();
      c.fillStyle = INK.earInside;
      c.fill();
    }
  }
  if (kind === "meche") {
    const tx = 12.3;
    c.beginPath();
    c.moveTo(tx, top + 0.4);
    c.bezierCurveTo(tx - 0.2, top - 1.8, tx + 2.6, top - 2.8, tx + 2.4, top - 1.2);
    c.bezierCurveTo(tx + 2.2, top - 0.2, tx + 1, top - 0.5, tx + 1.2, top - 1.4);
    c.lineCap = "round";
    c.lineJoin = "round";
    c.lineWidth = 1.25;
    c.strokeStyle = tiers[2];
    c.stroke();
  }

  // The inside, seen while the flaps are open.
  if (box.open > 0.02) {
    c.beginPath();
    c.roundRect(left + 0.5, top - 0.2, width - 1, 1.6, 0.6);
    c.fillStyle = rgba(outline, 0.85 * Math.min(1, box.open * 2));
    c.fill();
  }

  // The sheet falls in before the front is drawn: the front hides what went in.
  if (box.sheet >= 0 && box.sheet < 1) {
    const y = 1.5 + (top + 2.5 - 1.5) * box.sheet;
    c.save();
    c.translate(12, y);
    c.rotate(-0.18 + 0.18 * box.sheet);
    c.beginPath();
    c.moveTo(-2, -2.6);
    c.lineTo(1, -2.6);
    c.lineTo(2, -1.6);
    c.lineTo(2, 2.6);
    c.lineTo(-2, 2.6);
    c.closePath();
    c.fillStyle = INK.glint;
    c.fill();
    c.lineWidth = 0.25;
    c.strokeStyle = rgba(outline, 0.4);
    c.stroke();
    c.strokeStyle = rgba(outline, 0.35);
    c.lineWidth = 0.35;
    for (const line of [-0.9, 0.2, 1.3]) {
      c.beginPath();
      c.moveTo(-1.2, line);
      c.lineTo(1.2, line);
      c.stroke();
    }
    c.restore();
  }

  // The front: Mochi's matter, lit from the top left, darker toward the ground.
  const front = (): void => {
    c.beginPath();
    c.roundRect(left, top, width, height, radius);
  };
  front();
  const light = c.createLinearGradient(left, top, right, ground);
  light.addColorStop(0, tiers[0]);
  light.addColorStop(0.3, tiers[1]);
  light.addColorStop(0.75, tiers[2]);
  light.addColorStop(1, tiers[3]);
  c.fillStyle = light;
  c.fill();
  c.lineWidth = 0.32;
  c.strokeStyle = rgba(outline, 0.45);
  c.stroke();
  // A light streak under the rim, then the rim itself.
  c.beginPath();
  c.roundRect(left + 0.8, top + 1.6, width * 0.32, 0.5, 0.25);
  c.fillStyle = INK.shine;
  c.fill();
  c.beginPath();
  c.moveTo(left + radius * 0.3, top + 1.15);
  c.lineTo(right - radius * 0.3, top + 1.15);
  c.lineWidth = 0.3;
  c.strokeStyle = rgba(outline, 0.3 * box.morph);
  c.stroke();

  // The face on the front; Écran keeps his visor.
  const face = params.face ?? "happy";
  const ey = top + height * 0.52;
  if (kind === "ecran") {
    c.beginPath();
    c.roundRect(12 - 5.2, ey - 2.7, 10.4, 5.4, 2.3);
    c.fillStyle = SCREEN.visor;
    c.fill();
    drawFace(c, face, 12, ey, true);
  } else {
    drawFace(c, face, 12, ey, false);
  }

  // The flaps, hinged on the top corners: flat on the box when closed, splayed out when
  // open; each one a little narrower at its free edge, like cardboard.
  const angle = box.open * 2.4;
  c.globalAlpha = box.morph;
  for (const s of [-1, 1]) {
    c.save();
    c.translate(s < 0 ? left + 0.3 : right - 0.3, top);
    c.rotate(s < 0 ? -angle : angle);
    const length = (width / 2 - 0.2) * (s < 0 ? 1 : -1);
    c.beginPath();
    c.moveTo(0, -1);
    c.lineTo(length, -0.65);
    c.lineTo(length, 0.65);
    c.lineTo(0, 1);
    c.closePath();
    c.fillStyle = box.open > 0.5 ? tiers[2] : tiers[1];
    c.fill();
    c.lineWidth = 0.28;
    c.strokeStyle = rgba(outline, 0.45 * box.morph);
    c.stroke();
    c.restore();
  }
  c.restore();
}
