import { describe, expect, it } from "vitest";
import { type Calm, EMOTIONS, EMOTION_MAX_MS, EMOTION_MIN_MS, canEmote, emotionMs, nextEmotionDelay, pickEmotion } from "./idle";
import type { OwnMove } from "./moves";

/** A small deterministic generator for the simulations. */
function seeded(seed: number): () => number {
  let state = seed;
  return () => {
    state = (Math.imul(state ^ (state >>> 15), 0x2c1b3c6d) + 0x6d2b79f5) >>> 0;
    return state / 2 ** 32;
  };
}

const calm: Calm = { folded: true, working: false, playing: false, busy: false, away: false, animated: true };

describe("small emotions at rest", () => {
  it("smile, yawn, look around and stretch", () => {
    expect(EMOTIONS).toEqual(["smile", "yawn", "glance", "stretch"]);
  });

  it("wait 2 to 5 minutes between two, drawn evenly", () => {
    expect(nextEmotionDelay(() => 0)).toBe(EMOTION_MIN_MS);
    expect(nextEmotionDelay(() => 1)).toBe(EMOTION_MAX_MS);
    expect(nextEmotionDelay(() => 0.5)).toBe(210_000);
    expect(nextEmotionDelay(() => -3)).toBe(EMOTION_MIN_MS);
    const random = seeded(7);
    const delays = Array.from({ length: 2000 }, () => nextEmotionDelay(random));
    expect(Math.min(...delays)).toBeGreaterThanOrEqual(120_000);
    expect(Math.max(...delays)).toBeLessThanOrEqual(300_000);
    const mean = delays.reduce((a, b) => a + b, 0) / delays.length;
    expect(mean).toBeGreaterThan(200_000);
    expect(mean).toBeLessThan(220_000);
  });

  it("over a quiet day, never come closer than 2 minutes nor more than 30 an hour", () => {
    const random = seeded(42);
    let t = 0;
    const starts: number[] = [];
    let previous: OwnMove | undefined;
    while (t < 24 * 3_600_000) {
      t += nextEmotionDelay(random);
      previous = pickEmotion(random, previous);
      starts.push(t);
      t += emotionMs(previous);
    }
    const gaps = starts.slice(1).map((s, i) => s - starts[i]);
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(EMOTION_MIN_MS);
    for (let hour = 0; hour < 24; hour++) {
      const inHour = starts.filter((s) => s >= hour * 3_600_000 && s < (hour + 1) * 3_600_000).length;
      expect(inHour, `hour ${hour}`).toBeLessThanOrEqual(30);
      expect(inHour, `hour ${hour}`).toBeGreaterThanOrEqual(10);
    }
  });

  it("are short and never the same twice in a row", () => {
    for (const emotion of EMOTIONS) {
      expect(emotionMs(emotion)).toBeGreaterThanOrEqual(1500);
      expect(emotionMs(emotion)).toBeLessThanOrEqual(3500);
    }
    const random = seeded(3);
    let previous: OwnMove | undefined;
    const seen = new Set<OwnMove>();
    for (let i = 0; i < 500; i++) {
      const next = pickEmotion(random, previous);
      expect(next).not.toBe(previous);
      seen.add(next);
      previous = next;
    }
    expect(seen.size).toBe(EMOTIONS.length);
    expect(pickEmotion(() => 0.9999999)).toBe("stretch");
  });

  it("only come when nothing goes on", () => {
    expect(canEmote(calm)).toBe(true);
    for (const key of ["working", "playing", "busy", "away"] as const) expect(canEmote({ ...calm, [key]: true }), key).toBe(false);
    expect(canEmote({ ...calm, folded: false })).toBe(false);
    expect(canEmote({ ...calm, animated: false }), "animations off or reduced motion").toBe(false);
  });
});
