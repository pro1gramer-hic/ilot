// Mochi's wardrobe (prompt 12, lot 1): hats, glasses, a scarf or a bow tie, headphones,
// one piece per slot, drawn in vectors over `drawMochi` on the same 24-unit grid. Each
// piece reads the frame's own body (bodyOf, faceAnchor), so it fits the three
// silhouettes and follows every move: breathing, dancing, jumping, stretching. Season
// outfits fill the empty slots on their dates unless the user turns them off.

import { texts } from "../i18n/live.svelte";
import { bodyOf, drawMochi, faceAnchor, type Body, type FrameParams } from "./engine";
import { EYE_GAP, ROUND, onSphere } from "./faces";

export type Slot = "head" | "eyes" | "neck" | "ears";

/** A slot, named in the language of the moment. */
function slot(id: Slot): { id: Slot; label: string } {
  return {
    id,
    get label() {
      return texts().mochi.slots[id];
    },
  };
}

export const SLOTS: readonly { id: Slot; label: string }[] = [slot("head"), slot("eyes"), slot("neck"), slot("ears")];

export type PieceId =
  | "bonnet"
  | "noel"
  | "casquette"
  | "sorciere"
  | "couronne"
  | "fete"
  | "fleur"
  | "rondes"
  | "soleil"
  | "echarpe"
  | "noeud"
  | "casque"
  | "cacheoreilles";

/** A piece: its slot, and its name in the language of the moment. */
function piece(id: PieceId, slot: Slot): { slot: Slot; label: string } {
  return {
    slot,
    get label() {
      return texts().mochi.pieces[id];
    },
  };
}

export const PIECES: Readonly<Record<PieceId, { slot: Slot; label: string }>> = {
  bonnet: piece("bonnet", "head"),
  noel: piece("noel", "head"),
  casquette: piece("casquette", "head"),
  sorciere: piece("sorciere", "head"),
  couronne: piece("couronne", "head"),
  fete: piece("fete", "head"),
  fleur: piece("fleur", "head"),
  rondes: piece("rondes", "eyes"),
  soleil: piece("soleil", "eyes"),
  echarpe: piece("echarpe", "neck"),
  noeud: piece("noeud", "neck"),
  casque: piece("casque", "ears"),
  cacheoreilles: piece("cacheoreilles", "ears"),
};

export const PIECE_IDS = Object.keys(PIECES) as PieceId[];

/** The pieces of each slot, in the wardrobe's order. */
export function piecesOf(slot: Slot): PieceId[] {
  return PIECE_IDS.filter((id) => PIECES[id].slot === slot);
}

/** What Mochi wears: at most one piece per slot. */
export type Outfit = Partial<Record<Slot, PieceId>>;

/** A slot kept bare on purpose: the season's outfit does not fill it. */
export const BARE = "aucun";

/** The choices of the settings: a piece, or `null` for a slot kept bare; a free slot is
 * absent (the season may fill it). */
export type Choice = Partial<Record<Slot, PieceId | null>>;

/** The pieces chosen in the settings ("" free, "aucun" bare); anything unknown or
 * misplaced leaves its slot free. */
export function chosenOutfit(choice: Partial<Record<Slot, string>>): Choice {
  const chosen: Choice = {};
  for (const { id } of SLOTS) {
    const piece = choice[id];
    if (piece === BARE) chosen[id] = null;
    else if (piece && piece in PIECES && PIECES[piece as PieceId].slot === id) chosen[id] = piece as PieceId;
  }
  return chosen;
}

// ---------- seasons ----------

export type SeasonId = "nouvelan" | "noel" | "halloween" | "hiver" | "printemps" | "ete";

/** A season: its name in the language of the moment, and its outfit. */
function season(id: SeasonId, outfit: Outfit): { label: string; outfit: Outfit } {
  return {
    get label() {
      return texts().mochi.seasons[id];
    },
    outfit,
  };
}

export const SEASONS: Readonly<Record<SeasonId, { label: string; outfit: Outfit }>> = {
  nouvelan: season("nouvelan", { head: "fete" }),
  noel: season("noel", { head: "noel", neck: "echarpe" }),
  halloween: season("halloween", { head: "sorciere" }),
  hiver: season("hiver", { head: "bonnet", neck: "echarpe" }),
  printemps: season("printemps", { head: "fleur" }),
  ete: season("ete", { eyes: "soleil" }),
};

export const SEASON_IDS = Object.keys(SEASONS) as SeasonId[];

/** Each season's days, as [month, day] from and to (inclusive, months 1 to 12), in
 * order of precedence: the first one that holds wins. */
const SEASON_DAYS: readonly [SeasonId, [number, number], [number, number]][] = [
  ["nouvelan", [12, 31], [12, 31]],
  ["nouvelan", [1, 1], [1, 1]],
  ["noel", [12, 18], [12, 26]],
  ["halloween", [10, 24], [11, 1]],
  ["hiver", [12, 1], [12, 31]],
  ["hiver", [1, 1], [2, 29]],
  ["printemps", [3, 20], [5, 31]],
  ["ete", [6, 21], [8, 31]],
];

/** The season of a day (local date), or none. */
export function seasonOf(date: Date): SeasonId | null {
  const day = (date.getMonth() + 1) * 100 + date.getDate();
  const found = SEASON_DAYS.find(([, [m1, d1], [m2, d2]]) => day >= m1 * 100 + d1 && day <= m2 * 100 + d2);
  return found ? found[0] : null;
}

/** What Mochi wears now: the chosen pieces, and the season's in the free slots. */
export function wornOutfit(chosen: Choice, seasonal: boolean, date: Date): Outfit {
  const season = seasonal ? seasonOf(date) : null;
  const worn: Outfit = {};
  for (const { id } of SLOTS) {
    const piece = id in chosen ? chosen[id] : season ? SEASONS[season].outfit[id] : undefined;
    if (piece) worn[id] = piece;
  }
  return worn;
}

// ---------- drawing ----------

/** Colors of the pieces: fabrics and metals of their own, the same in every theme (like
 * Mochi's palettes, they are part of the drawing). */
const INKS = {
  wool: "#E8604C",
  woolShade: "#B9402F",
  cream: "#F4ECE1",
  creamShade: "#D9CDBE",
  santa: "#D8393B",
  santaShade: "#A72830",
  navy: "#34407A",
  navyShade: "#232C5A",
  witch: "#33284A",
  witchShade: "#1F1830",
  pumpkin: "#F08A24",
  buckle: "#F7C948",
  gold: "#F2C14E",
  goldShade: "#C9922B",
  ruby: "#E5484D",
  sapphire: "#4F8EF7",
  emerald: "#3DBE85",
  partyPink: "#FF6FA8",
  partyYellow: "#FFD25A",
  petal: "#FF8FB1",
  petalShade: "#E9668F",
  leaf: "#5BBF7A",
  frame: "#5A3E2B",
  lens: "rgba(255,255,255,0.16)",
  glint: "rgba(255,255,255,0.65)",
  shades: "#1E1B2E",
  shadesLight: "#4A4268",
  shadesFrame: "#121019",
  bow: "#C93B5A",
  bowShade: "#932842",
  phones: "#2B2D3A",
  phonesLight: "#4E5370",
  phonesDot: "#FF7A59",
  muff: "#F5A9C2",
  muffShade: "#D97FA0",
  muffBand: "#C9C2D6",
} as const;

/** Half the body's width at row `y` (its superellipse), 0 above or below it. */
export function halfWidthAt(body: Body, y: number): number {
  const dy = Math.abs(y - body.cy);
  if (dy >= body.ry) return 0;
  const p = y > body.cy ? body.shape.bottom : body.shape.top;
  return body.rx * (1 - (dy / body.ry) ** p) ** (1 / p);
}

/** Everything a piece needs to know about the frame. */
type Fit = {
  c: CanvasRenderingContext2D;
  body: Body;
  /** Where the face is (between the eyes) and the gaze, for the glasses. */
  face: { x: number; y: number };
  look: number;
  sphere: boolean;
  /** Swing of the soft parts (pompom, scarf end, a hat's tip), 0 when still. */
  swing: number;
};

function fill(c: CanvasRenderingContext2D, color: string | CanvasGradient): void {
  c.fillStyle = color;
  c.fill();
}

function vertical(c: CanvasRenderingContext2D, top: number, bottom: number, from: string, to: string): CanvasGradient {
  const g = c.createLinearGradient(0, top, 0, bottom);
  g.addColorStop(0, from);
  g.addColorStop(1, to);
  return g;
}

/** A soft ball: a disc and a ring of bumps (pompoms, muffs). */
function fluff(c: CanvasRenderingContext2D, x: number, y: number, r: number, color: string, shade: string): void {
  c.beginPath();
  for (let i = 0; i < 9; i++) {
    const a = (i / 9) * Math.PI * 2;
    c.moveTo(x + Math.cos(a) * r * 0.82 + r * 0.32, y + Math.sin(a) * r * 0.82);
    c.arc(x + Math.cos(a) * r * 0.82, y + Math.sin(a) * r * 0.82, r * 0.32, 0, ROUND);
  }
  c.moveTo(x + r * 0.86, y);
  c.arc(x, y, r * 0.86, 0, ROUND);
  fill(c, vertical(c, y - r, y + r, color, shade));
}

/** The row where a hat's base sits and how wide it is there. */
function hatBase(body: Body, depth: number): { x: number; y: number; w: number } {
  const y = body.top + depth;
  return { x: body.cx, y, w: halfWidthAt(body, y) + 0.3 };
}

function drawBonnet({ c, body, swing }: Fit): void {
  const { x, y, w } = hatBase(body, 1.5);
  const h = 3.3;
  c.beginPath();
  c.moveTo(x - w, y);
  c.bezierCurveTo(x - w, y - h * 0.9, x - w * 0.55, y - h * 1.05, x, y - h);
  c.bezierCurveTo(x + w * 0.55, y - h * 1.05, x + w, y - h * 0.9, x + w, y);
  c.closePath();
  fill(c, vertical(c, y - h, y, INKS.wool, INKS.woolShade));
  // Knitted ribs.
  c.strokeStyle = "rgba(0,0,0,0.12)";
  c.lineWidth = 0.18;
  for (const k of [-0.5, -0.17, 0.17, 0.5]) {
    c.beginPath();
    c.moveTo(x + w * k, y - 0.4);
    c.quadraticCurveTo(x + w * k * 0.9, y - h * 0.7, x + w * k * 0.45, y - h * 0.95);
    c.stroke();
  }
  // The folded brim.
  c.beginPath();
  c.roundRect(x - w - 0.25, y - 1.35, 2 * w + 0.5, 1.6, 0.8);
  fill(c, vertical(c, y - 1.35, y + 0.25, INKS.cream, INKS.creamShade));
  fluff(c, x + swing * 0.35, y - h - 0.55, 1.05, INKS.cream, INKS.creamShade);
}

function drawSanta({ c, body, swing }: Fit): void {
  const { x, y, w } = hatBase(body, 1.5);
  // A soft cone that flops to the right, its pompom hanging.
  const tip = { x: x + w + 1.1 + swing * 0.3, y: y - 1.9 };
  c.beginPath();
  c.moveTo(x - w, y);
  c.bezierCurveTo(x - w * 0.9, y - 3.2, x + 0.4, y - 4.6, x + w * 0.7, y - 3.4);
  c.quadraticCurveTo(tip.x - 0.4, y - 3.1, tip.x, tip.y);
  c.quadraticCurveTo(x + w * 0.75, y - 2.2, x + w, y);
  c.closePath();
  fill(c, vertical(c, y - 4.4, y, INKS.santa, INKS.santaShade));
  c.beginPath();
  c.roundRect(x - w - 0.3, y - 1.3, 2 * w + 0.6, 1.7, 0.85);
  fill(c, vertical(c, y - 1.3, y + 0.4, INKS.cream, INKS.creamShade));
  fluff(c, tip.x, tip.y + 0.3, 0.95, INKS.cream, INKS.creamShade);
}

function drawCap({ c, body }: Fit): void {
  const { x, y, w } = hatBase(body, 1.4);
  const h = 3.2;
  // A shadow on the forehead, under the visor.
  c.beginPath();
  c.ellipse(x + 0.6, y + 0.9, w * 0.62, 0.7, 0, 0, ROUND);
  fill(c, "rgba(0,0,0,0.12)");
  c.beginPath();
  c.moveTo(x - w * 0.92, y);
  c.bezierCurveTo(x - w * 0.95, y - h * 0.95, x - w * 0.45, y - h * 1.05, x, y - h);
  c.bezierCurveTo(x + w * 0.45, y - h * 1.05, x + w * 0.95, y - h * 0.95, x + w * 0.92, y);
  c.closePath();
  fill(c, vertical(c, y - h, y, INKS.navy, INKS.navyShade));
  // Seams and the button on top.
  c.strokeStyle = "rgba(255,255,255,0.16)";
  c.lineWidth = 0.16;
  for (const k of [-0.42, 0, 0.42]) {
    c.beginPath();
    c.moveTo(x, y - h);
    c.quadraticCurveTo(x + w * k * 0.8, y - h * 0.55, x + w * k, y - 0.2);
    c.stroke();
  }
  c.beginPath();
  c.arc(x, y - h, 0.4, 0, ROUND);
  fill(c, INKS.navyShade);
  // The visor, pointing forward and a little to the side: a flat crescent, lit on top.
  c.beginPath();
  c.moveTo(x - w * 0.55, y - 0.15);
  c.quadraticCurveTo(x + w * 0.2, y + 2.1, x + w * 1.25, y + 0.35);
  c.quadraticCurveTo(x + w * 0.5, y + 0.5, x - w * 0.55, y - 0.15);
  c.closePath();
  fill(c, vertical(c, y - 0.2, y + 1.5, INKS.navy, INKS.navyShade));
  c.beginPath();
  c.moveTo(x - w * 0.5, y - 0.1);
  c.quadraticCurveTo(x + w * 0.5, y + 0.45, x + w * 1.2, y + 0.35);
  c.lineWidth = 0.18;
  c.strokeStyle = "rgba(255,255,255,0.22)";
  c.stroke();
}

function drawWitch({ c, body, swing }: Fit): void {
  const { x, y, w } = hatBase(body, 1.2);
  // A wide brim, a tall cone whose tip bends with him.
  const bend = 1.2 + swing * 0.4;
  c.beginPath();
  c.ellipse(x, y - 0.25, w + 2.3, 1.05, 0, 0, ROUND);
  fill(c, vertical(c, y - 1.3, y + 0.8, INKS.witch, INKS.witchShade));
  c.beginPath();
  c.moveTo(x - w * 0.6, y - 0.5);
  c.quadraticCurveTo(x - 0.6, y - 3.8, x + bend * 0.5, y - 5.6);
  c.quadraticCurveTo(x + bend * 1.4, y - 6.2, x + bend * 1.9, y - 5.2);
  c.quadraticCurveTo(x + bend * 0.9, y - 5, x + 0.9, y - 3.6);
  c.quadraticCurveTo(x + w * 0.62, y - 2, x + w * 0.62, y - 0.5);
  c.closePath();
  fill(c, vertical(c, y - 6, y, INKS.witch, INKS.witchShade));
  // An orange band and its buckle.
  c.beginPath();
  c.moveTo(x - w * 0.6, y - 0.6);
  c.lineTo(x + w * 0.62, y - 0.6);
  c.lineTo(x + w * 0.58, y - 1.55);
  c.lineTo(x - w * 0.55, y - 1.55);
  c.closePath();
  fill(c, INKS.pumpkin);
  c.lineWidth = 0.22;
  c.strokeStyle = INKS.buckle;
  c.strokeRect(x - 0.45, y - 1.45, 0.9, 0.75);
}

function drawCrown({ c, body }: Fit): void {
  const { x, y, w } = hatBase(body, 1.1);
  const half = Math.max(2.4, w - 1.1);
  const base = y + 0.1;
  const peak = y - 2.9;
  c.beginPath();
  c.moveTo(x - half, base);
  c.lineTo(x - half, y - 1.2);
  const spikes = 5;
  for (let i = 0; i < spikes; i++) {
    const left = x - half + (2 * half * i) / spikes;
    const mid = left + half / spikes;
    const right = left + (2 * half) / spikes;
    c.lineTo(mid, peak + (i % 2 ? 0.6 : 0));
    c.lineTo(right, y - 1.2);
  }
  c.lineTo(x + half, base);
  c.closePath();
  c.lineJoin = "round";
  fill(c, vertical(c, peak, base, INKS.gold, INKS.goldShade));
  c.lineWidth = 0.15;
  c.strokeStyle = INKS.goldShade;
  c.stroke();
  [INKS.sapphire, INKS.ruby, INKS.emerald].forEach((gem, i) => {
    c.beginPath();
    c.arc(x + (i - 1) * half * 0.6, y - 0.55, 0.36, 0, ROUND);
    fill(c, gem);
  });
}

function drawPartyHat({ c, body, swing }: Fit): void {
  const { x, y } = hatBase(body, 1.0);
  const left = x - 0.9;
  const right = x + 3.1;
  const tip = { x: x + 1.9 + swing * 0.25, y: y - 5 };
  c.save();
  c.beginPath();
  c.moveTo(left, y);
  c.lineTo(tip.x, tip.y);
  c.lineTo(right, y);
  c.closePath();
  fill(c, INKS.partyPink);
  c.clip();
  c.fillStyle = INKS.partyYellow;
  for (let k = 0; k < 4; k++) {
    const sy = y - 0.5 - k * 1.35;
    c.beginPath();
    c.moveTo(left - 1, sy + 0.9);
    c.lineTo(right + 1, sy - 0.2);
    c.lineTo(right + 1, sy - 0.75);
    c.lineTo(left - 1, sy + 0.35);
    c.closePath();
    c.fill();
  }
  c.restore();
  fluff(c, tip.x, tip.y, 0.7, INKS.cream, INKS.partyYellow);
}

function drawFlower({ c, body, swing }: Fit): void {
  const { x, y, w } = hatBase(body, 0.9);
  const fx = x + w * 0.5;
  const fy = y - 0.5;
  // Tilted a little, and a little more as he sways.
  const tilt = -0.25 + swing * 0.12;
  c.beginPath();
  c.ellipse(fx - 1.1 * Math.cos(tilt + 0.8), fy + 1.1 * Math.sin(tilt + 0.8), 1.1, 0.42, tilt - 0.6, 0, ROUND);
  fill(c, INKS.leaf);
  for (let i = 0; i < 5; i++) {
    const a = (i / 5) * Math.PI * 2 - Math.PI / 2 + tilt;
    c.beginPath();
    c.ellipse(fx + Math.cos(a) * 0.75, fy + Math.sin(a) * 0.75, 0.62, 0.48, a, 0, ROUND);
    fill(c, i % 2 ? INKS.petalShade : INKS.petal);
  }
  c.beginPath();
  c.arc(fx, fy, 0.42, 0, ROUND);
  fill(c, INKS.partyYellow);
}

/** Glasses sit on the eyes, which may slide and narrow on the sphere: each lens goes
 * with its eye. */
function lens(fit: Fit, side: -1 | 1, draw: (x: number, y: number) => void): void {
  const x = fit.face.x + side * EYE_GAP;
  const y = fit.face.y;
  if (fit.sphere && fit.look !== 0) onSphere(fit.c, fit.look, side, x, y, () => draw(x, y));
  else draw(x, y);
}

/** Arms of the glasses: from each lens to the side of the head. */
function temples(fit: Fit, reach: number, color: string): void {
  const { c, body, face } = fit;
  const edge = halfWidthAt(body, face.y - 0.3) - 0.15;
  c.strokeStyle = color;
  c.lineWidth = 0.3;
  for (const side of [-1, 1]) {
    const from = face.x + side * (EYE_GAP + reach);
    const to = body.cx + side * edge;
    if (side * (to - from) <= 0) continue;
    c.beginPath();
    c.moveTo(from, face.y - 0.3);
    c.lineTo(to, face.y - 0.45);
    c.stroke();
  }
}

function drawRoundGlasses(fit: Fit): void {
  const { c, face } = fit;
  temples(fit, 1.75, INKS.frame);
  for (const side of [-1, 1] as const) {
    lens(fit, side, (x, y) => {
      c.beginPath();
      c.arc(x, y, 1.75, 0, ROUND);
      fill(c, INKS.lens);
      c.lineWidth = 0.36;
      c.strokeStyle = INKS.frame;
      c.stroke();
      c.beginPath();
      c.arc(x, y, 1.25, Math.PI * 1.1, Math.PI * 1.4);
      c.lineWidth = 0.2;
      c.strokeStyle = INKS.glint;
      c.stroke();
    });
  }
  c.beginPath();
  c.moveTo(face.x - EYE_GAP + 1.7, face.y - 0.35);
  c.quadraticCurveTo(face.x, face.y - 1.1, face.x + EYE_GAP - 1.7, face.y - 0.35);
  c.lineWidth = 0.32;
  c.strokeStyle = INKS.frame;
  c.stroke();
}

function drawSunglasses(fit: Fit): void {
  const { c, face } = fit;
  temples(fit, 1.75, INKS.shadesFrame);
  for (const side of [-1, 1] as const) {
    lens(fit, side, (x, y) => {
      c.beginPath();
      c.roundRect(x - 1.8, y - 1.25, 3.6, 2.6, 1.05);
      fill(c, vertical(c, y - 1.25, y + 1.35, INKS.shades, INKS.shadesLight));
      c.lineWidth = 0.28;
      c.strokeStyle = INKS.shadesFrame;
      c.stroke();
      c.beginPath();
      c.moveTo(x - 1.0, y - 0.75);
      c.lineTo(x - 0.1, y - 0.75);
      c.lineTo(x - 0.9, y + 0.45);
      c.closePath();
      fill(c, "rgba(255,255,255,0.28)");
    });
  }
  c.beginPath();
  c.moveTo(face.x - EYE_GAP + 1.75, face.y - 0.55);
  c.lineTo(face.x + EYE_GAP - 1.75, face.y - 0.55);
  c.lineWidth = 0.34;
  c.strokeStyle = INKS.shadesFrame;
  c.stroke();
}

/** The neck row: a little below the mouth, wherever the face is. */
function neckRow(fit: Fit): number {
  return fit.face.y + 4;
}

function drawScarf(fit: Fit): void {
  const { c, body, swing } = fit;
  const y = neckRow(fit);
  const hw = halfWidthAt(body, y) + 0.25;
  const x = body.cx;
  const band = (): void => {
    c.beginPath();
    c.moveTo(x - hw, y - 0.7);
    c.quadraticCurveTo(x, y + 0.2, x + hw, y - 0.7);
    c.lineTo(x + hw * 0.96, y + 0.8);
    c.quadraticCurveTo(x, y + 1.75, x - hw * 0.96, y + 0.8);
    c.closePath();
  };
  // The end that hangs, under the band, swinging a little.
  const ex = x + hw * 0.42;
  const sway = swing * 0.3;
  c.beginPath();
  c.moveTo(ex - 0.7, y + 0.6);
  c.lineTo(ex + 0.7, y + 0.6);
  c.lineTo(ex + 0.75 + sway, y + 3.3);
  c.lineTo(ex - 0.6 + sway, y + 3.3);
  c.closePath();
  fill(c, vertical(c, y, y + 3.3, INKS.wool, INKS.woolShade));
  c.strokeStyle = INKS.cream;
  c.lineWidth = 0.18;
  for (const k of [-0.35, 0.05, 0.45]) {
    c.beginPath();
    c.moveTo(ex + k + sway, y + 3.3);
    c.lineTo(ex + k + sway * 1.2, y + 3.85);
    c.stroke();
  }
  band();
  fill(c, vertical(c, y - 0.7, y + 1.7, INKS.wool, INKS.woolShade));
  c.save();
  band();
  c.clip();
  c.fillStyle = INKS.cream;
  for (let sx = x - hw + 1.2; sx < x + hw; sx += 2.3) c.fillRect(sx, y - 1, 0.55, 3);
  c.restore();
  c.beginPath();
  c.ellipse(ex, y + 0.55, 0.85, 0.65, 0, 0, ROUND);
  fill(c, INKS.woolShade);
}

function drawBowTie(fit: Fit): void {
  const { c, body } = fit;
  const y = neckRow(fit) + 0.1;
  const x = body.cx;
  for (const side of [-1, 1]) {
    c.beginPath();
    c.moveTo(x + side * 0.3, y);
    c.quadraticCurveTo(x + side * 1.2, y - 1.3, x + side * 2.1, y - 1.0);
    c.quadraticCurveTo(x + side * 2.4, y, x + side * 2.1, y + 1.0);
    c.quadraticCurveTo(x + side * 1.2, y + 1.3, x + side * 0.3, y);
    c.closePath();
    fill(c, vertical(c, y - 1.2, y + 1.2, INKS.bow, INKS.bowShade));
  }
  c.beginPath();
  c.roundRect(x - 0.5, y - 0.6, 1, 1.2, 0.4);
  fill(c, INKS.bowShade);
}

/** Where the headphones and muffs sit: on each side of the head, a little above the eyes. */
function earRow(fit: Fit): { y: number; hw: number } {
  const y = fit.body.cy - fit.body.ry * 0.28;
  return { y, hw: halfWidthAt(fit.body, y) - 0.15 };
}

function drawHeadphones(fit: Fit): void {
  const { c, body } = fit;
  const { y, hw } = earRow(fit);
  const x = body.cx;
  c.beginPath();
  c.moveTo(x - hw, y - 1.3);
  c.bezierCurveTo(x - hw, body.top - 2.4, x + hw, body.top - 2.4, x + hw, y - 1.3);
  c.lineCap = "round";
  c.lineWidth = 0.75;
  c.strokeStyle = INKS.phones;
  c.stroke();
  c.lineWidth = 0.22;
  c.strokeStyle = INKS.phonesLight;
  c.stroke();
  for (const side of [-1, 1]) {
    const cx = x + side * hw;
    c.beginPath();
    c.roundRect(cx - 0.9, y - 1.65, 1.8, 3.3, 0.85);
    fill(c, vertical(c, y - 1.65, y + 1.65, INKS.phonesLight, INKS.phones));
    c.beginPath();
    c.arc(cx + side * 0.15, y, 0.32, 0, ROUND);
    fill(c, INKS.phonesDot);
  }
}

function drawEarmuffs(fit: Fit): void {
  const { c, body } = fit;
  const { y, hw } = earRow(fit);
  const x = body.cx;
  c.beginPath();
  c.moveTo(x - hw, y - 1);
  c.bezierCurveTo(x - hw, body.top - 2.1, x + hw, body.top - 2.1, x + hw, y - 1);
  c.lineCap = "round";
  c.lineWidth = 0.42;
  c.strokeStyle = INKS.muffBand;
  c.stroke();
  for (const side of [-1, 1]) fluff(c, x + side * hw, y, 1.55, INKS.muff, INKS.muffShade);
}

const DRAW: Readonly<Record<PieceId, (fit: Fit) => void>> = {
  bonnet: drawBonnet,
  noel: drawSanta,
  casquette: drawCap,
  sorciere: drawWitch,
  couronne: drawCrown,
  fete: drawPartyHat,
  fleur: drawFlower,
  rondes: drawRoundGlasses,
  soleil: drawSunglasses,
  echarpe: drawScarf,
  noeud: drawBowTie,
  casque: drawHeadphones,
  cacheoreilles: drawEarmuffs,
};

/** Back to front: what goes around the neck, then the ears, the head, the eyes. */
const ORDER: readonly Slot[] = ["neck", "ears", "head", "eyes"];

/**
 * Draws `outfit` over the frame already drawn on `c` (the grid mapped by drawMochi).
 * Pieces lean with the body; glasses follow the eyes. `animated` lets the soft parts
 * swing with him (never in a still frame, so never with reduced motion).
 */
export function drawOutfit(c: CanvasRenderingContext2D, params: FrameParams, outfit: Outfit, animated: boolean): void {
  if (!ORDER.some((slot) => outfit[slot])) return;
  const body = bodyOf(params);
  const fit: Fit = {
    c,
    body,
    face: faceAnchor(params, body),
    look: params.look || 0,
    sphere: params.sphere === true,
    swing: animated ? params.sway || 0 : 0,
  };
  c.save();
  for (const slot of ORDER) {
    const piece = outfit[slot];
    if (!piece) continue;
    c.save();
    // Hats, scarf and headphones lean with the body, as its outline does.
    if (slot !== "eyes") c.transform(1, 0, -body.shear, 1, body.shear * body.bot, 0);
    DRAW[piece](fit);
    c.restore();
  }
  c.restore();
}

/** One frame of Mochi wearing `outfit`, over the whole canvas (see drawMochi). */
export function drawDressed(
  c: CanvasRenderingContext2D,
  size: number,
  params: FrameParams,
  outfit: Outfit,
  now: number,
  animated: boolean,
): void {
  drawMochi(c, size, params, now);
  drawOutfit(c, params, outfit, animated);
}
