// Mochi out of the island (src/Buddy.svelte): where he goes, how he hops, follows and
// flies home, as in `pickSpot`, `hopTo`, `wander` and `buddyHome` of the final demo.

import { describe, expect, it } from "vitest";
import {
  ACT_AT,
  FOLLOW_OFFSET,
  ROAM_MS,
  WEIGHTS,
  along,
  dropAt,
  dropDirection,
  flightAt,
  followStep,
  hops,
  lookOf,
  pickSpot,
  readStart,
  releaseSpeed,
} from "./Buddy.svelte";
import type { BuddySpot } from "./lib/ipc";

const SPOTS: BuddySpot[] = [
  { kind: "bas", x: 640, y: 690 },
  { kind: "fenetre", x: 300, y: 180 },
  { kind: "curseur", x: 520, y: 400 },
  { kind: "bord", x: 1340, y: 380 },
  { kind: "milieu", x: 680, y: 420 },
];

/** Draws that walk through [0, 1) in order. */
function sequence(...values: number[]): () => number {
  let i = 0;
  return () => values[i++ % values.length];
}

describe("start", () => {
  it("reads the point and the mode from the address", () => {
    expect(readStart("?x=812.5&y=40&mode=suit")).toEqual({ start: { x: 812.5, y: 40 }, mode: "suit" });
    expect(readStart("?x=10&y=20&mode=balade").mode).toBe("balade");
  });

  it("falls back to wandering from the origin", () => {
    expect(readStart("?x=abc&mode=reste")).toEqual({ start: { x: 0, y: 0 }, mode: "balade" });
  });

  it("jumps away from the island, toward the middle of the screen", () => {
    expect(dropDirection({ x: 640, y: 28 }, SPOTS)).toBe(1);
    expect(dropDirection({ x: 640, y: 1000 }, SPOTS)).toBe(-1);
    expect(dropDirection({ x: 640, y: 28 }, [])).toBe(1);
    const out = dropAt({ x: 100, y: 50 }, 1, 1);
    expect(out.x).toBe(80);
    expect(out.y).toBeCloseTo(140);
    expect(dropAt({ x: 100, y: 500 }, -1, 1).y).toBeCloseTo(410);
  });
});

describe("places", () => {
  it("every kind of place has its action", () => {
    expect(ACT_AT).toEqual({ bas: "look", fenetre: "dance", curseur: "wave", bord: "peek", milieu: "sleep" });
    const total = Object.values(WEIGHTS).reduce((a, b) => a + b, 0);
    expect(total).toBeCloseTo(1);
  });

  it("draws the kind by its weight", () => {
    expect(pickSpot(SPOTS, sequence(0, 0))?.kind).toBe("bas");
    expect(pickSpot(SPOTS, sequence(0.3, 0))?.kind).toBe("fenetre");
    expect(pickSpot(SPOTS, sequence(0.5, 0))?.kind).toBe("curseur");
    expect(pickSpot(SPOTS, sequence(0.7, 0))?.kind).toBe("bord");
    expect(pickSpot(SPOTS, sequence(0.99, 0))?.kind).toBe("milieu");
  });

  it("only draws among the kinds on offer", () => {
    const bottomOnly = SPOTS.filter((s) => s.kind === "bas");
    expect(pickSpot(bottomOnly, sequence(0.9, 0.9))?.kind).toBe("bas");
    expect(pickSpot([], Math.random)).toBeUndefined();
  });

  it("goes somewhere else than where he stands", () => {
    const bottom = [
      { kind: "bas" as const, x: 100, y: 690 },
      { kind: "bas" as const, x: 900, y: 690 },
    ];
    expect(pickSpot(bottom, sequence(0, 0), { x: 110, y: 690 })).toEqual(bottom[1]);
    expect(pickSpot(bottom.slice(0, 1), sequence(0, 0), { x: 110, y: 690 })).toEqual(bottom[0]);
  });
});

describe("moves", () => {
  it("hops about 130 px at a time, in arcs", () => {
    const trip = hops({ x: 0, y: 0 }, { x: 520, y: 0 });
    expect(trip).toHaveLength(4);
    expect(trip[0]).toMatchObject({ from: { x: 0, y: 0 }, to: { x: 130, y: 0 }, height: 26, ms: 286 });
    expect(trip[3].to).toEqual({ x: 520, y: 0 });
    expect(along(trip[0], 0.5)).toEqual({ x: 65, y: -26 });
    expect(along(trip[0], 1).x).toBe(130);
  });

  it("climbs with higher hops, never too slow or too fast", () => {
    const [climb] = hops({ x: 0, y: 300 }, { x: 0, y: 200 });
    expect(climb.height).toBe(66);
    expect(climb.ms).toBe(260);
    expect(hops({ x: 0, y: 0 }, { x: 0, y: 0 })).toHaveLength(1);
  });

  it("flies home in an arc and lands exactly on his place", () => {
    const from = { x: 900, y: 600 };
    const to = { x: 640, y: 28 };
    expect(flightAt(from, to, 0)).toEqual(from);
    expect(flightAt(from, to, 1).x).toBeCloseTo(640);
    expect(flightAt(from, to, 1).y).toBeCloseTo(28);
    expect(flightAt(from, to, 0.5).y).toBeCloseTo(314 - 110);
  });

  it("keeps the throw of the release", () => {
    const now = 1000;
    const trail = [
      { x: 0, y: 0, t: now },
      { x: 30, y: -10, t: now + 50 },
      { x: 60, y: -20, t: now + 100 },
    ];
    const speed = releaseSpeed(trail);
    expect(speed.x).toBeCloseTo(600);
    expect(speed.y).toBeCloseTo(-200);
    expect(releaseSpeed(trail.slice(0, 1))).toEqual({ x: 0, y: 0 });
  });

  it("follows the cursor on a spring and settles beside it", () => {
    const cursor = { x: 800, y: 400 };
    const target = { x: cursor.x + FOLLOW_OFFSET.x, y: cursor.y + FOLLOW_OFFSET.y };
    let state = { pos: { x: 100, y: 100 }, speed: { x: 0, y: 0 } };
    for (let i = 0; i < 240; i++) state = followStep(state.pos, state.speed, target, 1 / 60);
    expect(state.pos.x).toBeCloseTo(target.x, 1);
    expect(state.pos.y).toBeCloseTo(target.y, 1);
    const thrown = followStep(target, { x: 900, y: 0 }, target, 1 / 60);
    expect(thrown.pos.x).toBeGreaterThan(target.x);
  });
});

describe("looks", () => {
  it("each moment has its move and face", () => {
    expect(lookOf("held", null, 0)).toEqual({ move: "breathe", face: "wow" });
    expect(lookOf("home", null, 0)).toEqual({ move: "fly", face: "wow" });
    expect(lookOf("hop", null, 0)).toEqual({ move: "breathe", face: "happy" });
    expect(lookOf("stop", "dance", 0)).toEqual({ move: "party" });
    expect(lookOf("stop", "sleep", 0)).toEqual({ move: "sleep" });
    expect(lookOf("stop", "peek", 0)).toEqual({ move: "peek", face: "wow" });
    expect(lookOf("follow", null, 300)).toEqual({ move: "bounce", face: "happy" });
    expect(lookOf("follow", null, 20)).toEqual({ move: "breathe" });
  });

  it("an outing lasts about fifteen seconds", () => {
    expect(ROAM_MS).toBe(15_000);
  });
});
