/// <reference types="vite/client" />
// The TypeScript engine against `drawMochiV()` of the final demo, call by call: both draw on
// a recording context (tests/ui/mochi-recorder.ts), with the same frame parameters, and must
// make the same canvas calls with the same arguments (îlot only wraps the drawing in a
// save/restore pair, so no setting leaks into the next frame). The reference's pure part
// is evaluated without any DOM, see tests/ui/mochi-reference.ts; the pixels are compared
// in a browser by tests/ui/mochi.e2e.ts.

import { describe, expect, it } from "vitest";
import finaleHtml from "../../../reference/ilot-demo-finale.html?raw";
import vectorEngine from "../../../reference/mochi-vectoriel.js?raw";
import { loadFinaleReference, type RefParams } from "../../../tests/ui/mochi-reference";
import { firstDifference, record } from "../../../tests/ui/mochi-recorder";
import { backingSize, drawMochi, type FrameParams } from "./engine";
import { FACE_IDS, HEAD_IDS, HEAD_LABELS } from "./faces";
import { BODIES, KIND_IDS, KIND_LABELS } from "./kinds";
import { ANIMATIONS, MOVE_IDS, MOVES, poseAt } from "./moves";
import { EFFECTS, INK, PALETTE_IDS, PALETTES, SCREEN } from "./palettes";

/** The reference's clock: the waving arm reads it through `performance.now()`. */
let clockMs = 0;
const ref = loadFinaleReference(finaleHtml, { window: { devicePixelRatio: 1 }, performance: { now: () => clockMs } });

/**
 * Draws the same frame with both engines at `scale` units per CSS pixel and `nowMs` on the
 * clock; returns the first differing call, or "" (`palette` is `pal` in the reference).
 */
function compare(params: FrameParams, scale = 7, nowMs = 0): string {
  clockMs = nowMs;
  const theirs = record();
  const { palette, ...rest } = params;
  ref.drawMochiV(theirs.canvas, palette === undefined ? rest : { ...rest, pal: palette }, scale);
  const size = backingSize(scale, 1);
  if (theirs.canvas.width !== size) return `canvas of ${theirs.canvas.width} px instead of ${size}`;
  const ours = record();
  drawMochi(ours.ctx, size, params, nowMs);
  // The one addition: a save/restore pair around the drawing, after the clearing.
  const [setTransform, clear, save, ...drawing] = ours.calls;
  const restore = drawing.pop();
  if (JSON.stringify([save, restore]) !== '[["save"],["restore"]]') return "the frame is not wrapped in save/restore";
  return firstDifference([setTransform, clear, ...drawing], theirs.calls);
}

/** Time allowed to the tests that replay thousands of frames, beside the rest of the suite. */
const HEAVY_MS = 30_000;

/** Frame instants: 6 s at 30 frames/s (every move loops within 4.8 s) plus off-grid times. */
const TIMES = [
  ...Array.from({ length: 180 }, (_, i) => i / 30),
  0.0137, 0.5, 1.1999, 2.3999, 3.0001, 4.15, 4.2, 7.77, 13.4, 61.3,
];

/** The reference's move with this label; its leftover `sproutDrop` (the old leaf) is dropped. */
function refPose(label: string): (t: number) => RefParams {
  const pose = ref.MV[label];
  if (!pose) throw new Error(`move "${label}" missing from the reference`);
  return (t) => {
    const params = { ...pose(t) };
    delete params.sproutDrop;
    return params;
  };
}

/** A script without blank lines nor indentation differences. */
const lines = (source: string): string[] =>
  source
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

describe("data matches the reference", () => {
  it("the demo draws Mochi with reference/mochi-vectoriel.js, unchanged", () => {
    expect(lines(ref.engineSource)).toEqual(lines(vectorEngine));
  });

  it("palettes, eye, cheek and screen colors", () => {
    expect(PALETTE_IDS).toEqual(Object.keys(ref.PAL));
    for (const id of PALETTE_IDS) {
      expect(PALETTES[id].name).toBe(ref.PAL[id].n);
      expect(PALETTES[id].tiers).toEqual(ref.PAL[id].t);
      expect(PALETTES[id].outline).toBe(ref.PAL[id].o);
    }
    expect([INK.eye, INK.cheek, SCREEN.visor, SCREEN.glow]).toEqual([ref.V_EYE, ref.V_CHEEK, ref.V_SCREEN, ref.V_GLOW]);
  });

  it("silhouettes: same ids, labels and bodies", () => {
    expect(KIND_IDS.map((id) => [id, KIND_LABELS[id]])).toEqual(ref.KINDS);
    expect(KIND_IDS).toEqual(Object.keys(ref.MV_KIND));
    for (const id of KIND_IDS) {
      const { pt, pb, rx, ry } = ref.MV_KIND[id];
      expect(BODIES[id]).toEqual({ top: pt, bottom: pb, rx, ry });
    }
  });

  it("heads: same ids, same French labels, same order", () => {
    expect(HEAD_IDS.map((id) => [id, HEAD_LABELS[id]])).toEqual(ref.HEADS);
  });

  it("animations: the ten of the reference, in its order; « Idle » is said in French", () => {
    expect(ANIMATIONS.map(({ move }) => MOVES[move].label)).toEqual(ref.ANIMS.map(([name]) => name));
    expect(ANIMATIONS.map(({ label }) => label)).toEqual(ref.ANIMS.map(([, label]) => (label === "Idle" ? "Immobile" : label)));
  });

  it("moves: every one is in the reference, with the same frame parameters at every instant", () => {
    for (const id of MOVE_IDS) {
      const reference = refPose(MOVES[id].label);
      for (const t of TIMES) expect(MOVES[id].pose(t), `${id} at t=${t}`).toEqual(reference(t));
    }
  });

  it("each move's still face is one the move itself shows", () => {
    for (const id of MOVE_IDS) {
      const faces = new Set(TIMES.map((t) => MOVES[id].pose(t).face));
      expect(faces.has(MOVES[id].face), id).toBe(true);
    }
  });
});

describe("drawing matches drawMochiV call by call", () => {
  it("every silhouette × palette × face, at rest (234 frames)", () => {
    let frames = 0;
    for (const kind of KIND_IDS) {
      for (const palette of PALETTE_IDS) {
        for (const face of FACE_IDS) {
          expect(compare({ kind, palette, face }), `${kind}/${palette}/${face}`).toBe("");
          frames++;
        }
      }
    }
    expect(frames).toBe(234);
  });

  it("every silhouette × move at 190 instants, palettes in turn (8550 frames)", () => {
    let frames = 0;
    for (const kind of KIND_IDS) {
      for (const id of MOVE_IDS) {
        TIMES.forEach((t, k) => {
          const palette = PALETTE_IDS[k % PALETTE_IDS.length];
          expect(compare({ ...MOVES[id].pose(t), kind, palette }), `${kind}/${id}/${palette} t=${t}`).toBe("");
          frames++;
        });
      }
    }
    expect(frames).toBe(8550);
  }, HEAVY_MS);

  it("every face imposed on every move, 8 instants each (4680 frames)", () => {
    let frames = 0;
    for (const kind of KIND_IDS) {
      MOVE_IDS.forEach((id, i) => {
        const palette = PALETTE_IDS[i % PALETTE_IDS.length];
        for (const face of FACE_IDS) {
          for (const t of TIMES.filter((_, k) => k % 24 === 0)) {
            expect(compare({ ...poseAt(id, t, { face }), kind, palette }), `${kind}/${id}+${face} t=${t}`).toBe("");
            frames++;
          }
        }
      });
    }
    expect(frames).toBe(4680);
  }, HEAVY_MS);

  it("the gaze and the face offsets move the face of every silhouette like the reference", () => {
    for (const kind of KIND_IDS) {
      for (const face of ["idle", "blink", "big", "dizzy"] as const) {
        for (const look of [-1, -0.5, 0, 1]) {
          expect(compare({ kind, face, look, lx: 0.4, ly: -0.3 }), `${kind}/${face} look ${look}`).toBe("");
        }
      }
    }
  });

  it("the waving arm swings with the injected clock, like the reference's performance.now()", () => {
    for (const kind of KIND_IDS) {
      for (const nowMs of [0, 95, 377, 1234.5, 98765]) {
        expect(compare({ kind, wave: true, face: "happy" }, 3, nowMs), `${kind} at ${nowMs} ms`).toBe("");
      }
    }
  });

  it("random frame parameters at random sizes (900 frames)", () => {
    let seed = 0x9e3779b9;
    const random = (): number => {
      seed = (Math.imul(seed ^ (seed >>> 15), 0x2c1b3c6d) + 0x6d2b79f5) >>> 0;
      return seed / 2 ** 32;
    };
    const between = (min: number, max: number): number => min + random() * (max - min);
    const int = (min: number, max: number): number => Math.floor(between(min, max + 1));
    const pick = <T>(items: readonly T[]): T => items[int(0, items.length - 1)];
    for (let i = 0; i < 900; i++) {
      const fx: [number, number, string][] = Array.from({ length: int(0, 4) }, () => [
        between(-2, 25),
        between(-2, 25),
        pick(EFFECTS.confetti),
      ]);
      const params: FrameParams = {
        palette: pick(PALETTE_IDS),
        kind: pick(KIND_IDS),
        cx: between(7, 17),
        bot: random() > 0.5 ? int(14, 23) : between(14, 33),
        rx: between(4, 11),
        ry: between(3, 8),
        shear: between(-0.3, 0.3),
        face: pick(FACE_IDS),
        look: between(-2, 2),
        lx: int(-2, 2),
        ly: int(-2, 2),
        sway: between(-2, 2),
        wave: random() > 0.7,
        shadow: random() > 0.2,
        shadowK: between(0.3, 1.4),
        fx,
      };
      const scale = pick([1, 2, 7 / 3, 3, 4, 7, 8.5]);
      const nowMs = between(0, 1e5);
      expect(compare(params, scale, nowMs), JSON.stringify({ params, scale, nowMs })).toBe("");
    }
  }, HEAVY_MS);
});
