import { describe, expect, it } from "vitest";
import { unreadOf } from "./notifications";
import { demoConversations, readPreview } from "./preview";

describe("readPreview", () => {
  it("reads the state, the tab and the theme", () => {
    expect(readPreview("?state=permission&tab=agents&theme=neon&media=crunchyroll&agents=trio")).toEqual({
      state: "permission",
      tab: "agents",
      theme: "neon",
      media: "crunchyroll",
      agents: "trio",
    });
  });

  it("falls back to a busy dark pill on the home tab, with Spotify playing", () => {
    expect(readPreview("")).toEqual({ state: "pill-busy", tab: "today", theme: "fusion", media: "spotify", agents: "demo" });
    expect(readPreview("?state=nope&tab=<script>&theme=pink&media=vlc&agents=all")).toEqual({
      state: "pill-busy",
      tab: "today",
      theme: "fusion",
      media: "spotify",
      agents: "demo",
    });
  });
});

describe("demoConversations", () => {
  it("fills three threads, newest first, with a copied reply and two unread", () => {
    const list = demoConversations(10_000_000);
    expect(list.map((c) => c.appLabel)).toEqual(["WhatsApp", "Discord", "Snapchat"]);
    expect(list.map(unreadOf)).toEqual([1, 1, 0]);
    expect(list[0].messages.map((m) => m.mine)).toEqual([false, true, false]);
    expect(list[0].updatedAt).toBe(10_000_000 - 2 * 60_000);
  });
});
