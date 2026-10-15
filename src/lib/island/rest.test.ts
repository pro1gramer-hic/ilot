import { describe, expect, it } from "vitest";
import { GREETING_MS, INTRO_LINGER_MS, INTRO_MS, LAUNCH_LINGER_MS, introAt, introBeat, introLength } from "../hub";
import { OWN_MOVE_S } from "../mochi/moves";
import { type IslandMode, mochiPlace, resolveMode, sizeFor } from "./model";
import { type Activity, REST_AFTER_MS, isQuiet } from "./rest";
import { DEFAULT_MODES } from "../modes";

const quiet: Activity = {
  mode: "pill",
  hubOpen: false,
  panelOpen: false,
  working: false,
  playing: false,
  waiting: false,
  handled: false,
  roaming: false,
  near: false,
};

describe("« Transparente au repos »", () => {
  it("fades only while nothing at all goes on", () => {
    expect(isQuiet(quiet)).toBe(true);
    for (const key of ["hubOpen", "panelOpen", "working", "playing", "waiting", "handled", "roaming", "near"] as const) {
      expect(isQuiet({ ...quiet, [key]: true }), key).toBe(false);
    }
  });

  it("comes back for every opening, a permission first of all", () => {
    const modes: IslandMode[] = ["finished", "failed", "permission", "message", "music", "track", "drop", "toast", "menu", "wardrobe"];
    for (const mode of modes) expect(isQuiet({ ...quiet, mode }), mode).toBe(false);
  });

  it("waits a few seconds before fading", () => {
    expect(REST_AFTER_MS).toBeGreaterThanOrEqual(3000);
  });
});

describe("the wardrobe opening", () => {
  it("comes after a permission and a drop, with the menu, before the events", () => {
    const base = { permission: false, drop: false, menuOpen: false, event: null };
    expect(resolveMode({ ...base, wardrobeOpen: true })).toBe("wardrobe");
    expect(resolveMode({ ...base, wardrobeOpen: true, event: "finished" })).toBe("wardrobe");
    expect(resolveMode({ ...base, wardrobeOpen: true, permission: true })).toBe("permission");
    expect(resolveMode({ ...base, wardrobeOpen: true, drop: true })).toBe("drop");
    expect(resolveMode({ ...base, wardrobeOpen: true, menuOpen: true })).toBe("menu");
    expect(resolveMode(base)).toBe("pill");
  });

  it("is 540 wide, fits its content, and shows Mochi large instead of in his slot", () => {
    const size = sizeFor("wardrobe", { modes: DEFAULT_MODES, badge: false, risky: false, measures: { contentHeight: 262 } });
    expect(size).toEqual({ width: 540, height: 262, radius: 24 });
    expect(mochiPlace("wardrobe", size)).toBeNull();
  });
});

describe("the launch greeting", () => {
  it("waves after the catch at launch, in place of the sway; « Revoir l'intro » stays as it was", () => {
    expect(GREETING_MS).toBe(OWN_MOVE_S.hello * 1000);
    expect(introLength(false)).toBe(INTRO_MS);
    expect(introLength(true)).toBe(3810 + GREETING_MS);
    for (const t of [0, 950, 2000, 3000, 3809]) expect(introBeat(t, true), `${t}`).toEqual(introAt(t));
    expect(introAt(3900).phase).toBe("sway");
    expect(introBeat(3810, true)).toEqual({ phase: "hello", move: "hello" });
    expect(introBeat(INTRO_MS + 800, true).move).toBe("hello");
    for (const t of [3810, 4000, INTRO_MS]) expect(introBeat(t, false), `${t}`).toEqual(introAt(t));
  });

  it("keeps the launch as long as before: the home lingers less after the greeting", () => {
    expect(introLength(true) + LAUNCH_LINGER_MS).toBe(INTRO_MS + INTRO_LINGER_MS);
    expect(LAUNCH_LINGER_MS).toBeGreaterThanOrEqual(1500);
  });
});
