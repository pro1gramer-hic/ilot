import { describe, expect, it } from "vitest";
import { record, type Call } from "../../../tests/ui/mochi-recorder";
import { drawFace, EYE_GAP, FACE_IDS, HEAD_IDS, HEAD_LABELS, isHead, type FaceId } from "./faces";
import { INK, SCREEN } from "./palettes";

/** The calls made to draw `face` centered on (12, 10). */
function calls(face: FaceId, screen = false): Call[] {
  const { ctx, calls } = record();
  drawFace(ctx, face, 12, 10, screen);
  return calls;
}

const named = (all: Call[], name: string): Call[] => all.filter(([method]) => method === name);
const sets = (all: Call[], property: string, value: unknown): number =>
  all.filter(([method, set]) => method === `${property} =` && set === value).length;

describe("faces", () => {
  it("has the 9 heads of the design doc, in its order, then 4 reactions", () => {
    expect(HEAD_IDS.map((id) => HEAD_LABELS[id])).toEqual([
      "Content",
      "Clin d'œil",
      "Heureux",
      "Surpris",
      "Inquiet",
      "Endormi",
      "Amoureux",
      "Émerveillé",
      "Malicieux",
    ]);
    expect(FACE_IDS.slice(HEAD_IDS.length)).toEqual(["big", "annoyed", "dizzy", "eat"]);
  });

  it("recognizes only the nine heads", () => {
    expect(HEAD_IDS.every((id) => isHead(id))).toBe(true);
    for (const value of ["big", "angry", "", null, 3]) expect(isHead(value)).toBe(false);
  });
});

describe("drawFace", () => {
  it("draws 13 different faces on the body and 13 on the screen", () => {
    for (const screen of [false, true]) {
      expect(new Set(FACE_IDS.map((face) => JSON.stringify(calls(face, screen)))).size).toBe(13);
    }
  });

  it("restores the context: its glow and line settings do not leak", () => {
    for (const face of FACE_IDS) {
      const all = calls(face, true);
      expect(all[0]).toEqual(["save"]);
      expect(all.at(-1)).toEqual(["restore"]);
    }
  });

  it("puts the eyes 2.6 units on each side of the center", () => {
    const eyes = named(calls("idle"), "ellipse").filter(([, , , w]) => w === 0.92);
    expect(eyes.map(([, x, y]) => [x, y])).toEqual([
      [12 - EYE_GAP, 10],
      [12 + EYE_GAP, 10],
    ]);
  });

  it("gives open eyes a white glint on the body, never on the screen", () => {
    for (const face of ["idle", "wow", "worry", "wink", "big"] as const) {
      expect(sets(calls(face), "fillStyle", INK.glint), face).toBeGreaterThan(0);
      expect(sets(calls(face, true), "fillStyle", INK.glint), face).toBe(0);
    }
    for (const face of ["blink", "happy", "sleep", "annoyed", "dizzy", "eat"] as const) {
      expect(sets(calls(face), "fillStyle", INK.glint), face).toBe(0);
    }
  });

  it("big eyes are the largest ovals, with a second glint each", () => {
    const big = named(calls("big"), "ellipse");
    expect(big.slice(0, 2).map(([, , , w, h]) => [w, h])).toEqual([
      [1.22, 1.7],
      [1.22, 1.7],
    ]);
    expect(named(calls("big"), "arc").filter(([, , , r]) => r === 0.2)).toHaveLength(2);
  });

  it("pink cheeks on the body, except when worried, annoyed or dizzy; none on the screen", () => {
    for (const face of FACE_IDS) {
      const expected = ["worry", "annoyed", "dizzy"].includes(face) ? 0 : 1;
      expect(sets(calls(face), "fillStyle", INK.cheek), face).toBe(expected);
      expect(sets(calls(face, true), "fillStyle", INK.cheek), face).toBe(0);
    }
  });

  it("the screen glows in cyan: halo and every stroke and fill in the glow color", () => {
    const all = calls("happy", true);
    expect(all.slice(1, 3)).toEqual([
      ["shadowColor =", SCREEN.glow],
      ["shadowBlur =", 1.2],
    ]);
    expect(sets(all, "fillStyle", INK.mouth) + sets(all, "strokeStyle", INK.mouth) + sets(all, "fillStyle", INK.tongue)).toBe(0);
    expect(sets(calls("happy"), "shadowBlur", 1.2)).toBe(0);
  });

  it("annoyed: two chevrons pointing inward and a flat mouth", () => {
    const all = calls("annoyed");
    expect(named(all, "moveTo")).toHaveLength(5);
    const tips = named(all, "lineTo").slice(0, 4).map(([, x]) => x);
    expect(tips[0]).toBeGreaterThan(12 - EYE_GAP);
    expect(tips[2]).toBeLessThan(12 + EYE_GAP);
  });

  it("dizzy: a two-turn spiral in each eye, growing from its center", () => {
    const all = calls("dizzy");
    const points = named(all, "lineTo").slice(0, 41);
    expect(points).toHaveLength(41);
    const [, x0, y0] = points[0] as [string, number, number];
    const [, x1, y1] = points[40] as [string, number, number];
    expect(Math.hypot(x0 - (12 - EYE_GAP), y0 - 10)).toBeCloseTo(0.12);
    expect(Math.hypot(x1 - (12 - EYE_GAP), y1 - 10)).toBeCloseTo(1.07);
    expect(named(all, "stroke")).toHaveLength(3);
  });

  it("eat: smiling eyes and a big open mouth with a tongue", () => {
    const ovals = named(calls("eat"), "ellipse").map(([, , , w, h]) => [w, h]);
    expect(ovals).toContainEqual([1.2, 1]);
    expect(ovals).toContainEqual([0.7, 0.4]);
  });

  it("uses the dedicated colors for hearts, stars, sweat and the tongue", () => {
    expect(sets(calls("love"), "fillStyle", INK.heart)).toBe(2);
    expect(sets(calls("star"), "fillStyle", INK.star)).toBe(2);
    expect(sets(calls("worry"), "fillStyle", INK.sweat)).toBe(1);
    expect(sets(calls("happy"), "fillStyle", INK.tongue)).toBe(1);
  });

  it("draws an unknown face like the face at rest", () => {
    expect(calls("unknown" as FaceId)).toEqual(calls("idle"));
  });
});
