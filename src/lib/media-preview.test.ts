import { describe, expect, it } from "vitest";
import {
  PREVIEW_ERROR,
  applyCommand,
  applyMode,
  applySeek,
  previewDetach,
  readMiniPreview,
  readScene,
  sceneSnapshot,
  withPlayModes,
} from "./media-preview";

describe("preview scenes", () => {
  it("plays the source of the scene", () => {
    const scene = sceneSnapshot("youtube", 42);
    expect(scene.current?.source).toBe("YouTube");
    expect(scene.sessions.filter((t) => t.playing).map((t) => t.source)).toEqual(["YouTube"]);
    expect(scene.sessions.every((t) => t.updatedAt === 42)).toBe(true);
    expect(scene.recent.length).toBeGreaterThan(0);
  });

  it("has empty and error scenes", () => {
    expect(sceneSnapshot("none", 0)).toEqual({ current: null, sessions: [], recent: [], error: null });
    expect(sceneSnapshot("error", 0).error).toBe(PREVIEW_ERROR);
  });

  it("reads the URL with safe fallbacks", () => {
    expect(readScene("crunchyroll")).toBe("crunchyroll");
    expect(readScene("<script>")).toBe("spotify");
    expect(readMiniPreview("?media=none&tab=YouTube&theme=neon")).toEqual({ theme: "neon", media: "none", tab: "YouTube" });
    expect(readMiniPreview("?tab=Deezer")).toEqual({ theme: "fusion", media: "spotify", tab: null });
  });
});

describe("fake core", () => {
  const start = sceneSnapshot("spotify", 0);
  const spotify = start.current!;

  it("pauses and plays the target session", () => {
    const paused = applyCommand(start, "pause", spotify.appId, 10_000);
    const session = paused.sessions.find((t) => t.appId === spotify.appId)!;
    expect(session.playing).toBe(false);
    expect(session.position).toBe(spotify.position + 10);
    expect(paused.current?.playing).toBe(false);
    const again = applyCommand(paused, "toggle", undefined, 20_000);
    expect(again.current?.playing).toBe(true);
  });

  it("starts another source when asked to play it", () => {
    const crunchyroll = start.sessions.find((t) => t.source === "Crunchyroll")!;
    const next = applyCommand(start, "play", crunchyroll.appId, 0);
    expect(next.sessions.find((t) => t.source === "Crunchyroll")?.playing).toBe(true);
    expect(next.current?.source).toBe("Crunchyroll");
  });

  it("restarts the track on next and previous", () => {
    expect(applyCommand(start, "next", spotify.appId, 0).current?.position).toBe(0);
  });

  it("detaches a video as the core would", () => {
    const none = { detached: null, site: null, extension: false } as const;
    const youtube = start.sessions.find((t) => t.source === "YouTube")!;
    const crunchyroll = start.sessions.find((t) => t.source === "Crunchyroll")!;
    expect(previewDetach({ ...youtube, appId: "ilot.exe", videoId: "dQw4w9WgXcQ" }, none)).toEqual({
      outcome: "window",
      video: { detached: "window", site: "youtube", extension: false },
    });
    expect(previewDetach(youtube, none)).toEqual({ outcome: "onTop", video: { detached: "onTop", site: null, extension: false } });
    const linked = { ...none, extension: true };
    expect(previewDetach(crunchyroll, linked)).toEqual({ outcome: "needsGesture", video: linked });
    expect(previewDetach(undefined, none)).toEqual({ outcome: "miniPlayer", video: none });
  });

  it("seeks the target session only", () => {
    const moved = applySeek(start, spotify.appId, 600, 5);
    expect(moved.current).toMatchObject({ position: 600, updatedAt: 5 });
    expect(moved.sessions.filter((t) => t.position === 600)).toHaveLength(1);
    expect(applySeek(sceneSnapshot("none", 0), "x", 1, 0).current).toBeNull();
  });

  it("shows Spotify with or without shuffle and repeat, both cases being possible", () => {
    const allowed = withPlayModes(start, true);
    const shown = allowed.sessions.find((t) => t.source === "Spotify")!;
    expect(shown.controls).toMatchObject({ shuffle: true, repeat: true });
    expect(shown).toMatchObject({ shuffle: false, repeat: "none" });
    expect(allowed.sessions.find((t) => t.source === "YouTube")?.controls.shuffle).toBeUndefined();
    const plain = withPlayModes(start, false).sessions.find((t) => t.source === "Spotify")!;
    expect(plain.controls).toMatchObject({ shuffle: false, repeat: false });
    const changed = applyMode(allowed, shown.appId, { repeat: "track" });
    expect(changed.sessions.find((t) => t.appId === shown.appId)?.repeat).toBe("track");
  });
});
