import { describe, expect, it } from "vitest";
import { PEAK, SAMPLE_RATE, type Voice, encodeWav, envelope, gainOf, lengthOf, mixdown, pitch, render } from "./synth";

const tone: Voice = { timbre: "sine", from: 440, at: 0, length: 0.2 };

describe("envelope", () => {
  it("rises fast, decays to a thousandth, and ends at zero", () => {
    expect(envelope(tone, 0)).toBe(0);
    expect(envelope(tone, 0.006)).toBeCloseTo(1, 5);
    expect(envelope(tone, 0.1)).toBeLessThan(0.1);
    expect(envelope(tone, 0.1999)).toBeLessThan(0.002);
    expect(envelope(tone, 0.2)).toBe(0);
    expect(envelope(tone, -0.01)).toBe(0);
  });

  it("keeps a slow attack when asked, within half the voice", () => {
    expect(envelope({ ...tone, attack: 0.05 }, 0.025)).toBeCloseTo(0.5);
    expect(envelope({ ...tone, attack: 5 }, 0.05)).toBeCloseTo(0.5);
  });
});

describe("pitch", () => {
  it("glides exponentially from one pitch to the other", () => {
    const glide = { ...tone, to: 880 };
    expect(pitch(glide, 0)).toBe(440);
    expect(pitch(glide, 0.1)).toBeCloseTo(440 * Math.SQRT2);
    expect(pitch(glide, 0.2)).toBeCloseTo(880);
    expect(pitch(glide, 1)).toBeCloseTo(880);
  });

  it("wobbles around the glide with a vibrato", () => {
    const wobbly = { ...tone, vibrato: { rate: 5, depth: 1 } };
    expect(pitch(wobbly, 0.05)).toBeCloseTo(440 * 2 ** (1 / 12));
    expect(pitch(wobbly, 0.15)).toBeCloseTo(440 * 2 ** (-1 / 12));
  });
});

describe("render", () => {
  it("lasts as long as its longest voice, its loudest sample at the peak", () => {
    const recipe: Voice[] = [tone, { ...tone, from: 660, at: 0.1, length: 0.3 }];
    const samples = render(recipe);
    expect(lengthOf(recipe)).toBeCloseTo(0.4);
    expect(samples.length).toBe(Math.ceil(lengthOf(recipe) * SAMPLE_RATE));
    expect(Math.max(...samples.map(Math.abs))).toBeCloseTo(PEAK, 5);
  });

  it("starts and ends in silence, without a click", () => {
    const samples = render([tone]);
    expect(Math.abs(samples[0])).toBeLessThan(0.01);
    expect(Math.abs(samples[samples.length - 1])).toBeLessThan(0.001);
  });

  it("makes the same sound every time, noise included", () => {
    const recipe: Voice[] = [{ timbre: "noise", from: 1, at: 0, length: 0.05, cutoff: 900 }, { ...tone, timbre: "bell" }];
    expect(render(recipe)).toEqual(render(recipe));
  });

  it("leaves out overtones the rate cannot hold", () => {
    const high = render([{ timbre: "bell", from: 3000, at: 0, length: 0.1 }]);
    expect(high.every(Number.isFinite)).toBe(true);
    expect(Math.max(...high.map(Math.abs))).toBeCloseTo(PEAK, 5);
  });
});

describe("encodeWav", () => {
  it("writes a 16-bit mono PCM WAV the core accepts", () => {
    const wav = encodeWav(new Float32Array([0, 0.5, -0.5, 1, -1, 2]), 22_050, 1);
    const view = new DataView(wav.buffer);
    const text = (from: number): string => String.fromCharCode(...wav.slice(from, from + 4));
    expect([text(0), text(8), text(12), text(36)]).toEqual(["RIFF", "WAVE", "fmt ", "data"]);
    expect(view.getUint32(4, true)).toBe(36 + 12);
    expect(view.getUint16(20, true)).toBe(1);
    expect(view.getUint16(22, true)).toBe(1);
    expect(view.getUint32(24, true)).toBe(22_050);
    expect(view.getUint16(34, true)).toBe(16);
    expect(view.getUint32(40, true)).toBe(12);
    expect([1, 2, 3, 4, 5, 6].map((n) => view.getInt16(42 + n * 2, true))).toEqual([0, 16384, -16383, 32767, -32767, 32767]);
  });

  it("scales by the gain", () => {
    const view = new DataView(encodeWav(new Float32Array([1]), 22_050, 0.25).buffer);
    expect(view.getInt16(44, true)).toBe(Math.round(0.25 * 0x7fff));
    expect(new DataView(encodeWav(new Float32Array([1]), 22_050, 0).buffer).getInt16(44, true)).toBe(0);
  });
});

describe("gainOf", () => {
  it("follows the ear: half the slider is a quarter of the amplitude", () => {
    expect(gainOf(0)).toBe(0);
    expect(gainOf(0.5)).toBe(0.25);
    expect(gainOf(1)).toBe(1);
    expect(gainOf(3)).toBe(1);
    expect(gainOf(Number.NaN)).toBe(0);
  });
});

describe("mixdown", () => {
  it("averages the channels, cuts the length and brings the peak to îlot's", () => {
    const left = new Float32Array([0.2, 0.4, 0.1, 0.3]);
    const right = new Float32Array([0, 0.2, 0.1]);
    const mono = mixdown([left, right], 2);
    expect(mono.length).toBe(2);
    expect(mono[0]).toBeCloseTo((0.1 / 0.3) * PEAK);
    expect(mono[1]).toBeCloseTo(PEAK);
    expect(mixdown([new Float32Array(3)], 10)).toEqual(new Float32Array(3));
  });
});
