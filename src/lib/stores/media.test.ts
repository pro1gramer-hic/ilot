import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MediaLists, MediaSnapshot, Track } from "../media";

// The store talks to the Rust core through ipc.ts: replaced here by a fake core.
const core = vi.hoisted(() => ({
  handlers: {} as Record<string, (payload: unknown) => void>,
  snapshot: null as unknown,
  calls: [] as unknown[][],
  refuse: null as string | null,
  outcome: "window" as string,
  video: { detached: null, site: null, extension: false } as unknown,
}));

vi.mock("../ipc", () => {
  const record = (name: string) => async (...args: unknown[]) => {
    core.calls.push([name, ...args]);
    if (core.refuse) throw core.refuse;
  };
  return {
    NO_VIDEO: { detached: null, site: null, extension: false },
    detachVideo: async (...args: unknown[]) => {
      core.calls.push(["detach_video", ...args]);
      if (core.refuse) throw core.refuse;
      return core.outcome;
    },
    dockVideo: async (...args: unknown[]) => {
      core.calls.push(["dock_video", ...args]);
    },
    openCrunchyroll: async () => {
      core.calls.push(["open_crunchyroll"]);
    },
    videoState: async () => core.video,
    getMedia: async () => core.snapshot,
    mediaCommand: record("media_command"),
    mediaSeek: record("media_seek"),
    mediaMode: record("media_mode"),
    openMediaSource: record("open_media_source"),
    openMiniPlayer: record("open_mini_player"),
    playYoutube: record("play_youtube"),
    onCoreEvent: (name: string, handler: (payload: unknown) => void) => {
      core.handlers[name] = handler;
      return () => {};
    },
  };
});

const { MediaStore, NOTICE_MS, TIP_MS } = await import("./media.svelte");

const track = (patch: Partial<Track> = {}): Track => ({
  id: "spotify-1",
  title: "Paris la nuit",
  artist: "4Keus",
  source: "Spotify",
  app: "Spotify",
  appId: "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify",
  playing: true,
  position: 105,
  duration: 221,
  updatedAt: Date.now(),
  controls: { playPause: true, next: true, previous: true },
  ...patch,
});

const spotify = track();
const youtube = track({ id: "yt", source: "YouTube", app: "Google Chrome", appId: "Chrome", playing: false });
const lists = (sessions: Track[]): MediaLists => ({ sessions, recent: sessions, error: null });

beforeEach(() => {
  vi.useFakeTimers();
  core.handlers = {};
  core.calls = [];
  core.refuse = null;
  core.outcome = "window";
  core.video = { detached: null, site: null, extension: false };
  core.snapshot = { current: spotify, ...lists([spotify, youtube]) } satisfies MediaSnapshot;
});

afterEach(() => {
  vi.useRealTimers();
});

describe("media store", () => {
  it("loads the state of the core, then follows its events", async () => {
    const media = new MediaStore();
    await media.connect();
    expect(media.current).toEqual(spotify);
    expect(media.sessions).toHaveLength(2);
    expect(media.playing).toBe(true);
    core.handlers["media:changed"](null);
    core.handlers["media:sessions"]({ sessions: [], recent: [spotify], error: null });
    expect(media.current).toBeNull();
    expect(media.playing).toBe(false);
    expect(media.recent).toEqual([spotify]);
  });

  it("shuffle and repeat answer at once, and come back when the app refuses", async () => {
    const media = new MediaStore();
    const modal = track({ controls: { playPause: true, next: true, previous: true, shuffle: true, repeat: true }, shuffle: false, repeat: "none" });
    media.apply({ current: modal, ...lists([modal]) });
    await media.setMode({ shuffle: true });
    expect(core.calls.at(-1)).toEqual(["media_mode", modal.appId, { shuffle: true }]);
    expect(media.shown?.shuffle).toBe(true);
    expect(media.current?.shuffle).toBe(true);
    core.refuse = "L'app n'a pas accepté la commande.";
    await media.setMode({ repeat: "list" });
    expect(media.shown?.repeat).toBe("none");
    expect(media.notice).toBe("L'app n'a pas accepté la commande.");
  });

  it("shows the session of the chosen tab", () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify, youtube]) });
    expect(media.shown).toEqual(spotify);
    expect(media.tab).toBe("Spotify");
    media.select("YouTube");
    expect(media.shown).toEqual(youtube);
    media.select("Crunchyroll");
    expect(media.shown).toBeNull();
    expect(media.tab).toBe("Crunchyroll");
  });

  it("follows what plays when another source starts", () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify, youtube]) });
    media.select("YouTube");
    media.setCurrent({ ...spotify, position: 120 });
    expect(media.filter).toBe("YouTube");
    media.setCurrent({ ...youtube, playing: true });
    expect(media.filter).toBeNull();
    expect(media.tab).toBe("YouTube");
  });

  it("sends commands to the shown session", async () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify, youtube]) });
    await media.command("pause");
    media.select("YouTube");
    await media.command("play");
    await media.open(youtube);
    await media.playVideo("dQw4w9WgXcQ", 30);
    expect(core.calls).toEqual([
      ["media_command", "pause", spotify.appId],
      ["media_command", "play", "Chrome"],
      ["open_media_source", "Chrome"],
      ["play_youtube", "dQw4w9WgXcQ", 30],
    ]);
  });

  it("seeks the shown session at once, then asks the core", async () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify, youtube]) });
    await media.seek(60);
    expect(media.current?.position).toBe(60);
    expect(media.sessions[0].position).toBe(60);
    expect(media.sessions[1]).toEqual(youtube);
    await media.seek(999);
    expect(core.calls).toEqual([
      ["media_seek", spotify.appId, 60],
      ["media_seek", spotify.appId, 221],
    ]);
    media.select("Crunchyroll");
    await media.seek(10);
    expect(core.calls).toHaveLength(2);
  });

  it("detaches music into the mini-player, and a video into a real player the core picks", async () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify]) });
    expect(await media.detach()).toBe(true);
    expect(await media.detach({ ...youtube, appId: "ilot.exe", videoId: "dQw4w9WgXcQ" })).toBe(true);
    expect(core.calls).toEqual([["open_mini_player"], ["detach_video", "ilot.exe", true]]);
    expect(media.tip).toBeNull();
  });

  it("follows where the video is, and « Ramener la vidéo » sends it back", async () => {
    const media = new MediaStore();
    core.video = { detached: "onTop", site: null, extension: true };
    await media.connect();
    expect(media.video).toEqual({ detached: "onTop", site: null, extension: true });
    expect(await media.detach(youtube)).toBe(true);
    expect(core.calls).toEqual([["dock_video", true]]);
    core.handlers["video:state"]({ detached: null, site: null, extension: true });
    expect(media.video.detached).toBeNull();
    core.video = { detached: null, site: null, extension: false };
  });

  it("keeps the panel open with a tip when the browser waits for a key", async () => {
    const media = new MediaStore();
    media.apply({ current: youtube, ...lists([youtube]) });
    core.outcome = "needsGesture";
    expect(await media.detach({ ...youtube, source: "Crunchyroll" })).toBe(false);
    expect(media.tip).toBe("Appuie sur Ctrl+Maj+8 : ton navigateur détachera la vidéo.");
    // The user pressed it: the video floats, the tip goes.
    media.setVideo({ detached: "pictureInPicture", site: null, extension: true });
    expect(media.tip).toBeNull();
    media.setVideo({ detached: null, site: null, extension: true });
    expect(await media.detach({ ...youtube, source: "Crunchyroll" })).toBe(false);
    vi.advanceTimersByTime(TIP_MS);
    expect(media.tip).toBeNull();
    core.outcome = "window";
  });

  it("a refused detach says why and keeps the panel", async () => {
    const media = new MediaStore();
    core.refuse = "Impossible de trouver la fenêtre de cette vidéo.";
    expect(await media.detach(youtube)).toBe(false);
    expect(media.notice).toBe("Impossible de trouver la fenêtre de cette vidéo.");
  });

  it("opens îlot's Crunchyroll window", async () => {
    const media = new MediaStore();
    await media.openCrunchyroll();
    expect(core.calls).toEqual([["open_crunchyroll"]]);
  });

  it("shows a refused command for a few seconds", async () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify]) });
    core.refuse = "L'app n'a pas accepté la commande.";
    await media.command("next");
    expect(media.notice).toBe("L'app n'a pas accepté la commande.");
    vi.advanceTimersByTime(NOTICE_MS);
    expect(media.notice).toBeNull();
  });

  it("keeps Windows' refusal as an error", () => {
    const media = new MediaStore();
    media.apply({ current: null, sessions: [], recent: [], error: "Windows ne donne pas accès aux médias en cours." });
    expect(media.error).toMatch(/Windows/);
    expect(media.shown).toBeNull();
  });

  it("ticks once a second while something plays, and stops at rest", () => {
    const media = new MediaStore();
    media.apply({ current: spotify, ...lists([spotify]) });
    const start = media.now;
    vi.advanceTimersByTime(3_000);
    expect(media.now - start).toBe(3_000);
    const paused = { ...spotify, playing: false };
    media.apply({ current: paused, ...lists([paused]) });
    const rest = media.now;
    vi.advanceTimersByTime(5_000);
    expect(media.now).toBe(rest);
    expect(vi.getTimerCount()).toBe(0);
  });
});
