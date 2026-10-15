/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";
import soundsRs from "../../../src-tauri/src/sounds.rs?raw";
import { type Conditions, RECIPES, SOUNDS, SOUND_IDS, mayPlay } from "./catalog";
import { SAMPLE_RATE, lengthOf, render } from "./synth";

const calm: Conditions = { muted: false, volume: 0.5, doNotDisturb: false, reducedMotion: false };

describe("the sounds", () => {
  it("are twenty, each with a French name, and known to the core under the same file names", () => {
    expect(SOUND_IDS).toHaveLength(20);
    expect(new Set(SOUND_IDS).size).toBe(20);
    expect(Object.keys(SOUNDS).sort()).toEqual([...SOUND_IDS].sort());
    expect(Object.keys(RECIPES).sort()).toEqual([...SOUND_IDS].sort());
    const core = /SOUND_IDS: \[&str; 20\] = \[([^\]]+)\]/.exec(soundsRs)?.[1] ?? "";
    expect([...core.matchAll(/"(\w+)"/g)].map((m) => m[1])).toEqual([...SOUND_IDS]);
    for (const id of SOUND_IDS) expect(SOUNDS[id].label, id).toMatch(/^[A-ZÉ]/);
  });

  it("are short, never silent, and all different", () => {
    const heard = new Set<string>();
    for (const id of SOUND_IDS) {
      const length = lengthOf(RECIPES[id]);
      expect(length, id).toBeGreaterThan(0.04);
      expect(length, id).toBeLessThan(1);
      const samples = render(RECIPES[id]);
      const energy = samples.reduce((sum, s) => sum + s * s, 0) / samples.length;
      expect(energy, id).toBeGreaterThan(0.005);
      // A coarse print of the sound: its loudness every 20 ms.
      const print = Array.from({ length: Math.ceil(samples.length / (SAMPLE_RATE / 50)) }, (_, i) =>
        Math.round(Math.max(...samples.slice(i * 441, (i + 1) * 441).map(Math.abs)) * 10),
      ).join(",");
      heard.add(`${samples.length}:${print}`);
    }
    expect(heard.size).toBe(20);
  });
});

describe("mayPlay", () => {
  it("plays when nothing holds sounds back", () => {
    expect(mayPlay("finish", calm)).toBe(true);
    expect(mayPlay("click", calm)).toBe(true);
  });

  it("stays silent when sounds are cut, at volume 0, or under Ne pas déranger", () => {
    for (const id of SOUND_IDS) {
      expect(mayPlay(id, { ...calm, muted: true }), id).toBe(false);
      expect(mayPlay(id, { ...calm, volume: 0 }), id).toBe(false);
      expect(mayPlay(id, { ...calm, doNotDisturb: true }), id).toBe(false);
    }
  });

  it("with reduced motion, keeps the sounds of the user's gestures only", () => {
    const reduced = { ...calm, reducedMotion: true };
    for (const id of SOUND_IDS) expect(mayPlay(id, reduced), id).toBe(SOUNDS[id].origin === "user");
    expect(mayPlay("finish", reduced)).toBe(false);
    expect(mayPlay("hello", reduced)).toBe(false);
    expect(mayPlay("click", reduced)).toBe(true);
    expect(mayPlay("allow", reduced)).toBe(true);
  });

  it("Écouter in Réglages plays even while sounds are cut, never at volume 0", () => {
    const cut = { muted: true, volume: 0.5, doNotDisturb: true, reducedMotion: true };
    for (const id of SOUND_IDS) expect(mayPlay(id, cut, true), id).toBe(true);
    expect(mayPlay("finish", { ...cut, volume: 0 }, true)).toBe(false);
  });

  it("every event that comes on its own is marked so", () => {
    const auto = SOUND_IDS.filter((id) => SOUNDS[id].origin === "auto");
    expect(auto).toEqual(["finish", "fail", "approval", "message", "masked", "track", "jump", "hello", "yawn", "back"]);
  });
});
