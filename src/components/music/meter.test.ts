import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The meter talks to the Rust core through ipc.ts: replaced here by a fake core.
const core = vi.hoisted(() => ({
  inTauri: true,
  calls: [] as unknown[][],
  handler: null as ((payload: { peak: number }) => void) | null,
}));

vi.mock("../../lib/ipc", () => ({
  get inTauri() {
    return core.inTauri;
  },
  audioMeter: async (on: boolean) => {
    core.calls.push(["audio_meter", on]);
  },
  onCoreEvent: (_name: string, handler: (payload: { peak: number }) => void) => {
    core.handler = handler;
    return () => {
      core.calls.push(["unlisten"]);
      core.handler = null;
    };
  },
}));

const { meter, previewPeak } = await import("./meter");

beforeEach(() => {
  vi.useFakeTimers();
  core.inTauri = true;
  core.calls = [];
  core.handler = null;
});

afterEach(() => {
  vi.useRealTimers();
});

describe("meter", () => {
  it("asks the core once, whatever the number of equalizers, and stops with the last", () => {
    const first = meter.listen();
    const second = meter.listen();
    expect(core.calls).toEqual([["audio_meter", true]]);
    core.handler?.({ peak: 0.42 });
    expect(meter.peak).toBe(0.42);
    first();
    first();
    expect(core.calls).toHaveLength(1);
    second();
    expect(core.calls).toEqual([["audio_meter", true], ["unlisten"], ["audio_meter", false]]);
    expect(meter.peak).toBe(0);
  });

  it("makes a level up in the browser preview, and never calls the core", () => {
    core.inTauri = false;
    const stop = meter.listen();
    vi.advanceTimersByTime(100);
    expect(meter.peak).toBeGreaterThan(0);
    stop();
    expect(meter.peak).toBe(0);
    expect(core.calls).toEqual([]);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("gives the preview a level that beats and stays within the meter", () => {
    const levels = Array.from({ length: 300 }, (_, i) => previewPeak(i * 33));
    expect(Math.min(...levels)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...levels)).toBeLessThanOrEqual(1);
    expect(Math.max(...levels) - Math.min(...levels)).toBeGreaterThan(0.4);
  });
});
