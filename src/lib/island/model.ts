// The island's shapes and their geometry (docs/09-DEMO-FINALE.md, sections 3 and 4):
// a pill by default, and one opening per kind of event, each with its own size. The
// large panel is a card of its own, below the pill or above it.

import type { HubView } from "../hub";
import { texts } from "../i18n/live.svelte";
import type { Modes } from "../modes";

export type IslandMode =
  | "pill"
  | "finished"
  | "failed"
  | "permission"
  | "message"
  | "music"
  | "track"
  | "drop"
  | "file"
  | "toast"
  | "menu"
  | "wardrobe"
  | "voice"
  | "translation";

export type Tab =
  | "today"
  | "agents"
  | "messages"
  | "music"
  | "pastilles"
  | "chat"
  | "mochi"
  | "themes"
  | "modes"
  | "sound"
  | "general"
  | "shortcuts";

/** The panel's tabs, in order, named in the language of the moment. */
export const TABS: readonly { id: Tab; label: string }[] = (
  ["today", "agents", "messages", "music", "pastilles", "chat", "mochi", "themes", "modes", "sound", "general", "shortcuts"] as const
).map((id) => ({
  id,
  get label() {
    return texts().common.tabs[id];
  },
}));

export type Size = { width: number; height: number; radius: number };

/** Logical size of the transparent window that hosts the island (see window.rs). */
export const WINDOW_WIDTH = 980;
export const WINDOW_HEIGHT = 720;
/** The pill's height; its center is the anchor the core places. */
export const BAR_HEIGHT = 40;
/** Gap between the island and the panel, and around everything in the window. */
export const PANEL_GAP = 16;
export const EDGE = 8;
export const PANEL_MAX: Size = { width: 920, height: 620, radius: 24 };
export const MENU_WIDTH = 216;
/** Mochi's wardrobe, opened by a right click on him (prompt 12, lot 1). */
export const WARDROBE_WIDTH = 540;

/** The pill's width for each « Agents dans la pilule » mode, plus room for a badge. */
const PILL_WIDTH: Record<Modes["agents"], number> = { compteur: 220, points: 156, activite: 340 };
const BADGE_WIDTH = 26;

export function pillSize(agents: Modes["agents"], badge: boolean): Size {
  return { width: PILL_WIDTH[agents] + (badge ? BADGE_WIDTH : 0), height: BAR_HEIGHT, radius: BAR_HEIGHT / 2 };
}

/** Fixed openings: [width, height, radius]; a height of 0 fits the content. */
const OPENINGS: Record<Exclude<IslandMode, "pill">, [number, number, number]> = {
  finished: [440, 64, 32],
  failed: [540, 64, 32],
  permission: [560, 0, 22],
  message: [500, 0, 24],
  music: [500, 96, 30],
  track: [420, 56, 28],
  drop: [340, 88, 26],
  /** A file just swallowed, with what to do with it (prompt 12, lot 3). */
  file: [480, 0, 26],
  toast: [440, 52, 26],
  menu: [MENU_WIDTH, 0, 18],
  wardrobe: [WARDROBE_WIDTH, 0, 24],
  voice: [500, 0, 24],
  /** The clipboard and its DeepL translation (prompt 12), laid out like a message. */
  translation: [500, 0, 24],
};

/** A risky command opens wider (620 px). */
export const RISKY_WIDTH = 620;
/** Openings that fit their content never grow past this (the permission scrolls inside). */
export const MAX_OPEN_HEIGHT = 480;

export type Measures = { contentHeight: number };

export type SizeInputs = { modes: Modes; badge: boolean; risky: boolean; measures: Measures };

/** Size of the island; openings without a fixed height fit their measured content. */
export function sizeFor(mode: IslandMode, { modes, badge, risky, measures }: SizeInputs): Size {
  if (mode === "pill") return pillSize(modes.agents, badge);
  const [width, fixed, radius] = OPENINGS[mode];
  const height = fixed || Math.min(MAX_OPEN_HEIGHT, Math.max(BAR_HEIGHT * 2, Math.ceil(measures.contentHeight)));
  return {
    width: mode === "permission" && risky ? RISKY_WIDTH : width,
    height,
    radius: Math.min(radius, height / 2),
  };
}

/** Where the pill sits in its window, and which way the island opens (core: `island:placed`). */
export type Placement = { anchorX: number; anchorY: number; opensDown: boolean };

/** Before the core speaks: top center of the window, opening down. */
export const DEFAULT_PLACEMENT: Placement = { anchorX: WINDOW_WIDTH / 2, anchorY: EDGE + BAR_HEIGHT / 2, opensDown: true };

export type Box = { x: number; y: number; width: number; height: number };

/** The island's box in its window: around the anchor, opening down or up, never over
 * the window's edges. */
export function islandBox(size: Size, { anchorX, anchorY, opensDown }: Placement): Box {
  const x = Math.min(Math.max(EDGE, anchorX - size.width / 2), WINDOW_WIDTH - size.width - EDGE);
  const y = opensDown ? anchorY - BAR_HEIGHT / 2 : anchorY + BAR_HEIGHT / 2 - size.height;
  return { x, y, width: size.width, height: size.height };
}

/** The panel's box: under the pill (or above it), as large as the room allows. */
export function panelBox({ anchorX, anchorY, opensDown }: Placement): Box {
  const width = Math.min(PANEL_MAX.width, WINDOW_WIDTH - 2 * EDGE);
  const room = opensDown ? WINDOW_HEIGHT - (anchorY + BAR_HEIGHT / 2 + PANEL_GAP) - EDGE : anchorY - BAR_HEIGHT / 2 - PANEL_GAP - EDGE;
  const height = Math.max(0, Math.min(PANEL_MAX.height, room));
  const x = Math.min(Math.max(EDGE, anchorX - width / 2), WINDOW_WIDTH - width - EDGE);
  const y = opensDown ? anchorY + BAR_HEIGHT / 2 + PANEL_GAP : anchorY - BAR_HEIGHT / 2 - PANEL_GAP - height;
  return { x, y, width, height };
}

/** The hub (09, section 8 bis): 720 px wide, 250 high at home, 420 for a session followed
 * live, 140 while Mochi gets his breath back. */
export const HUB_WIDTH = 720;
export const HUB_RADIUS = 24;
const HUB_HEIGHT: Record<HubView, number> = { home: 250, messages: 250, pastilles: 250, live: 420, dizzy: 140, chat: 420, mail: 330 };

export function hubHeight(view: HubView): number {
  return HUB_HEIGHT[view];
}

/** The hub's box: it comes out of the island (`island`, as drawn now), below it or above
 * it, centered on it, never past the window's edges (it gets shorter if room lacks). */
export function hubBox(island: Box, opensDown: boolean, height: number): Box {
  const width = Math.min(HUB_WIDTH, WINDOW_WIDTH - 2 * EDGE);
  const x = Math.min(Math.max(EDGE, island.x + island.width / 2 - width / 2), WINDOW_WIDTH - width - EDGE);
  const room = opensDown ? WINDOW_HEIGHT - (island.y + island.height + PANEL_GAP) - EDGE : island.y - PANEL_GAP - EDGE;
  const fitted = Math.max(0, Math.min(height, room));
  const y = opensDown ? island.y + island.height + PANEL_GAP : island.y - PANEL_GAP - fitted;
  return { x, y, width, height: fitted };
}

export type Shape = { x: number; y: number; width: number; height: number; radius: number };

/** Island outline relative to the window, sent to the Rust core for click-through. */
export function shapeFor(size: Size, placement: Placement): Shape {
  const box = islandBox(size, placement);
  return { ...box, radius: size.radius };
}

export type ModeInputs = {
  permission: boolean;
  drop: boolean;
  menuOpen: boolean;
  /** Mochi's wardrobe (prompt 12); absent counts as closed. */
  wardrobeOpen?: boolean;
  event: Exclude<IslandMode, "pill" | "permission" | "drop" | "menu" | "wardrobe"> | null;
};

/** A permission goes first and stays until answered; then a file being dropped; then
 * the quick menu or the wardrobe; then the event on screen; else the pill. */
export function resolveMode({ permission, drop, menuOpen, wardrobeOpen = false, event }: ModeInputs): IslandMode {
  if (permission) return "permission";
  if (drop) return "drop";
  if (menuOpen) return "menu";
  if (wardrobeOpen) return "wardrobe";
  return event ?? "pill";
}

/** Mochi is drawn 48 px wide over a 36 px slot that each opening keeps for him (6 px
 * left of it, 8 px above); where he sits in each shape, from the island's top-left corner
 * (none: he steps aside while the music opening or the new track shows, and in his
 * wardrobe, which shows him large). */
export const MOCHI_PX = 48;

export type MochiPlace = { x: number; y: number } | null;

export function mochiPlace(mode: IslandMode, size: Size): MochiPlace {
  switch (mode) {
    case "pill":
      return { x: 2, y: -6 };
    case "finished":
    case "failed":
    case "toast":
      return { x: 6, y: (size.height - MOCHI_PX) / 2 - 2 };
    case "permission":
      return { x: 18, y: 10 };
    case "message":
    case "voice":
    case "translation":
      return { x: size.width - 58, y: 4 };
    case "drop":
      return { x: 24, y: (size.height - MOCHI_PX) / 2 - 2 };
    case "file":
      return { x: 14, y: 12 };
    case "menu":
      return { x: 6, y: -6 };
    case "music":
    case "track":
    case "wardrobe":
      return null;
  }
}

/** Distance from a point to a box (0 inside it). */
export function distanceTo(box: Box, x: number, y: number): number {
  const dx = Math.max(0, box.x - x, x - (box.x + box.width));
  const dy = Math.max(0, box.y - y, y - (box.y + box.height));
  return Math.hypot(dx, dy);
}

/** The folded pill swells when the cursor comes this close. */
export const NEAR_PX = 64;

/** Scale of the island: carried while it is moved, pressed, approached when folded. */
export function scaleFor({ dragging, pressed, near }: { dragging: boolean; pressed: boolean; near: boolean }): number {
  if (dragging) return 1.07;
  if (pressed) return 0.97;
  return near ? 1.05 : 1;
}

const clamp = (value: number, min: number, max: number): number => Math.min(max, Math.max(min, value));

/** The 3D relief leans toward the cursor: at most 4° around X and 6° around Y, about
 * half that once opened, and flat again when the cursor is far (the core then stops
 * reporting it). */
export function tiltFor(pointer: { x: number; y: number; near: boolean; inside: boolean }, box: Box, open: boolean): { x: number; y: number } {
  if (!pointer.near && !pointer.inside) return { x: 0, y: 0 };
  const k = open ? 0.45 : 1;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  return {
    x: clamp((-(pointer.y - cy) / WINDOW_HEIGHT) * 9, -4, 4) * k,
    y: clamp(((pointer.x - cx) / WINDOW_WIDTH) * 12, -6, 6) * k,
  };
}

/** Where the specular light sits, in percent of the island (it may slip past the edges). */
export function sheenAt(pointer: { x: number; y: number }, box: Box): { x: number; y: number } {
  return {
    x: clamp(((pointer.x - box.x) / Math.max(1, box.width)) * 100, -20, 120),
    y: clamp(((pointer.y - box.y) / Math.max(1, box.height)) * 100, -60, 160),
  };
}

/** Pulling Mochi out of the pill stretches the island with an elastic resistance: the
 * pull's first pixels count by half, then less and less (never past 140 px). */
export function stretchOf(pull: number): number {
  const reach = 140;
  const give = 0.55;
  return (pull * reach * give) / (reach + give * Math.abs(pull));
}

/** Past this pull, Mochi leaves the island (unless « Reste dans l'île »). */
export const PULL_OUT_PX = 72;

/** The music bubble stuck to the folded pill: on its right, or on its left when the
 * window's edge is too close. `width` and `size` follow their springs. */
export function bubbleBox(island: Box, width: number, size: number, gap: number): Box {
  const right = island.x + island.width + gap + width < WINDOW_WIDTH - EDGE;
  const x = right ? island.x + island.width + gap : island.x - gap - width;
  return { x, y: island.y + island.height / 2 - size / 2, width, height: size };
}
