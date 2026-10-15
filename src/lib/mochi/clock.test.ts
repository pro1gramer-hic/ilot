import { describe, expect, it } from "vitest";
import { FPS, playFrames, sampleTime, Ticker, type Clock } from "./clock";

const PERIOD = 1000 / 30;

/** A manual clock: time only moves when the test runs the pending timer. */
function fakeClock(start = 0) {
  let time = start;
  let pending: { id: number; at: number; callback: () => void } | null = null;
  let nextId = 1;
  let timers = 0;
  const clock: Clock = {
    now: () => time,
    setTimeout: (callback, ms) => {
      if (pending) throw new Error("two timers at once");
      timers++;
      pending = { id: nextId++, at: time + ms, callback };
      return pending.id;
    },
    clearTimeout: (id) => {
      if (pending?.id === id) pending = null;
    },
  };
  /** Runs the pending timer `lateMs` after its due time (before it when negative). */
  const fire = (lateMs = 0): void => {
    const timer = pending;
    if (!timer) throw new Error("no timer pending");
    pending = null;
    time = timer.at + lateMs;
    timer.callback();
  };
  return {
    clock,
    fire,
    hasPending: () => pending !== null,
    delay: () => (pending ? pending.at - time : NaN),
    timers: () => timers,
  };
}

describe("sampleTime", () => {
  it("quantizes to 30 frames per second, as the demo: floor(ms / 1000 × 30) / 30", () => {
    expect(FPS).toBe(30);
    expect(sampleTime(0)).toEqual({ frame: 0, t: 0 });
    expect(sampleTime(1000)).toEqual({ frame: 30, t: 1 });
    for (const ms of [1, 33, 34, 999, 4321, 123456.7]) {
      expect(sampleTime(ms).t).toBe(Math.floor((ms / 1000) * 30) / 30);
    }
  });
});

describe("Ticker", () => {
  it("plays at once, then on the boundaries of the 30 frames/s grid", () => {
    const { clock, fire, delay } = fakeClock(10);
    const ticker = new Ticker(clock);
    const seen: number[] = [];
    ticker.add((now) => seen.push(now));
    expect(seen).toEqual([10]);
    expect(delay()).toBeCloseTo(PERIOD - 10);
    fire(3);
    expect(seen[1]).toBeCloseTo(PERIOD + 3);
    expect(delay()).toBeCloseTo(PERIOD - 3);
  });

  it("wakes every player in the same task, with a single timer", () => {
    const { clock, fire, timers } = fakeClock();
    const ticker = new Ticker(clock);
    const a: number[] = [];
    const b: number[] = [];
    ticker.add((now) => a.push(now));
    ticker.add((now) => b.push(now));
    for (let i = 0; i < 5; i++) fire();
    expect(a).toHaveLength(6);
    expect(b).toHaveLength(6);
    expect(a.slice(1)).toEqual(b.slice(1));
    expect(timers()).toBe(6);
  });

  it("stops its timer when the last player leaves, and starts again with the next one", () => {
    const { clock, fire, hasPending } = fakeClock();
    const ticker = new Ticker(clock);
    const stopA = ticker.add(() => {});
    const stopB = ticker.add(() => {});
    stopA();
    expect(hasPending()).toBe(true);
    stopB();
    expect(hasPending()).toBe(false);
    ticker.add(() => {});
    expect(hasPending()).toBe(true);
    fire();
    expect(hasPending()).toBe(true);
  });

  it("lets a player leave or join during a tick without a second timer", () => {
    const { clock, fire, hasPending } = fakeClock();
    const ticker = new Ticker(clock);
    const late: number[] = [];
    let joined = false;
    const stop = ticker.add((now) => {
      if (now === 0 || joined) return;
      joined = true;
      ticker.add((at) => late.push(at));
    });
    fire();
    stop();
    fire();
    expect(late).toHaveLength(2);
    expect(hasPending()).toBe(true);
  });

  it("never wakes up twice for one boundary when a tick fires right on it", () => {
    const { clock, fire, delay } = fakeClock();
    const ticker = new Ticker(clock);
    ticker.add(() => {});
    for (let i = 0; i < 90; i++) {
      fire();
      expect(delay()).toBeGreaterThan(PERIOD / 4);
    }
  });
});

describe("playFrames", () => {
  it("draws at once, then once per frame with the elapsed seconds", () => {
    const { clock, fire } = fakeClock(500);
    const drawn: number[] = [];
    playFrames((t) => drawn.push(t), 500, new Ticker(clock));
    for (let i = 0; i < 4; i++) fire(i % 2 ? 4 : 0);
    expect(drawn.map((t) => Math.round(t * 30))).toEqual([0, 1, 2, 3, 4]);
  });

  it("shows each next frame exactly once, even when ticks fire up to half a frame late", () => {
    for (const origin of [0, 1, 16, 33.3, 500.4, 1234.5]) {
      const { clock, fire } = fakeClock(origin);
      const drawn: number[] = [];
      playFrames((t) => drawn.push(Math.round(t * 30)), origin, new Ticker(clock));
      for (let i = 0; i < 60; i++) fire((i * 7) % 15);
      expect(drawn.slice(1), `origin ${origin}`).toEqual(Array.from({ length: 60 }, (_, i) => drawn[1] + i));
      expect(drawn[0], `origin ${origin}`).toBe(0);
    }
  });

  it("never draws the same frame twice, even when woken early", () => {
    const { clock, fire, delay } = fakeClock();
    const drawn: number[] = [];
    playFrames((t) => drawn.push(t), 0, new Ticker(clock));
    for (let i = 0; i < 200; i++) fire(-Math.min(i % 20, delay() - 0.5));
    expect(new Set(drawn).size).toBe(drawn.length);
    expect(drawn).toEqual([...drawn].sort((a, b) => a - b));
  });

  it("keeps one frame in `every` for slow moves, and still never draws one twice", () => {
    const { clock, fire } = fakeClock(500);
    const drawn: number[] = [];
    playFrames((t) => drawn.push(Math.round(t * 30)), 500, new Ticker(clock), 3);
    for (let i = 0; i < 12; i++) fire(i % 2 ? 4 : 0);
    expect(drawn).toEqual([0, 3, 6, 9, 12]);
  });

  it("stops when asked", () => {
    const { clock, hasPending } = fakeClock();
    const stop = playFrames(() => {}, 0, new Ticker(clock));
    expect(hasPending()).toBe(true);
    stop();
    expect(hasPending()).toBe(false);
  });
});
