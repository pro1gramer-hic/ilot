import { describe, expect, it } from "vitest";
import { BREAK_AFTER_MS, RESET_AFTER_IDLE_MS, WorkClock } from "./worktime";

const MIN = 60_000;

describe("WorkClock", () => {
  it("suggests a break once after 2 h of continuous work", () => {
    const clock = new WorkClock();
    expect(clock.update(0, true)).toBe(false);
    expect(clock.update(BREAK_AFTER_MS - 1, true)).toBe(false);
    expect(clock.update(BREAK_AFTER_MS, true)).toBe(true);
    expect(clock.update(BREAK_AFTER_MS + MIN, true)).toBe(false);
  });

  it("short pauses do not reset the count", () => {
    const clock = new WorkClock();
    clock.update(0, true);
    clock.update(60 * MIN, true);
    clock.update(70 * MIN, false);
    expect(clock.update(BREAK_AFTER_MS, true)).toBe(true);
  });

  it("a real pause of 15 min starts again", () => {
    const clock = new WorkClock();
    clock.update(0, true);
    clock.update(60 * MIN, true);
    clock.update(60 * MIN + RESET_AFTER_IDLE_MS, false);
    expect(clock.update(BREAK_AFTER_MS, true)).toBe(false);
    expect(clock.update(2 * BREAK_AFTER_MS, true)).toBe(true);
  });
});
