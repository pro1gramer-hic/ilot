import { describe, expect, it } from "vitest";
import {
  type Track,
  METER_FLOOR_DB,
  activeTab,
  barHeights,
  detachLabel,
  detachTip,
  emptyState,
  formatTime,
  isOwnApp,
  isVideo,
  meterLevel,
  nextRepeat,
  openLabel,
  ownTrack,
  playedBefore,
  pickShown,
  positionAt,
  progress,
  recentOf,
  repeatLabel,
  seekAt,
  seeked,
  sessionOf,
  smoothLevel,
  sourceLabel,
  subtitle,
  tabStatus,
  timeEnds,
  timeLabel,
  WAVE_FLOOR,
  waveform,
  withMode,
} from "./media";

const track = (patch: Partial<Track> = {}): Track => ({
  id: "spotify-1",
  title: "Lo-fi beats pour coder",
  artist: "Lofi Girl",
  source: "Spotify",
  app: "Spotify",
  appId: "Spotify.exe",
  playing: true,
  position: 30,
  duration: 120,
  updatedAt: 1_000,
  controls: { playPause: true, next: true, previous: true },
  ...patch,
});

const spotify = track();
const youtube = track({ id: "yt", source: "YouTube", app: "Microsoft Edge", appId: "MSEdge", playing: false });
const crunchyroll = track({ id: "cr", source: "Crunchyroll", app: "Crunchyroll", appId: "CR!App", artist: "" });

describe("formatTime", () => {
  it("formats minutes and hours", () => {
    expect(formatTime(0)).toBe("0:00");
    expect(formatTime(187)).toBe("3:07");
    expect(formatTime(3765)).toBe("1:02:45");
  });

  it("guards against invalid values", () => {
    expect(formatTime(-4)).toBe("0:00");
    expect(formatTime(Number.NaN)).toBe("0:00");
  });
});

describe("positionAt", () => {
  it("moves forward while playing, from the time the core read it", () => {
    expect(positionAt(spotify, 6_000)).toBe(35);
    expect(positionAt(track({ playing: false }), 6_000)).toBe(30);
  });

  it("stays within the track and never goes back", () => {
    expect(positionAt(spotify, 1_000_000)).toBe(120);
    expect(positionAt(spotify, 0)).toBe(30);
    expect(positionAt(track({ duration: 0 }), 11_000)).toBe(40);
  });
});

describe("progress and time", () => {
  it("is the played share of the track, clamped to 0..1", () => {
    expect(progress(spotify, 1_000)).toBe(0.25);
    expect(progress(spotify, 500_000)).toBe(1);
    expect(progress(track({ duration: 0 }), 1_000)).toBe(0);
  });

  it("labels the time, or a live stream", () => {
    expect(timeLabel(spotify, 1_000)).toBe("0:30 / 2:00");
    expect(timeLabel(track({ duration: 0, position: 760 }), 1_000)).toBe("12:40 · direct");
  });

  it("gives both ends of the bar: time played and length, or a live stream", () => {
    expect(timeEnds(spotify, 6_000)).toEqual({ elapsed: "0:35", total: "2:00" });
    expect(timeEnds(track({ position: 1313, duration: 7200, playing: false }), 6_000)).toEqual({
      elapsed: "21:53",
      total: "2:00:00",
    });
    expect(timeEnds(track({ duration: 0, position: 760 }), 1_000)).toEqual({ elapsed: "12:40", total: "direct" });
  });
});

describe("waveform", () => {
  it("draws the same wave for the same track, every time", () => {
    expect(waveform("spotify-lofi", 48)).toEqual(waveform("spotify-lofi", 48));
  });

  it("gives each track its own silhouette", () => {
    expect(waveform("spotify-lofi", 48)).not.toEqual(waveform("youtube-rain", 48));
  });

  it("keeps every bar between the floor and the full height", () => {
    for (const seed of ["a", "spotify-lofi", "Épisode 7 · Le dernier printemps", ""]) {
      const wave = waveform(seed, 64);
      expect(wave).toHaveLength(64);
      for (const bar of wave) {
        expect(bar).toBeGreaterThanOrEqual(WAVE_FLOOR);
        expect(bar).toBeLessThanOrEqual(1);
      }
    }
  });

  it("moves: a calm wave, never a flat line", () => {
    const wave = waveform("spotify-lofi", 64);
    const middle = wave.slice(8, 56);
    expect(Math.max(...middle) - Math.min(...middle)).toBeGreaterThan(0.2);
    // Soft: next to each other, two bars never jump from the floor to the top.
    for (let i = 1; i < wave.length; i++) expect(Math.abs(wave[i] - wave[i - 1])).toBeLessThan(0.5);
  });

  it("rises from the floor at the start and falls back at the end", () => {
    const wave = waveform("spotify-lofi", 64);
    expect(wave[0]).toBeLessThan(0.3);
    expect(wave[63]).toBeLessThan(0.3);
  });

  it("draws nothing for no bars", () => {
    expect(waveform("spotify-lofi", 0)).toEqual([]);
  });
});

describe("labels", () => {
  it("names the source, or the app for other sources", () => {
    expect(sourceLabel(spotify)).toBe("Spotify");
    expect(sourceLabel(track({ source: "other", app: "VLC" }))).toBe("VLC");
  });

  it("falls back to the app when the session has no artist", () => {
    expect(subtitle(spotify)).toBe("Lofi Girl");
    expect(subtitle(crunchyroll)).toBe("Crunchyroll");
  });

  it("opens the app, or shows îlot's own video window", () => {
    expect(openLabel(youtube)).toBe("Ouvrir Microsoft Edge");
    expect(openLabel(track({ videoId: "dQw4w9WgXcQ" }))).toBe("Afficher la vidéo");
  });

  it("tells what each source tab holds", () => {
    const sessions = [spotify, youtube];
    expect(tabStatus(sessions, "Spotify")).toBe("Spotify : en lecture");
    expect(tabStatus(sessions, "YouTube")).toBe("YouTube : en pause");
    expect(tabStatus(sessions, "Crunchyroll")).toBe("Crunchyroll : rien en cours");
  });

  it("gives every empty tab a way forward", () => {
    for (const tab of [null, "Spotify", "YouTube", "Crunchyroll"] as const) {
      const { title, hint } = emptyState(tab);
      expect(title.length).toBeGreaterThan(0);
      expect(hint).toMatch(/^Lance /);
    }
  });

  it("points to the link field, below or in the Musique tab", () => {
    expect(emptyState("YouTube", true).hint).toMatch(/colle un lien ci-dessous\.$/);
    expect(emptyState("YouTube").hint).toMatch(/onglet Musique/);
    expect(emptyState("Spotify", true)).toEqual(emptyState("Spotify"));
  });
});

describe("which session the player shows", () => {
  const sessions = [youtube, spotify, crunchyroll];

  it("follows Windows' current session when no tab is chosen", () => {
    expect(pickShown(crunchyroll, sessions, null)).toBe(crunchyroll);
    expect(pickShown(null, sessions, null)).toBe(spotify);
    expect(pickShown(null, [youtube], null)).toBe(youtube);
    expect(pickShown(null, [], null)).toBeNull();
  });

  it("shows the session of the chosen tab, or nothing", () => {
    expect(pickShown(spotify, sessions, "YouTube")).toBe(youtube);
    expect(pickShown(spotify, [spotify], "Crunchyroll")).toBeNull();
  });

  it("prefers the session that plays within a source", () => {
    const paused = track({ id: "paused", playing: false });
    expect(sessionOf([paused, spotify], "Spotify")).toBe(spotify);
    expect(sessionOf([paused], "Spotify")).toBe(paused);
  });

  it("highlights the chosen tab, or the tab of what plays", () => {
    expect(activeTab(spotify, null)).toBe("Spotify");
    expect(activeTab(spotify, "YouTube")).toBe("YouTube");
    expect(activeTab(track({ source: "other" }), null)).toBeNull();
    expect(activeTab(null, null)).toBeNull();
  });

  it("keeps the history of one source, or all of it", () => {
    expect(recentOf([spotify, youtube, crunchyroll], "YouTube")).toEqual([youtube]);
    expect(recentOf([spotify, youtube], null)).toEqual([spotify, youtube]);
  });
});

describe("videos", () => {
  it("YouTube and Crunchyroll show a picture, îlot's own video too", () => {
    expect(isVideo(youtube)).toBe(true);
    expect(isVideo(crunchyroll)).toBe(true);
    expect(isVideo(spotify)).toBe(false);
    expect(isVideo(track({ source: "other", videoId: "dQw4w9WgXcQ" }))).toBe(true);
    expect(isVideo(null)).toBe(false);
  });

  it("names the detach button after what detaches", () => {
    expect(detachLabel(youtube)).toBe("Détacher la vidéo");
    expect(detachLabel(youtube, true)).toBe("Ramener la vidéo");
    expect(detachLabel(spotify)).toBe("Détacher le lecteur");
    expect(detachLabel(spotify, true)).toBe("Détacher le lecteur");
    expect(detachLabel(null)).toBe("Détacher le lecteur");
  });

  it("tells which key to press only when the browser waits for one", () => {
    expect(detachTip("needsGesture")).toBe("Appuie sur Ctrl+Maj+8 : ton navigateur détachera la vidéo.");
    for (const outcome of ["window", "pictureInPicture", "onTop", "miniPlayer"] as const) expect(detachTip(outcome)).toBeNull();
  });
});

describe("îlot's own video windows", () => {
  const own = track({ id: "own-yt", source: "YouTube", app: "îlot", appId: "ilot.exe", videoId: "dQw4w9WgXcQ" });
  const ownCrunchyroll = track({ id: "own-cr", source: "Crunchyroll", app: "Microsoft Edge", appId: "msedgewebview2.exe" });

  it("recognizes their sessions, never a browser's", () => {
    expect(isOwnApp("ilot.exe")).toBe(true);
    expect(isOwnApp("C:\\Program Files\\WebView2\\msedgewebview2.exe")).toBe(true);
    expect(isOwnApp("MSEdge")).toBe(false);
    expect(isOwnApp("15EF7777.Crunchyroll_mgdgtskya6f22!App")).toBe(false);
  });

  it("gives each bar the session of its own window", () => {
    const sessions = [spotify, youtube, crunchyroll, own, ownCrunchyroll];
    expect(ownTrack(sessions, "youtube")).toEqual(own);
    expect(ownTrack(sessions, "crunchyroll")).toEqual(ownCrunchyroll);
    expect(ownTrack([spotify, youtube, crunchyroll], "youtube")).toBeNull();
    expect(ownTrack([spotify, youtube, crunchyroll], "crunchyroll")).toBeNull();
  });
});

describe("played before", () => {
  it("lists the history without what plays now, a few at most", () => {
    const history = [spotify, youtube, crunchyroll, track({ id: "a" }), track({ id: "b" }), track({ id: "c" })];
    expect(playedBefore(history, spotify).map((t) => t.id)).toEqual(["yt", "cr", "a", "b"]);
    expect(playedBefore(history, null, 2)).toEqual([spotify, youtube]);
    expect(playedBefore([], spotify)).toEqual([]);
  });
});

describe("seeking", () => {
  it("turns a share of the wave into seconds, never past the ends", () => {
    expect(seekAt(spotify, 0.5)).toBe(60);
    expect(seekAt(spotify, -1)).toBe(0);
    expect(seekAt(spotify, 2)).toBe(120);
  });

  it("cannot seek a live stream", () => {
    expect(seekAt(track({ duration: 0 }), 0.5)).toBeNull();
  });

  it("moves the position at once, true from now on", () => {
    const moved = seeked(spotify, 90, 5_000);
    expect(moved.position).toBe(90);
    expect(moved.updatedAt).toBe(5_000);
    expect(positionAt(moved, 6_000)).toBe(91);
    expect(seeked(spotify, 500, 0).position).toBe(120);
    expect(seeked(spotify, -3, 0).position).toBe(0);
  });
});

describe("equalizer", () => {
  it("reads the peak in decibels: silence is flat, full scale is full height", () => {
    expect(meterLevel(0)).toBe(0);
    expect(meterLevel(1)).toBe(1);
    expect(meterLevel(10 ** (METER_FLOOR_DB / 2 / 20))).toBeCloseTo(0.5 ** 1.6, 5);
    expect(meterLevel(0.5)).toBeLessThan(meterLevel(0.9));
    expect(meterLevel(10 ** ((METER_FLOOR_DB - 6) / 20))).toBe(0);
    expect(meterLevel(Number.NaN)).toBe(0);
    expect(meterLevel(3)).toBe(1);
  });

  it("rises at once and falls back slowly", () => {
    const up = smoothLevel(0.2, 1);
    const down = smoothLevel(1, 0.2);
    expect(up - 0.2).toBeGreaterThan(1 - down);
    expect(smoothLevel(0.5, 0.5)).toBe(0.5);
  });

  it("ripples one level across the bars, flat in silence", () => {
    expect(barHeights([0, 0, 0], 5)).toEqual([0, 0, 0, 0, 0]);
    const history = [1, ...Array<number>(12).fill(0)];
    const bars = barHeights(history, 5);
    expect(bars[2]).toBe(1);
    expect(bars.filter((h) => h > 0)).toHaveLength(1);
    const steady = barHeights(Array<number>(12).fill(0.5), 5);
    expect(new Set(steady).size).toBe(5);
    expect(Math.max(...steady)).toBe(0.5);
  });

  it("repeat goes off, list, title, as in Spotify", () => {
    expect(nextRepeat(undefined)).toBe("list");
    expect(nextRepeat("none")).toBe("list");
    expect(nextRepeat("list")).toBe("track");
    expect(nextRepeat("track")).toBe("none");
    expect(repeatLabel("track")).toBe("Répéter le titre");
    expect(repeatLabel(undefined)).toBe("Pas de répétition");
    const base = { shuffle: false } as Track;
    expect(withMode(base, { repeat: "list" })).toEqual({ shuffle: false, repeat: "list" });
    expect(withMode(base, { shuffle: true })).toEqual({ shuffle: true });
  });

  it("copes with a short history", () => {
    expect(barHeights([0.4], 3)).toEqual([0.248, 0.344, 0.4]);
    expect(barHeights([], 2)).toEqual([0, 0]);
  });
});
