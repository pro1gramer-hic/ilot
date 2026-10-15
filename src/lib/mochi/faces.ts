// Mochi's faces, ported from `vFace()` in reference/mochi-vectoriel.js and `HEADS` in
// reference/ilot-demo-finale.html: the nine heads the user can choose, plus four reaction
// faces (big eyes under the pointer, annoyed chevrons, dizzy spirals, a mouth open to eat).

import { liveRecord, texts } from "../i18n/live.svelte";
import { INK, SCREEN } from "./palettes";

/** The nine heads; the chosen one becomes Mochi's face at rest. */
export type HeadId = "idle" | "blink" | "happy" | "wow" | "worry" | "sleep" | "love" | "star" | "wink";
export type FaceId = HeadId | "big" | "annoyed" | "dizzy" | "eat";

/** Head ids in display order (same order as the reference). */
export const HEAD_IDS: readonly HeadId[] = ["idle", "blink", "happy", "wow", "worry", "sleep", "love", "star", "wink"];

/** Every face the engine draws: the nine heads, then the reactions. */
export const FACE_IDS: readonly FaceId[] = [...HEAD_IDS, "big", "annoyed", "dizzy", "eat"];

/** Labels of the heads in the language of the moment (the French ones from the
 * reference's onglet Mochi). */
export const HEAD_LABELS: Readonly<Record<HeadId, string>> = liveRecord(HEAD_IDS, (id) => texts().mochi.heads[id]);

/** True for one of the nine heads (settings come from outside the type system). */
export function isHead(value: unknown): value is HeadId {
  return typeof value === "string" && (HEAD_IDS as readonly string[]).includes(value);
}

/** Radians past a full turn: `arc` and `ellipse` draw the whole shape (the reference's value). */
export const ROUND = 7;

/** Horizontal distance from the face's center to each eye, in grid units. */
export const EYE_GAP = 2.6;

/** Blur of the glowing eyes on the visor, in canvas pixels as the reference sets it. */
const GLOW_BLUR = 1.2;

// Eyes on a sphere (prompt 12, lot 1): looking aside, each eye slides a little further
// than the face, toward the edge, and narrows into an ellipse as a mark on a ball would,
// the eye on the side he looks toward more than the other.
/** Extra slide of the eyes per step of gaze, in grid units. */
const LENS_SLIDE = 0.45;
/** Narrowing at a full step of gaze: the eye on the near edge, the other one. */
const LENS_NEAR = 0.28;
const LENS_FAR = 0.08;

/** How one eye (`side` -1 left, 1 right) slides and narrows for a gaze of `look`. */
export function eyeLens(look: number, side: -1 | 1): { shift: number; squeeze: number } {
  const l = Math.max(-1, Math.min(1, look));
  const near = l * side > 0;
  return { shift: l * LENS_SLIDE, squeeze: 1 - Math.abs(l) * (near ? LENS_NEAR : LENS_FAR) };
}

/** Draws `draw` (one eye at `x`) slid and narrowed for a gaze of `look`, on its own row. */
export function onSphere(c: CanvasRenderingContext2D, look: number, side: -1 | 1, x: number, y: number, draw: () => void): void {
  const { shift, squeeze } = eyeLens(look, side);
  c.save();
  c.translate(x + shift, y);
  c.scale(squeeze, 1);
  c.translate(-x, -y);
  draw();
  c.restore();
}

/**
 * Draws `face` centered on (`cx`, `ey`), the eyes' row; the mouth sits 2.5 units lower.
 * On the « Écran » visor (`screen`), everything glows in cyan and there are no cheeks.
 * With `look` (not 0), the eyes sit on a sphere (`eyeLens`); without it, the drawing is
 * the reference's, call for call.
 */
export function drawFace(c: CanvasRenderingContext2D, face: FaceId, cx: number, ey: number, screen: boolean, look = 0): void {
  const col: string = screen ? SCREEN.glow : INK.eye;
  const mouthInk: string = screen ? col : INK.mouth;
  const L = cx - EYE_GAP;
  const R = cx + EYE_GAP;
  c.save();
  if (screen) {
    c.shadowColor = col;
    c.shadowBlur = GLOW_BLUR;
  }
  // One eye, on the sphere when Mochi looks aside.
  const eye = (side: -1 | 1, draw: () => void): void => {
    if (look === 0) draw();
    else onSphere(c, look, side, side < 0 ? L : R, ey, draw);
  };

  const oval = (x: number, y: number, w: number, h: number): void => {
    c.beginPath();
    c.ellipse(x, y, w, h, 0, 0, ROUND);
    c.fillStyle = col;
    c.fill();
    if (!screen) {
      c.fillStyle = INK.glint;
      c.beginPath();
      c.arc(x + w * 0.35, y - h * 0.4, w * 0.32, 0, ROUND);
      c.fill();
    }
  };
  // A closed eye: a curve up (smiling eye) or down (lowered lid).
  const arc = (x: number, y: number, up: boolean, w = 1.2): void => {
    c.beginPath();
    c.lineCap = "round";
    c.lineWidth = 0.55;
    c.strokeStyle = col;
    if (up) c.arc(x, y + 0.5, w, Math.PI * 1.15, Math.PI * 1.85);
    else c.arc(x, y - 0.6, w, Math.PI * 0.15, Math.PI * 0.85);
    c.stroke();
  };
  const line = (x1: number, y1: number, x2: number, y2: number, w = 0.5, color = col): void => {
    c.beginPath();
    c.lineCap = "round";
    c.lineWidth = w;
    c.strokeStyle = color;
    c.moveTo(x1, y1);
    c.lineTo(x2, y2);
    c.stroke();
  };
  const mouth = (shape: "smile" | "open" | "o" | "flat" | "wavy"): void => {
    const my = ey + 2.5;
    c.lineCap = "round";
    c.lineWidth = 0.45;
    c.strokeStyle = mouthInk;
    if (shape === "smile") {
      c.beginPath();
      c.arc(cx, my - 0.5, 0.8, Math.PI * 0.2, Math.PI * 0.8);
      c.stroke();
    } else if (shape === "open") {
      c.beginPath();
      c.arc(cx, my - 0.3, 0.95, 0, Math.PI);
      c.closePath();
      c.fillStyle = mouthInk;
      c.fill();
      if (!screen) {
        c.beginPath();
        c.arc(cx, my + 0.25, 0.45, 0, Math.PI);
        c.fillStyle = INK.tongue;
        c.fill();
      }
    } else if (shape === "o") {
      c.beginPath();
      c.ellipse(cx, my, 0.5, 0.62, 0, 0, ROUND);
      c.fillStyle = mouthInk;
      c.fill();
    } else if (shape === "flat") {
      line(cx - 0.7, my, cx + 0.7, my, 0.45, mouthInk);
    } else {
      c.beginPath();
      c.moveTo(cx - 1, my);
      c.quadraticCurveTo(cx - 0.5, my - 0.5, cx, my);
      c.quadraticCurveTo(cx + 0.5, my + 0.5, cx + 1, my);
      c.stroke();
    }
  };
  const cheeks = (): void => {
    if (screen) return;
    c.fillStyle = INK.cheek;
    for (const x of [L - 1, R + 1]) {
      c.beginPath();
      c.ellipse(x, ey + 1.9, 1, 0.55, 0, 0, ROUND);
      c.fill();
    }
  };
  const heart = (x: number, y: number, s: number): void => {
    c.beginPath();
    c.moveTo(x, y + s * 0.9);
    c.bezierCurveTo(x - s * 1.6, y - s * 0.2, x - s * 0.7, y - s * 1.4, x, y - s * 0.5);
    c.bezierCurveTo(x + s * 0.7, y - s * 1.4, x + s * 1.6, y - s * 0.2, x, y + s * 0.9);
    c.fillStyle = INK.heart;
    c.fill();
  };
  // Four branches: eight points alternating between the tip and the hollow.
  const star = (x: number, y: number, s: number): void => {
    c.beginPath();
    for (let i = 0; i < 8; i++) {
      const r = i % 2 ? s * 0.38 : s;
      const a = (i * Math.PI) / 4 - Math.PI / 2;
      c.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
    }
    c.closePath();
    c.fillStyle = INK.star;
    c.fill();
  };
  // Two turns growing from the center of the eye.
  const spiral = (x: number): void => {
    c.beginPath();
    c.lineWidth = 0.42;
    c.strokeStyle = col;
    for (let i = 0; i <= 40; i++) {
      const a = (i / 40) * Math.PI * 4;
      const r = 0.12 + (i / 40) * 0.95;
      c.lineTo(x + Math.cos(a) * r, ey + Math.sin(a) * r);
    }
    c.stroke();
  };

  switch (face) {
    case "blink":
      eye(-1, () => arc(L, ey, false));
      eye(1, () => arc(R, ey, false));
      cheeks();
      mouth("smile");
      break;
    case "happy":
      eye(-1, () => arc(L, ey, true));
      eye(1, () => arc(R, ey, true));
      cheeks();
      mouth("open");
      break;
    case "wow":
      eye(-1, () => oval(L, ey, 1.1, 1.55));
      eye(1, () => oval(R, ey, 1.1, 1.55));
      cheeks();
      mouth("o");
      break;
    case "worry":
      eye(-1, () => oval(L, ey + 0.2, 0.8, 1.1));
      eye(1, () => oval(R, ey + 0.2, 0.8, 1.1));
      line(L - 0.9, ey - 1.6, L + 0.6, ey - 2.1);
      line(R + 0.9, ey - 1.6, R - 0.6, ey - 2.1);
      mouth("wavy");
      c.fillStyle = INK.sweat;
      c.beginPath();
      c.ellipse(R + 2.4, ey - 1.4, 0.4, 0.6, 0, 0, ROUND);
      c.fill();
      break;
    case "sleep":
      eye(-1, () => line(L - 0.8, ey + 0.2, L + 0.8, ey + 0.2));
      eye(1, () => line(R - 0.8, ey + 0.2, R + 0.8, ey + 0.2));
      cheeks();
      mouth("o");
      break;
    case "love":
      eye(-1, () => heart(L, ey, 1));
      eye(1, () => heart(R, ey, 1));
      cheeks();
      mouth("smile");
      break;
    case "star":
      eye(-1, () => star(L, ey, 1.3));
      eye(1, () => star(R, ey, 1.3));
      cheeks();
      mouth("open");
      break;
    case "wink":
      eye(-1, () => oval(L, ey, 0.92, 1.32));
      eye(1, () => arc(R, ey, true));
      cheeks();
      mouth("smile");
      break;
    case "big":
      eye(-1, () => oval(L, ey, 1.22, 1.7));
      eye(1, () => oval(R, ey, 1.22, 1.7));
      if (!screen) {
        c.fillStyle = INK.glint;
        for (const x of [L, R]) {
          eye(x < cx ? -1 : 1, () => {
            c.beginPath();
            c.arc(x - 0.3, ey + 0.5, 0.2, 0, ROUND);
            c.fill();
          });
        }
      }
      cheeks();
      mouth("smile");
      break;
    case "annoyed":
      // Chevrons pointing at each other: > <
      eye(-1, () => {
        line(L - 0.8, ey - 0.8, L + 0.6, ey, 0.55);
        line(L + 0.6, ey, L - 0.8, ey + 0.8, 0.55);
      });
      eye(1, () => {
        line(R + 0.8, ey - 0.8, R - 0.6, ey, 0.55);
        line(R - 0.6, ey, R + 0.8, ey + 0.8, 0.55);
      });
      mouth("flat");
      break;
    case "dizzy":
      eye(-1, () => spiral(L));
      eye(1, () => spiral(R));
      mouth("wavy");
      break;
    case "eat":
      eye(-1, () => arc(L, ey, true));
      eye(1, () => arc(R, ey, true));
      cheeks();
      c.beginPath();
      c.ellipse(cx, ey + 2.5, 1.2, 1, 0, 0, ROUND);
      c.fillStyle = mouthInk;
      c.fill();
      if (!screen) {
        c.beginPath();
        c.ellipse(cx, ey + 3, 0.7, 0.4, 0, 0, ROUND);
        c.fillStyle = INK.tongue;
        c.fill();
      }
      break;
    default:
      // « Content », and any face this engine does not know (settings come from outside).
      eye(-1, () => oval(L, ey, 0.92, 1.32));
      eye(1, () => oval(R, ey, 0.92, 1.32));
      cheeks();
      mouth("smile");
      break;
  }
  c.restore();
}
