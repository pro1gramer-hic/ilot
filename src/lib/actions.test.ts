import { afterEach, describe, expect, it, vi } from "vitest";
import { goToAlert, registerActions } from "./actions";
import { DEMO_SHORT_PERMISSION } from "./fixtures";
import { island } from "./island/controller.svelte";
import { ActionRegistry, actions } from "./stores/actions.svelte";
import { banners } from "./stores/banners.svelte";
import { bubble } from "./stores/bubble.svelte";
import { permissions } from "./stores/permissions.svelte";

afterEach(() => {
  permissions.clear();
  banners.queue.clear();
  island.panelOpen = false;
  island.tab = "today";
});

describe("the action registry", () => {
  it("lists an action only once its module gives a real handler", () => {
    const registry = new ActionRegistry();
    expect(registry.available).toEqual([]);
    expect(registry.run("mute")).toBe(false);
    const handler = vi.fn();
    const takeBack = registry.provide("mute", handler);
    expect(registry.available.map((a) => a.id)).toEqual(["mute"]);
    expect(registry.run("mute")).toBe(true);
    expect(handler).toHaveBeenCalledOnce();
    takeBack();
    expect(registry.available).toEqual([]);
    expect(registry.run("mute")).toBe(false);
  });

  it("ignores ids it does not know", () => {
    const registry = new ActionRegistry();
    expect(registry.run("format-c")).toBe(false);
  });

  it("has a real handler for every action of the shortcuts' list", () => {
    registerActions();
    expect(actions.available.map((a) => a.id)).toEqual([
      "toggle-island",
      "go-to-alert",
      "open-chat",
      "go-to-terminal",
      "next-pastille",
      "mute",
      "mochi-to-desktop",
      "open-wardrobe",
      "translate-clipboard",
    ]);
  });
});

describe("« Aller à l'alerte »", () => {
  it("says so when nothing waits", () => {
    expect(goToAlert()).toBe(false);
    expect(bubble.text).toBe("Rien ne t'attend pour l'instant.");
  });

  it("brings the waiting request forward and never answers it", () => {
    permissions.add({ ...DEMO_SHORT_PERMISSION, receivedAt: Date.now() });
    expect(goToAlert()).toBe(true);
    expect(permissions.current?.id).toBe(DEMO_SHORT_PERMISSION.id);
    expect(permissions.answered).toBeNull();
  });

  it("shows the request in the panel's Agents tab when the panel is open", () => {
    island.panelOpen = true;
    permissions.add({ ...DEMO_SHORT_PERMISSION, receivedAt: Date.now() });
    expect(goToAlert()).toBe(true);
    expect(island.tab).toBe("agents");
  });

  it("opens what an opening on screen is about", () => {
    banners.queue.push({ kind: "failed", failed: { source: "codex", project: "site", detail: "npm test" } });
    expect(goToAlert()).toBe(true);
    expect(island.panelOpen).toBe(true);
    expect(island.tab).toBe("agents");
  });
});
