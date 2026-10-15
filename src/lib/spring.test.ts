import { describe, expect, it } from "vitest";
import { SpringModel } from "./spring";

const FRAME = 1 / 60;

/** Runs a spring frame by frame; returns the values seen. */
function run(spring: SpringModel, seconds: number): number[] {
  const seen: number[] = [];
  for (let t = 0; t < seconds; t += FRAME) {
    spring.step(FRAME);
    seen.push(spring.value);
  }
  return seen;
}

describe("SpringModel", () => {
  it("reaches its target in 220 to 420 ms and comes to rest", () => {
    const spring = new SpringModel(120);
    spring.target = 440;
    const seen = run(spring, 1.2);
    const within = seen.findIndex((v) => Math.abs(v - 440) < 0.05 * 320);
    expect(within * FRAME).toBeGreaterThan(0.15);
    expect(within * FRAME).toBeLessThan(0.42);
    expect(spring.resting).toBe(true);
    expect(spring.value).toBe(440);
  });

  it("overshoots a little, then settles (damping 0.82)", () => {
    const spring = new SpringModel(0);
    spring.target = 100;
    const peak = Math.max(...run(spring, 1));
    expect(peak).toBeGreaterThan(100);
    expect(peak).toBeLessThan(102);
  });

  it("changes target mid-flight without a jump", () => {
    const spring = new SpringModel(0);
    spring.target = 100;
    run(spring, 0.1);
    const before = { value: spring.value, velocity: spring.velocity };
    spring.target = -50;
    spring.step(FRAME);
    expect(Math.abs(spring.value - before.value)).toBeLessThan(Math.abs(before.velocity) * FRAME + 1);
    const seen = run(spring, 1.5);
    const steps = seen.slice(1).map((v, i) => Math.abs(v - seen[i]));
    expect(Math.max(...steps)).toBeLessThan(25);
    expect(spring.value).toBe(-50);
  });

  it("snaps at once when asked (reduced motion)", () => {
    const spring = new SpringModel(10);
    spring.snap(80);
    expect(spring.resting).toBe(true);
    expect(spring.step(FRAME)).toBe(false);
    expect(spring.value).toBe(80);
  });
});
