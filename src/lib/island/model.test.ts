import { describe, expect, it } from "vitest";
import { DEFAULT_MODES } from "../modes";
import {
  BAR_HEIGHT,
  DEFAULT_PLACEMENT,
  EDGE,
  MAX_OPEN_HEIGHT,
  PANEL_GAP,
  RISKY_WIDTH,
  WINDOW_HEIGHT,
  WINDOW_WIDTH,
  bubbleBox,
  distanceTo,
  islandBox,
  mochiPlace,
  panelBox,
  pillSize,
  resolveMode,
  scaleFor,
  shapeFor,
  sheenAt,
  sizeFor,
  stretchOf,
  tiltFor,
} from "./model";

const inputs = { modes: DEFAULT_MODES, badge: false, risky: false, measures: { contentHeight: 0 } };
const none = { permission: false, drop: false, menuOpen: false, event: null };

describe("resolveMode", () => {
  it("opens a permission on its own, above everything", () => {
    expect(resolveMode({ permission: true, drop: true, menuOpen: true, event: "message" })).toBe("permission");
  });

  it("shows a file being dropped, then the menu, then the event on screen", () => {
    expect(resolveMode({ ...none, drop: true, menuOpen: true, event: "finished" })).toBe("drop");
    expect(resolveMode({ ...none, menuOpen: true, event: "finished" })).toBe("menu");
    expect(resolveMode({ ...none, event: "failed" })).toBe("failed");
  });

  it("stays a pill otherwise", () => {
    expect(resolveMode(none)).toBe("pill");
  });
});

describe("sizeFor", () => {
  it("sizes the pill by the « Agents dans la pilule » mode, plus room for a badge", () => {
    expect(sizeFor("pill", inputs)).toEqual({ width: 220, height: 40, radius: 20 });
    expect(pillSize("points", false).width).toBe(156);
    expect(pillSize("activite", true).width).toBe(366);
  });

  it("gives every opening the size of the demo", () => {
    expect(sizeFor("finished", inputs)).toEqual({ width: 440, height: 64, radius: 32 });
    expect(sizeFor("failed", inputs)).toEqual({ width: 540, height: 64, radius: 32 });
    expect(sizeFor("music", inputs)).toEqual({ width: 500, height: 96, radius: 30 });
    expect(sizeFor("track", inputs)).toEqual({ width: 420, height: 56, radius: 28 });
    expect(sizeFor("drop", inputs)).toEqual({ width: 340, height: 88, radius: 26 });
    expect(sizeFor("toast", inputs)).toEqual({ width: 440, height: 52, radius: 26 });
  });

  it("fits a permission or a message to its content, wider for a risky command", () => {
    const measured = { ...inputs, measures: { contentHeight: 233.4 } };
    expect(sizeFor("permission", measured)).toEqual({ width: 560, height: 234, radius: 22 });
    expect(sizeFor("permission", { ...measured, risky: true }).width).toBe(RISKY_WIDTH);
    expect(sizeFor("message", measured)).toEqual({ width: 500, height: 234, radius: 24 });
    expect(sizeFor("permission", { ...inputs, measures: { contentHeight: 9000 } }).height).toBe(MAX_OPEN_HEIGHT);
  });
});

describe("islandBox and panelBox", () => {
  const size = { width: 440, height: 64, radius: 32 };

  it("opens down from the anchor in the upper half, up from it in the lower half", () => {
    expect(islandBox(size, DEFAULT_PLACEMENT)).toEqual({ x: 270, y: EDGE, width: 440, height: 64 });
    const low = { anchorX: 490, anchorY: 600, opensDown: false };
    expect(islandBox(size, low).y + 64).toBe(600 + BAR_HEIGHT / 2);
  });

  it("never crosses the window's edges", () => {
    expect(islandBox(size, { ...DEFAULT_PLACEMENT, anchorX: 20 }).x).toBe(EDGE);
    expect(islandBox(size, { ...DEFAULT_PLACEMENT, anchorX: 970 }).x).toBe(WINDOW_WIDTH - 440 - EDGE);
  });

  it("puts the panel under the pill, or above it, within the window", () => {
    const below = panelBox(DEFAULT_PLACEMENT);
    expect(below.y).toBe(EDGE + BAR_HEIGHT + PANEL_GAP);
    expect(below.y + below.height).toBeLessThanOrEqual(WINDOW_HEIGHT - EDGE);
    const above = panelBox({ anchorX: 490, anchorY: WINDOW_HEIGHT - 28, opensDown: false });
    expect(above.y + above.height).toBe(WINDOW_HEIGHT - 28 - BAR_HEIGHT / 2 - PANEL_GAP);
    expect(above.y).toBeGreaterThanOrEqual(EDGE);
  });

  it("sends the click-through shape where the island is drawn", () => {
    expect(shapeFor({ width: 220, height: 40, radius: 20 }, DEFAULT_PLACEMENT)).toEqual({ x: 380, y: EDGE, width: 220, height: 40, radius: 20 });
  });
});

describe("mochiPlace", () => {
  it("keeps Mochi in each opening's slot, and out of the music openings", () => {
    expect(mochiPlace("pill", { width: 220, height: 40, radius: 20 })).toEqual({ x: 2, y: -6 });
    expect(mochiPlace("finished", { width: 440, height: 64, radius: 32 })).toEqual({ x: 6, y: 6 });
    expect(mochiPlace("message", { width: 500, height: 150, radius: 24 })).toEqual({ x: 442, y: 4 });
    expect(mochiPlace("music", { width: 500, height: 96, radius: 30 })).toBeNull();
  });
});

describe("relief and proximity", () => {
  const box = { x: 380, y: 8, width: 220, height: 40 };

  it("measures how far the cursor is from the island", () => {
    expect(distanceTo(box, 400, 20)).toBe(0);
    expect(distanceTo(box, 490, 88)).toBe(40);
    expect(distanceTo(box, 300, 48)).toBe(80);
  });

  it("swells when approached, shrinks when pressed, grows a bit more while carried", () => {
    expect(scaleFor({ dragging: false, pressed: false, near: false })).toBe(1);
    expect(scaleFor({ dragging: false, pressed: false, near: true })).toBe(1.05);
    expect(scaleFor({ dragging: false, pressed: true, near: true })).toBe(0.97);
    expect(scaleFor({ dragging: true, pressed: true, near: true })).toBe(1.07);
  });

  it("leans toward the cursor within 4° and 6°, half as much opened, flat when far", () => {
    const far = { x: 0, y: 700, near: false, inside: false };
    expect(tiltFor(far, box, false)).toEqual({ x: 0, y: 0 });
    const below = tiltFor({ x: 490, y: 200, near: true, inside: false }, box, false);
    expect(below.x).toBeLessThan(0);
    expect(below.y).toBe(0);
    const corner = tiltFor({ x: 9000, y: -9000, near: true, inside: false }, box, false);
    expect(corner).toEqual({ x: 4, y: 6 });
    const opened = tiltFor({ x: 9000, y: -9000, near: true, inside: false }, box, true);
    expect(opened.x).toBeCloseTo(1.8);
    expect(opened.y).toBeCloseTo(2.7);
  });

  it("places the light under the cursor, a little past the edges at most", () => {
    expect(sheenAt({ x: 490, y: 28 }, box)).toEqual({ x: 50, y: 50 });
    expect(sheenAt({ x: -500, y: 900 }, box)).toEqual({ x: -20, y: 160 });
  });
});

describe("stretchOf", () => {
  it("follows the first pixels, then resists more and more", () => {
    expect(stretchOf(0)).toBe(0);
    expect(stretchOf(10)).toBeGreaterThan(5);
    expect(stretchOf(72)).toBeLessThan(72 * 0.55);
    expect(stretchOf(10_000)).toBeLessThan(140);
  });
});

describe("bubbleBox", () => {
  it("sticks to the right of the pill, or to its left near the window's edge", () => {
    const island = { x: 380, y: 8, width: 220, height: 40 };
    expect(bubbleBox(island, 190, 40, 10)).toEqual({ x: 610, y: 8, width: 190, height: 40 });
    const edge = { ...island, x: WINDOW_WIDTH - 230 };
    expect(bubbleBox(edge, 190, 40, 10).x).toBe(WINDOW_WIDTH - 230 - 10 - 190);
  });
});
