// Mochi's smooth engine, ported from `drawMochiV()` in reference/mochi-vectoriel.js (the
// same code draws every Mochi of reference/ilot-demo-finale.html). Canvas 2D on a grid of
// 24 units: a superellipse body with a radial gradient, a glossy highlight, a thin outline,
// a soft ground shadow and feet, then the silhouette's own parts and the face. The calls
// follow the reference one by one, in its order, inside a save/restore pair of our own
// (see `finale.test.ts`).

import { drawFace, ROUND, type FaceId } from "./faces";
import { BODIES, DEFAULT_KIND, type BodyShape, type KindId } from "./kinds";
import { DEFAULT_PALETTE, INK, PALETTES, SCREEN, rgba, type PaletteId } from "./palettes";

/** Mochi's drawing grid, in units: the canvas maps it onto its whole surface. */
export const GRID_SIZE = 24;

/** Beyond this pixel ratio, more canvas pixels cost CPU without a visible gain. */
const MAX_PIXEL_RATIO = 2;

/** Segments of the body's outline, a closed polygon of `BODY_SEGMENTS + 1` points. */
export const BODY_SEGMENTS = 72;

/** A small rounded square drawn on top of everything (zzz, confetti): [x, y, color]. */
export type Effect = readonly [x: number, y: number, color: string];

/**
 * Parameters of one frame. Names follow the reference so that moves stay a direct port.
 * `rx` and `ry` are relative: moves deform every silhouette by the same ratio.
 */
export interface FrameParams {
  palette?: PaletteId;
  kind?: KindId;
  /** Horizontal center of the body (default: grid center). */
  cx?: number;
  /** Ground line of the feet (default 21); above it, Mochi is in the air and lifts his feet. */
  bot?: number;
  /** Horizontal radius, 8.2 for the silhouette's own width. */
  rx?: number;
  /** Vertical radius, 6.2 for the silhouette's own height. */
  ry?: number;
  /** Horizontal lean: units of shift per unit above the body's bottom. */
  shear?: number;
  face?: FaceId;
  /** Gaze: the face slides 0.8 unit per step, -1 looks left, 1 looks right. */
  look?: number;
  /** Eyes on a sphere (îlot's own, off in the reference): looking aside, they slide a
   * little further and narrow toward the edge. No change at all while `look` is 0. */
  sphere?: boolean;
  /** Face offset from its natural position. */
  lx?: number;
  ly?: number;
  /** Swing of the lock (Mèche) and of the tail (Oreilles), about -1 to 1. */
  sway?: number;
  /** Mèche raises his right arm and waves it, swinging with the clock. */
  wave?: boolean;
  /** `false` hides the ground shadow. */
  shadow?: boolean;
  /** Ground shadow width factor (default 1). */
  shadowK?: number;
  fx?: readonly Effect[];
}

/** Where the body stands in the grid, once the move's ratios are applied to the silhouette. */
export interface Body {
  readonly shape: BodyShape;
  readonly cx: number;
  /** Center row of the superellipse. */
  readonly cy: number;
  readonly rx: number;
  readonly ry: number;
  /** Lowest row of the body, where the feet are. */
  readonly bot: number;
  /** Highest row of the body. */
  readonly top: number;
  readonly shear: number;
}

const BASE_RX = 8.2;
const BASE_RY = 6.2;
const GROUND = 21;

/** The body of a frame: the silhouette scaled by the move, standing on `bot`. */
export function bodyOf(params: FrameParams): Body {
  const shape = BODIES[params.kind ?? DEFAULT_KIND];
  const rx = shape.rx * ((params.rx ?? BASE_RX) / BASE_RX);
  const ry = shape.ry * ((params.ry ?? BASE_RY) / BASE_RY);
  const bot = (params.bot ?? GROUND) - 0.6;
  const cy = bot - ry;
  return { shape, cx: params.cx ?? GRID_SIZE / 2, cy, rx, ry, bot, top: cy - ry, shear: params.shear || 0 };
}

/** Horizontal position of (`x`, `y`) once the body leans: rows above the bottom shift. */
export function lean(body: Body, x: number, y: number): number {
  return x + body.shear * (body.bot - y);
}

/**
 * Point `i` of the body's outline, out of `BODY_SEGMENTS`, clockwise on screen from the
 * right: a superellipse, squarer above or below its center according to the silhouette.
 */
export function outlinePoint(body: Body, i: number): readonly [x: number, y: number] {
  const a = (i / BODY_SEGMENTS) * Math.PI * 2;
  const ca = Math.cos(a);
  const sa = Math.sin(a);
  const p = sa > 0 ? body.shape.bottom : body.shape.top;
  const x = body.cx + body.rx * Math.sign(ca) * Math.abs(ca) ** (2 / p);
  const y = body.cy + body.ry * Math.sign(sa) * Math.abs(sa) ** (2 / p);
  return [lean(body, x, y), y];
}

/** Canvas pixels on each side for a Mochi of `scale` CSS pixels per unit, sharp on this screen. */
export function backingSize(scale: number, pixelRatio: number): number {
  return Math.round(GRID_SIZE * scale * Math.min(MAX_PIXEL_RATIO, pixelRatio || 1));
}

/** Where the face sits: the center between the eyes, on their row (same arithmetic as
 * the drawing below; outfits.ts hangs glasses and a bow tie on it). */
export function faceAnchor(params: FrameParams, body: Body = bodyOf(params)): { x: number; y: number } {
  return {
    x: lean(body, body.cx, body.cy) + (params.lx || 0) + (params.look || 0) * 0.8,
    y: body.cy - body.ry * 0.06 + (params.ly || 0),
  };
}

/** Gaze toward a point: -1 when `x` is left of the box, 1 when right of it, else 0. */
export function lookToward(x: number, left: number, right: number): number {
  return x < left ? -1 : x > right ? 1 : 0;
}

/**
 * Draws one frame over the whole `size` × `size` canvas behind `c`. `now` (milliseconds)
 * only swings the waving arm, so tests can fix it.
 */
export function drawMochi(c: CanvasRenderingContext2D, size: number, params: FrameParams, now: number): void {
  c.setTransform(size / GRID_SIZE, 0, 0, size / GRID_SIZE, 0, 0);
  c.clearRect(0, 0, GRID_SIZE, GRID_SIZE);
  // The reference leaves line caps and joins set from one frame to the next: here each
  // frame starts from the canvas defaults, whatever was drawn before (a Mèche, then an Écran).
  c.save();
  paint(c, params, now);
  c.restore();
}

/** The reference's drawing, call for call, on a context already mapped to the grid. */
function paint(c: CanvasRenderingContext2D, params: FrameParams, now: number): void {
  const kind = params.kind ?? DEFAULT_KIND;
  const { tiers, outline } = PALETTES[params.palette ?? DEFAULT_PALETTE];
  const body = bodyOf(params);
  const { cx, cy, rx, ry, bot, top } = body;
  const sway = params.sway || 0;
  const X = (x: number, y: number): number => lean(body, x, y);

  // Ground shadow, soft, always under the grid's center.
  if (params.shadow !== false) {
    const radius = rx * 0.82 * (params.shadowK ?? 1);
    const g = c.createRadialGradient(12, 23, 0, 12, 23, radius);
    g.addColorStop(0, INK.groundShadow);
    g.addColorStop(1, INK.noShadow);
    c.save();
    c.translate(12, 23);
    c.scale(1, 0.22);
    c.translate(-12, -23);
    c.fillStyle = g;
    c.beginPath();
    c.arc(12, 23, radius, 0, ROUND);
    c.fill();
    c.restore();
  }

  if (kind === "oreilles") {
    // Pointed ears with a pink inside, behind the body.
    for (const s of [-1, 1]) {
      const bx = cx + s * 4.4;
      const by = top + 1.8;
      c.beginPath();
      c.moveTo(X(bx - s * 2.6, by + 0.4), by + 0.4);
      c.quadraticCurveTo(X(bx + s * 0.2, by - 4.4), by - 4.6, X(bx + s * 0.9, by - 4.2), by - 4.2);
      c.quadraticCurveTo(X(bx + s * 1.6, by - 1.6), by - 1.2, X(bx + s * 1.8, by + 0.8), by + 0.8);
      c.closePath();
      const g = c.createLinearGradient(0, by - 4.5, 0, by + 1);
      g.addColorStop(0, tiers[1]);
      g.addColorStop(1, tiers[2]);
      c.fillStyle = g;
      c.fill();
      c.beginPath();
      c.moveTo(X(bx - s * 1.4, by), by);
      c.quadraticCurveTo(X(bx + s * 0.3, by - 3), by - 3.1, X(bx + s * 0.7, by - 2.8), by - 2.8);
      c.quadraticCurveTo(X(bx + s * 1.1, by - 1), by - 0.8, X(bx + s * 1, by + 0.4), by + 0.4);
      c.closePath();
      c.fillStyle = INK.earInside;
      c.fill();
    }
    // A tail whose tip follows the sway.
    c.beginPath();
    c.moveTo(X(cx + rx - 0.6, bot - 1.2), bot - 1.2);
    c.bezierCurveTo(cx + rx + 1.8, bot - 1, cx + rx + 1.6 + sway * 0.4, bot - 3.6, cx + rx + 0.6 + sway * 0.8, bot - 4.6);
    c.lineCap = "round";
    c.lineWidth = 1.3;
    c.strokeStyle = tiers[2];
    c.stroke();
  }

  // Feet, lifted a little while Mochi is in the air.
  const lift = (params.bot ?? GROUND) < GROUND ? -0.35 : 0;
  for (const s of [-1, 1]) {
    c.beginPath();
    c.ellipse(cx + s * 2.9, bot + 0.35 + lift, 1.7, 0.85, 0, 0, ROUND);
    c.fillStyle = tiers[3];
    c.fill();
  }

  if (kind === "meche") {
    // Two little arms; with `wave`, the right one is raised and swings.
    const ay = cy + 1.1;
    c.fillStyle = tiers[2];
    c.beginPath();
    c.ellipse(X(cx - rx - 0.1, ay), ay, 0.85, 1.3, 0.3, 0, ROUND);
    c.fill();
    if (params.wave) {
      c.save();
      c.translate(X(cx + rx - 0.2, ay - 1.6), ay - 1.6);
      c.rotate(-0.9 + 0.3 * Math.sin(now / 120));
      c.beginPath();
      c.ellipse(0.6, -0.4, 0.85, 1.4, 0, 0, ROUND);
      c.fill();
      c.restore();
    } else {
      c.beginPath();
      c.ellipse(X(cx + rx + 0.1, ay), ay, 0.85, 1.3, -0.3, 0, ROUND);
      c.fill();
    }
  }

  // Body: a smooth superellipse, lit from the top left, darker toward the ground.
  const points = Array.from({ length: BODY_SEGMENTS + 1 }, (_, i) => outlinePoint(body, i));
  const path = (): void => {
    c.beginPath();
    points.forEach(([x, y], i) => (i ? c.lineTo(x, y) : c.moveTo(x, y)));
    c.closePath();
  };
  path();
  let g = c.createRadialGradient(cx - rx * 0.35, cy - ry * 0.55, 0.5, cx, cy, rx * 1.35);
  g.addColorStop(0, tiers[0]);
  g.addColorStop(0.35, tiers[1]);
  g.addColorStop(0.72, tiers[2]);
  g.addColorStop(1, tiers[3]);
  c.fillStyle = g;
  c.fill();
  c.save();
  path();
  c.clip();
  g = c.createLinearGradient(0, cy, 0, bot);
  g.addColorStop(0, INK.noShadow);
  g.addColorStop(1, rgba(outline, 0.28));
  c.fillStyle = g;
  c.fillRect(0, cy, GRID_SIZE, ry + 1);
  g = c.createLinearGradient(0, top, 0, cy);
  g.addColorStop(0, INK.topLight);
  g.addColorStop(1, INK.noLight);
  c.fillStyle = g;
  c.fillRect(0, top, GRID_SIZE, 2);
  c.restore();
  path();
  c.lineWidth = 0.32;
  c.strokeStyle = rgba(outline, 0.45);
  c.stroke();

  // Glossy highlight: a tilted oval and a dot.
  c.save();
  c.translate(X(cx - rx * 0.48, cy - ry * 0.55), cy - ry * 0.55);
  c.rotate(-0.5);
  c.beginPath();
  c.ellipse(0, 0, rx * 0.24, ry * 0.15, 0, 0, ROUND);
  c.fillStyle = INK.shine;
  c.fill();
  c.beginPath();
  c.arc(rx * 0.3, ry * 0.12, 0.35, 0, ROUND);
  c.fill();
  c.restore();

  if (kind === "meche") {
    // A loop of the body's own matter on top, swinging with `sway`, with a light streak.
    const tx = X(cx + 0.3, top);
    const ty = top + 0.5;
    c.beginPath();
    c.moveTo(tx, ty);
    c.bezierCurveTo(tx - 0.2, ty - 2.2, tx + 2.6 + sway * 0.6, ty - 3.2, tx + 2.4 + sway * 0.7, ty - 1.6);
    c.bezierCurveTo(tx + 2.2 + sway * 0.6, ty - 0.6, tx + 1, ty - 0.9, tx + 1.2, ty - 1.8);
    c.lineCap = "round";
    c.lineJoin = "round";
    c.lineWidth = 1.25;
    c.strokeStyle = tiers[2];
    c.stroke();
    c.lineWidth = 0.45;
    c.strokeStyle = rgba("#ffffff", 0.55);
    c.beginPath();
    c.moveTo(tx + 0.2, ty - 1.6);
    c.quadraticCurveTo(tx + 1, ty - 2.8, tx + 2, ty - 2.6);
    c.stroke();
  }

  const face = params.face ?? "idle";
  const fcx = X(cx, cy) + (params.lx || 0) + (params.look || 0) * 0.8;
  const ey = cy - ry * 0.06 + (params.ly || 0);
  if (kind === "ecran") {
    // A dark glossy visor, a bolt on each side, and eyes that light up.
    const x0 = fcx - 5.2;
    const y0 = ey - 2.7;
    const h = 5.4;
    c.beginPath();
    c.roundRect(x0, y0, 10.4, h, 2.3);
    c.fillStyle = SCREEN.visor;
    c.fill();
    g = c.createLinearGradient(0, y0, 0, y0 + h);
    g.addColorStop(0, SCREEN.gloss);
    g.addColorStop(0.35, INK.noLight);
    c.fillStyle = g;
    c.fill();
    for (const s of [-1, 1]) {
      c.beginPath();
      c.arc(cx + s * (rx - 0.2), ey, 0.55, 0, ROUND);
      c.fillStyle = tiers[3];
      c.fill();
    }
    drawFace(c, face, fcx, ey, true, params.sphere ? params.look || 0 : 0);
  } else {
    drawFace(c, face, fcx, ey, false, params.sphere ? params.look || 0 : 0);
  }

  for (const [x, y, color] of params.fx ?? []) {
    c.fillStyle = color;
    c.beginPath();
    c.roundRect(x + 0.1, y + 0.1, 0.8, 0.8, 0.25);
    c.fill();
  }
}
