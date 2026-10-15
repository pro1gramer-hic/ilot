import { describe, expect, it } from "vitest";
import { DOCK_RADIUS, SAMPLE_MS, THROW_S, keepRecent, nearHome, throwTarget, velocityOf } from "./fling";

const screen = { left: 0, top: 0, right: 1920, bottom: 1040 };
const pill = { width: 348, height: 112 };

describe("fling", () => {
  it("keeps only the last moments of a drag", () => {
    let samples = keepRecent([], { x: 0, y: 0, t: 0 });
    samples = keepRecent(samples, { x: 10, y: 0, t: 50 });
    samples = keepRecent(samples, { x: 30, y: 0, t: 120 });
    expect(samples.map((s) => s.t)).toEqual([50, 120]);
    expect(keepRecent(samples, { x: 0, y: 0, t: 120 + SAMPLE_MS + 1 })).toHaveLength(1);
  });

  it("measures the speed at release in pixels per second", () => {
    expect(velocityOf([{ x: 0, y: 0, t: 0 }, { x: 50, y: -20, t: 100 }])).toEqual({ x: 500, y: -200 });
    expect(velocityOf([{ x: 3, y: 3, t: 10 }])).toEqual({ x: 0, y: 0 });
    expect(velocityOf([{ x: 0, y: 0, t: 10 }, { x: 9, y: 9, t: 10 }])).toEqual({ x: 0, y: 0 });
  });

  it("carries a throw a little further, never off the screen", () => {
    expect(throwTarget({ x: 400, y: 300 }, { x: 1000, y: 0 }, pill, screen)).toEqual({ x: 400 + 1000 * THROW_S, y: 300 });
    expect(throwTarget({ x: 1800, y: 1000 }, { x: 4000, y: 4000 }, pill, screen)).toEqual({ x: 1920 - 348, y: 1040 - 112 });
    expect(throwTarget({ x: 10, y: 10 }, { x: -4000, y: -4000 }, pill, screen)).toEqual({ x: 0, y: 0 });
    expect(throwTarget({ x: 50, y: 50 }, { x: 0, y: 0 }, pill, { left: 0, top: 0, right: 100, bottom: 100 })).toEqual({ x: 0, y: 0 });
  });

  it("goes back into the island only when dropped close to it", () => {
    const home = { x: 960, y: 28 };
    expect(nearHome({ x: 960 + DOCK_RADIUS - 1, y: 28 }, home)).toBe(true);
    expect(nearHome({ x: 960, y: 28 + DOCK_RADIUS }, home)).toBe(false);
  });
});
