// Eyes on a sphere and îlot's own moves (prompt 12, lot 1). The reference's parity is
// checked by finale.test.ts with the sphere off, as drawMochiV draws; here, the sphere
// on: identical at a straight gaze, sliding and narrowing toward the edge otherwise.

import { describe, expect, it } from "vitest";
import { type Call, record } from "../../../tests/ui/mochi-recorder";
import { BODY_SEGMENTS, GRID_SIZE, bodyOf, drawMochi, faceAnchor, outlinePoint, type FrameParams } from "./engine";
import { EYE_GAP, FACE_IDS, drawFace, eyeLens } from "./faces";
import { KIND_IDS } from "./kinds";
import { MOVES, OWN_MOVES, OWN_MOVE_S, stillPose } from "./moves";

function frame(params: FrameParams): Call[] {
  const { ctx, calls } = record();
  drawMochi(ctx, 48, params, 0);
  return calls;
}

function face(look: number, name: (typeof FACE_IDS)[number] = "idle", screen = false): Call[] {
  const { ctx, calls } = record();
  drawFace(ctx, name, 12, 10, screen, look);
  return calls;
}

describe("eyes on a sphere", () => {
  it("change nothing at all while Mochi looks straight ahead", () => {
    for (const kind of KIND_IDS) {
      for (const name of FACE_IDS) {
        expect(frame({ kind, face: name, sphere: true }), `${kind}/${name}`).toEqual(frame({ kind, face: name }));
        expect(frame({ kind, face: name, sphere: true, look: 0 }), `${kind}/${name}`).toEqual(frame({ kind, face: name }));
      }
    }
  });

  it("are off unless asked: a gaze aside draws like the reference", () => {
    // The ground shadow is a squashed circle: one scale of the engine's own, then one per eye.
    const scales = (params: FrameParams): number => frame(params).filter(([m]) => m === "scale").length;
    expect(scales({ look: 1 })).toBe(1);
    expect(scales({ look: 1, sphere: true })).toBe(3);
  });

  it("slide each eye further than the face, and narrow the near one most", () => {
    expect(eyeLens(0, -1)).toEqual({ shift: 0, squeeze: 1 });
    const left = eyeLens(1, -1);
    const right = eyeLens(1, 1);
    expect(left.shift).toBeCloseTo(0.45);
    expect(right.shift).toBeCloseTo(0.45);
    expect(right.squeeze).toBeCloseTo(0.72);
    expect(left.squeeze).toBeCloseTo(0.92);
    expect(eyeLens(-1, -1).squeeze).toBeCloseTo(0.72);
    expect(eyeLens(-0.5, 1)).toEqual({ shift: -0.5 * 0.45, squeeze: 1 - 0.5 * 0.08 });
    expect(eyeLens(4, 1)).toEqual(eyeLens(1, 1));
  });

  it("move each eye around its own center, every face, on the body and the visor", () => {
    for (const screen of [false, true]) {
      for (const name of FACE_IDS) {
        const all = face(0.6, name, screen);
        const moves = all.filter(([m]) => m === "translate");
        // Two translations per eye: to its center (plus the slide), and back.
        expect(moves.length % 4, name).toBe(0);
        expect(moves.length, name).toBeGreaterThanOrEqual(4);
        const centers = moves.filter((_, i) => i % 2 === 0).map(([, x]) => (x as number) - 0.6 * 0.45);
        for (const x of centers) expect([12 - EYE_GAP, 12 + EYE_GAP].some((eye) => Math.abs(eye - x) < 1e-9), `${name} ${x}`).toBe(true);
        expect(all.filter(([m]) => m === "save").length).toBe(all.filter(([m]) => m === "restore").length);
      }
    }
  });

  it("the face's anchor is where the engine draws the eyes", () => {
    for (const kind of KIND_IDS) {
      const params: FrameParams = { kind, face: "wow", look: -1, lx: 1, ly: 0.5, shear: 0.1, ry: 6.6 };
      const anchor = faceAnchor(params);
      const eyes = frame(params).filter(([m, , , w]) => m === "ellipse" && w === 1.1);
      expect(eyes.map(([, x, y]) => [x, y]), kind).toEqual([
        [anchor.x - EYE_GAP, anchor.y],
        [anchor.x + EYE_GAP, anchor.y],
      ]);
    }
  });
});

describe("îlot's own moves", () => {
  it("wave hello: Mèche raises his arm, everyone hops, lands and winks", () => {
    const at = (t: number) => MOVES.hello.pose(t);
    expect(at(0.1).wave).toBe(false);
    expect(at(0.6).wave).toBe(true);
    expect(at(0.17).bot).toBeLessThan(21);
    expect(at(0.42).rx).toBeGreaterThan(8.2);
    expect(at(0.1).face).toBe("wow");
    expect(at(0.8).face).toBe("happy");
    expect(at(1.4).face).toBe("wink");
    expect(OWN_MOVE_S.hello).toBe(1.6);
  });

  it("look around with the eyes gliding, from left to right and back", () => {
    const looks = Array.from({ length: 97 }, (_, i) => MOVES.glance.pose(i / 30).look ?? 0);
    expect(Math.min(...looks)).toBeCloseTo(-1);
    expect(Math.max(...looks)).toBeCloseTo(1);
    // Never a jump: at most a fifth of a step from one frame to the next.
    for (let i = 1; i < looks.length; i++) expect(Math.abs(looks[i] - looks[i - 1])).toBeLessThan(0.2);
    expect(looks[0]).toBe(0);
    expect(looks.at(-1)).toBeCloseTo(0);
  });

  it("yawn with the mouth wide open, and stretch up tall", () => {
    expect(MOVES.yawn.pose(1).face).toBe("eat");
    expect(MOVES.yawn.pose(1).ry).toBeGreaterThan(6.6);
    expect(MOVES.stretch.pose(0.8).ry).toBeGreaterThan(7);
    expect(MOVES.stretch.pose(0.8).rx).toBeLessThan(7.6);
    expect(MOVES.smile.pose(0.6).face).toBe("happy");
    expect(MOVES.smile.pose(0.05).face).toBe("blink");
  });

  it("every one moves, starts and ends at rest, and keeps Mochi whole on his grid", () => {
    for (const id of OWN_MOVES) {
      const length = OWN_MOVE_S[id as keyof typeof OWN_MOVE_S];
      const frames = new Set<string>();
      for (let i = 0; i <= length * 30; i++) {
        const pose = MOVES[id].pose(i / 30);
        frames.add(JSON.stringify(pose));
        for (const kind of KIND_IDS) {
          const body = bodyOf({ ...pose, kind });
          const outline = Array.from({ length: BODY_SEGMENTS + 1 }, (_, k) => outlinePoint(body, k));
          const inside = outline.every(([x, y]) => x > 0 && x < GRID_SIZE && y > 0);
          if (!inside) expect.fail(`${id}/${kind} leaves the grid at t=${i / 30}`);
        }
      }
      expect(frames.size, id).toBeGreaterThan(5);
      expect(stillPose(id).face, id).toBe(MOVES[id].face);
    }
  });
});
