import { describe, expect, it } from "vitest";
import { record, type Call } from "../../../tests/ui/mochi-recorder";
import {
  BODY_SEGMENTS,
  GRID_SIZE,
  backingSize,
  bodyOf,
  drawMochi,
  lean,
  lookToward,
  outlinePoint,
  type FrameParams,
} from "./engine";
import { BODIES, KIND_IDS } from "./kinds";
import { INK, PALETTES, SCREEN } from "./palettes";

/** The calls made to draw one frame on a 48-pixel canvas. */
function calls(params: FrameParams, now = 0, size = 48): Call[] {
  const { ctx, calls } = record();
  drawMochi(ctx, size, params, now);
  return calls;
}

const named = (all: Call[], name: string): Call[] => all.filter(([method]) => method === name);
const sets = (all: Call[], property: string, value: unknown): number =>
  all.filter(([method, set]) => method === `${property} =` && set === value).length;

describe("bodyOf", () => {
  it("stands each silhouette on the ground line, centered, at its own size", () => {
    for (const kind of KIND_IDS) {
      const body = bodyOf({ kind });
      expect(body.shape).toBe(BODIES[kind]);
      expect([body.cx, body.rx, body.ry]).toEqual([12, BODIES[kind].rx, BODIES[kind].ry]);
      expect(body.bot).toBeCloseTo(20.4);
      expect(body.cy).toBeCloseTo(20.4 - BODIES[kind].ry);
      expect(body.top).toBeCloseTo(20.4 - 2 * BODIES[kind].ry);
    }
    expect(bodyOf({}).shape).toBe(BODIES.meche);
  });

  it("applies the move's radii as ratios of 8.2 × 6.2, so every silhouette deforms alike", () => {
    const squashed = bodyOf({ kind: "oreilles", rx: 8.2 * 1.25, ry: 6.2 * 0.5, bot: 18, cx: 9, shear: 0.1 });
    expect(squashed.rx).toBeCloseTo(BODIES.oreilles.rx * 1.25);
    expect(squashed.ry).toBeCloseTo(BODIES.oreilles.ry * 0.5);
    expect([squashed.cx, squashed.shear]).toEqual([9, 0.1]);
    expect(squashed.bot).toBeCloseTo(17.4);
  });
});

describe("lean", () => {
  it("shifts each row by shear × its height above the bottom", () => {
    const body = bodyOf({ shear: 0.2 });
    expect(lean(body, 12, body.bot)).toBe(12);
    expect(lean(body, 12, body.bot - 5)).toBeCloseTo(13);
    expect(lean(bodyOf({}), 7, 3)).toBe(7);
  });
});

describe("outlinePoint", () => {
  it("draws a closed superellipse: each point verifies |x/rx|^p + |y/ry|^p = 1", () => {
    for (const kind of KIND_IDS) {
      const body = bodyOf({ kind, rx: 7.1, ry: 5.3 });
      for (let i = 0; i <= BODY_SEGMENTS; i++) {
        const [x, y] = outlinePoint(body, i);
        const below = y > body.cy + 1e-12;
        const p = below ? body.shape.bottom : body.shape.top;
        const sum = Math.abs((x - body.cx) / body.rx) ** p + Math.abs((y - body.cy) / body.ry) ** p;
        expect(sum, `${kind} point ${i}`).toBeCloseTo(1, 9);
      }
      const [x0, y0] = outlinePoint(body, 0);
      // sin(2π) is not exactly 0: raised to a power below 1, it lands 1e-8 unit away.
      const [xn, yn] = outlinePoint(body, BODY_SEGMENTS);
      expect(xn).toBeCloseTo(x0, 6);
      expect(yn).toBeCloseTo(y0, 6);
    }
  });

  it("goes clockwise from the right: bottom on the ground, left side, then top", () => {
    const body = bodyOf({});
    const quarter = BODY_SEGMENTS / 4;
    const round = (point: readonly number[]) => point.map((v) => Math.round(v * 1e9) / 1e9);
    const at = (i: number) => round(outlinePoint(body, i));
    expect(at(0)).toEqual(round([body.cx + body.rx, body.cy]));
    expect(at(quarter)).toEqual(round([body.cx, body.bot]));
    expect(at(2 * quarter)).toEqual(round([body.cx - body.rx, body.cy]));
    expect(at(3 * quarter)).toEqual(round([body.cx, body.top]));
  });

  it("the squarer « Écran » fills its corners more than the rounder « Mèche »", () => {
    const corner = (kind: "meche" | "ecran") => {
      const body = bodyOf({ kind });
      const [x] = outlinePoint(body, BODY_SEGMENTS / 8);
      return (x - body.cx) / body.rx;
    };
    expect(corner("ecran")).toBeGreaterThan(corner("meche"));
  });

  it("leans with the body: the bottom stays, the top shifts", () => {
    const body = bodyOf({ shear: 0.1 });
    const [bottomX] = outlinePoint(body, BODY_SEGMENTS / 4);
    const [topX] = outlinePoint(body, (BODY_SEGMENTS * 3) / 4);
    expect(bottomX).toBeCloseTo(body.cx);
    expect(topX).toBeCloseTo(body.cx + 0.1 * 2 * body.ry);
  });
});

describe("backingSize", () => {
  it("multiplies the CSS size by the pixel ratio, up to 2", () => {
    expect(backingSize(2, 1)).toBe(48);
    expect(backingSize(2, 1.5)).toBe(72);
    expect(backingSize(2, 2)).toBe(96);
    expect(backingSize(2, 3)).toBe(96);
    expect(backingSize(7, 1.25)).toBe(210);
    expect(backingSize(56 / 24, 1)).toBe(56);
    expect(backingSize(3, 0)).toBe(72);
  });
});

describe("lookToward", () => {
  it("looks left of the box, right of it, and straight ahead inside it", () => {
    expect(lookToward(10, 50, 100)).toBe(-1);
    expect(lookToward(75, 50, 100)).toBe(0);
    expect(lookToward(50, 50, 100)).toBe(0);
    expect(lookToward(140, 50, 100)).toBe(1);
  });
});

describe("drawMochi", () => {
  it("maps the 24-unit grid onto the whole canvas and clears it first", () => {
    const all = calls({}, 0, 96);
    expect(all[0]).toEqual(["setTransform", 4, 0, 0, 4, 0, 0]);
    expect(all[1]).toEqual(["clearRect", 0, 0, GRID_SIZE, GRID_SIZE]);
  });

  it("paints the body with the palette's four lightest shades and its outline color", () => {
    const all = calls({ palette: "menthe" });
    const stops = all.filter(([method]) => String(method).endsWith(".addColorStop")).map(([, , color]) => color);
    for (const shade of PALETTES.menthe.tiers.slice(0, 4)) expect(stops).toContain(shade);
    expect(stops).toContain("rgba(23,71,60,0.28)");
    expect(sets(all, "strokeStyle", "rgba(23,71,60,0.45)")).toBe(1);
  });

  it("casts a soft ground shadow under the grid's center, unless disabled", () => {
    const shadow = (all: Call[]) => named(all, "createRadialGradient").filter(([, x, y]) => x === 12 && y === 23);
    const [gradient] = shadow(calls({ shadowK: 0.5 }));
    expect(gradient[6]).toBeCloseTo(BODIES.meche.rx * 0.82 * 0.5);
    expect(shadow(calls({ shadow: false }))).toHaveLength(0);
  });

  it("stands on two feet, lifted while in the air", () => {
    const feet = (bot?: number) => named(calls({ bot }), "ellipse").filter(([, , , w, h]) => w === 1.7 && h === 0.85);
    const ground = feet();
    expect(ground.map(([, x]) => x)).toEqual([12 - 2.9, 12 + 2.9]);
    expect(ground[0][2]).toBeCloseTo(20.75);
    expect(feet(18)[0][2]).toBeCloseTo(17.4);
  });

  it("Mèche: two arms and a loop on top; with wave, the right arm swings with the clock", () => {
    const rest = calls({});
    expect(named(rest, "bezierCurveTo")).toHaveLength(2);
    expect(named(rest, "rotate")).toEqual([["rotate", -0.5]]);
    const at = (now: number) => named(calls({ wave: true }, now), "rotate")[0][1] as number;
    expect(at(0)).toBeCloseTo(-0.9);
    expect(at(120 * (Math.PI / 2))).toBeCloseTo(-0.6);
    expect(at(377)).toBe(at(377));
    expect(at(377)).not.toBe(at(500));
    expect(named(calls({ wave: true }, 0, 48), "rotate")).toHaveLength(2);
  });

  it("Oreilles: two ears with a pink inside behind the body, and a tail that sways", () => {
    const all = calls({ kind: "oreilles" });
    expect(named(all, "quadraticCurveTo")).toHaveLength(8);
    expect(sets(all, "fillStyle", INK.earInside)).toBe(2);
    const tip = (sway: number) => named(calls({ kind: "oreilles", sway }), "bezierCurveTo")[0][5] as number;
    expect(tip(1) - tip(0)).toBeCloseTo(0.8);
    expect(named(calls({ kind: "meche" }), "quadraticCurveTo")).toHaveLength(1);
  });

  it("Écran: a dark glossy visor with a bolt on each side, eyes glowing in cyan, no cheeks", () => {
    const all = calls({ kind: "ecran" });
    expect(named(all, "roundRect")).toEqual([["roundRect", expect.any(Number), expect.any(Number), 10.4, 5.4, 2.3]]);
    expect(sets(all, "fillStyle", SCREEN.visor)).toBe(1);
    expect(sets(all, "shadowColor", SCREEN.glow)).toBe(1);
    expect(sets(all, "fillStyle", PALETTES.lavande.tiers[3])).toBeGreaterThanOrEqual(4);
    expect(sets(all, "fillStyle", INK.cheek)).toBe(0);
    expect(sets(calls({ kind: "meche" }), "fillStyle", INK.cheek)).toBe(1);
  });

  it("slides the face 0.8 unit per step of gaze, plus its own offsets", () => {
    const eyeX = (params: FrameParams) => named(calls({ face: "wow", ...params }), "ellipse").find(([, , , w]) => w === 1.1)![1] as number;
    expect(eyeX({ look: 1 }) - eyeX({})).toBeCloseTo(0.8);
    expect(eyeX({ look: -1, lx: 2 }) - eyeX({})).toBeCloseTo(1.2);
  });

  it("draws each effect as a small rounded square of its color, on top", () => {
    const all = calls({ fx: [[3, 4, "#ff6b81"], [30, -5, "#3ddc97"]] });
    expect(all.slice(-9, -1)).toEqual([
      ["fillStyle =", "#ff6b81"],
      ["beginPath"],
      ["roundRect", 3.1, 4.1, 0.8, 0.8, 0.25],
      ["fill"],
      ["fillStyle =", "#3ddc97"],
      ["beginPath"],
      ["roundRect", 30.1, -4.9, 0.8, 0.8, 0.25],
      ["fill"],
    ]);
  });

  it("starts each frame from the canvas defaults: the drawing sits in a save/restore pair", () => {
    for (const kind of KIND_IDS) {
      const all = calls({ kind, wave: true, face: "dizzy" });
      expect(all[2]).toEqual(["save"]);
      expect(all.at(-1)).toEqual(["restore"]);
      expect(named(all, "save")).toHaveLength(named(all, "restore").length);
    }
  });
});
