// What plays on Windows, as the island shows it (docs/01-VISION.md, "Le lecteur").
// The Rust core reads it from the Windows media sessions (src-tauri/src/media.rs).

import { texts } from "./i18n/live.svelte";
import type { DetachOutcome, VideoSite } from "./ipc";

export type MediaSource = "Spotify" | "YouTube" | "Crunchyroll" | "other";

/** Sources that have their own tab, in the order of the tabs. */
export const MEDIA_SOURCES = ["Spotify", "YouTube", "Crunchyroll"] as const;

export type TabSource = (typeof MEDIA_SOURCES)[number];

/** What the session lets others do; `shuffle` and `repeat` only when it says so (SMTC
 * `IsShuffleEnabled`, `IsRepeatEnabled`). */
export type MediaControls = { playPause: boolean; next: boolean; previous: boolean; shuffle?: boolean; repeat?: boolean };

/** Repeat mode of a session (`MediaPlaybackAutoRepeatMode`). */
export type RepeatMode = "none" | "track" | "list";

/** One media session (type `Track` of media.rs). */
export type Track = {
  /** Stable key: the same app playing the same title. */
  id: string;
  title: string;
  artist: string;
  source: MediaSource;
  /** App that plays: "Spotify", "Microsoft Edge"… */
  app: string;
  /** Windows app id of the session, used to send it commands. */
  appId: string;
  playing: boolean;
  /** Seconds, at `updatedAt`. */
  position: number;
  /** Seconds; 0 for a live stream or when unknown. */
  duration: number;
  /** Epoch milliseconds when `position` was true. */
  updatedAt: number;
  controls: MediaControls;
  /** Cover as a `data:` URL. */
  artwork?: string;
  /** YouTube video playing in îlot's own player window. */
  videoId?: string;
  /** Shuffle on or off, when the session says. */
  shuffle?: boolean;
  /** Repeat mode, when the session says. */
  repeat?: RepeatMode;
};

/** Commands of `media_command` (contract shared with the Rust core). */
export type MediaAction = "play" | "pause" | "toggle" | "next" | "previous";

export type MediaLists = { sessions: Track[]; recent: Track[]; error: string | null };

export type MediaSnapshot = MediaLists & { current: Track | null };

/** "3:07", "1:02:45". */
export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** Where playback is at `now`: the core sends a position, the interface moves it on. */
export function positionAt(track: Track, now: number): number {
  const elapsed = track.playing ? Math.max(0, now - track.updatedAt) / 1000 : 0;
  const position = Math.max(0, track.position + elapsed);
  return track.duration > 0 ? Math.min(track.duration, position) : position;
}

/** Played share of the track at `now`, from 0 to 1; 0 for a live stream. */
export function progress(track: Track, now: number): number {
  return track.duration > 0 ? positionAt(track, now) / track.duration : 0;
}

/** "21:52 / 2:00:00", or "12:40 · direct" for a live stream. */
export function timeLabel(track: Track, now: number): string {
  const { elapsed, total } = timeEnds(track, now);
  return track.duration > 0 ? `${elapsed} / ${total}` : texts().media.liveTime(elapsed);
}

/** The two ends of the progress bar: time played, and the length or "direct". */
export function timeEnds(track: Track, now: number): { elapsed: string; total: string } {
  return {
    elapsed: formatTime(positionAt(track, now)),
    total: track.duration > 0 ? formatTime(track.duration) : texts().media.live,
  };
}

/** Lowest bar of the wave, as a share of its height: also the flat line of an empty player. */
export const WAVE_FLOOR = 0.16;

/** Bars at each end of a track that rise from the floor, like a song that fades in and out. */
const WAVE_TAPER = 3;

/** FNV-1a hash of a string: the seed of a track's wave. */
function hash(text: string): number {
  let h = 0x811c9dc5;
  for (const char of text) {
    h ^= char.codePointAt(0) ?? 0;
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** Small seeded generator (mulberry32): the same seed always gives the same numbers. */
function seeded(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Shape of the wave drawn as the progress bar. Windows gives the title, not the sound:
 * each track gets its own calm silhouette from its id (three slow swells and a little
 * grain), the same every time it plays. Heights go from WAVE_FLOOR to 1.
 */
export function waveform(seed: string, count: number): number[] {
  const next = seeded(hash(seed));
  const swells = [1, 2, 3].map((weight) => ({ weight, speed: 0.07 + next() * 0.23, phase: next() * Math.PI * 2 }));
  const norm = swells.reduce((sum, s) => sum + 1 / s.weight, 0);
  return Array.from({ length: count }, (_, i) => {
    const swell = swells.reduce((sum, s) => sum + Math.sin(i * s.speed + s.phase) / s.weight, 0) / norm;
    const taper = Math.min(1, (i + 1) / (WAVE_TAPER + 1), (count - i) / (WAVE_TAPER + 1));
    const value = (0.6 + 0.28 * swell + 0.16 * (next() - 0.5)) * taper;
    return Math.round(Math.min(1, Math.max(WAVE_FLOOR, value)) * 100) / 100;
  });
}

/** Name of the source: the site or app, or the app that plays for other sources. */
export function sourceLabel(track: Track): string {
  return track.source === "other" ? track.app : track.source;
}

/** Second line under the title: the artist, or the app when the session gives none. */
export function subtitle(track: Track): string {
  return track.artist.trim() || track.app;
}

/** Label of the button that brings the source back to the front. */
export function openLabel(track: Track): string {
  return track.videoId ? texts().media.showVideo : texts().media.open(track.app);
}

/** First session of a source, preferring one that plays. */
export function sessionOf(sessions: readonly Track[], source: MediaSource): Track | null {
  const all = sessions.filter((t) => t.source === source);
  return all.find((t) => t.playing) ?? all[0] ?? null;
}

/**
 * The session the player shows: the one of the chosen source tab, or else the one
 * Windows calls current, or else the first one.
 */
export function pickShown(current: Track | null, sessions: readonly Track[], filter: TabSource | null): Track | null {
  if (filter) return sessionOf(sessions, filter);
  return current ?? sessions.find((t) => t.playing) ?? sessions[0] ?? null;
}

/** Source whose tab is highlighted: the chosen one, or the one that plays. */
export function activeTab(shown: Track | null, filter: TabSource | null): TabSource | null {
  if (filter) return filter;
  const source = shown?.source;
  return MEDIA_SOURCES.find((s) => s === source) ?? null;
}

/** History of the run limited to one source tab (all of it when no tab applies). */
export function recentOf(recent: readonly Track[], tab: TabSource | null): Track[] {
  return tab ? recent.filter((t) => t.source === tab) : [...recent];
}

/** What a source tab says about its sessions, for screen readers and the tooltip. */
export function tabStatus(sessions: readonly Track[], source: TabSource): string {
  const session = sessionOf(sessions, source);
  const said = texts().media;
  return said.tab(source, !session ? said.nothing : session.playing ? said.playing : said.paused);
}

/** Empty player, per tab: what is missing and what to do. The Musique tab has the YouTube
 * link field right below (`linkHere`), the island and the mini-player do not. */
export function emptyState(tab: TabSource | null, linkHere = false): { title: string; hint: string } {
  const { empty } = texts().media;
  switch (tab) {
    case "Spotify":
      return { ...empty.spotify };
    case "YouTube":
      return { title: empty.youtube.title, hint: linkHere ? empty.youtube.hintHere : empty.youtube.hintTab };
    case "Crunchyroll":
      return { ...empty.crunchyroll };
    default:
      return { ...empty.any };
  }
}

/** YouTube and Crunchyroll show a picture (the one Windows gives), never a record. */
export function isVideo(track: Track | null): boolean {
  return track !== null && (track.source === "YouTube" || track.source === "Crunchyroll" || track.videoId !== undefined);
}

/** « Détacher la vidéo » for a video (« Ramener la vidéo » once one is out), « Détacher le
 * lecteur » otherwise. */
export function detachLabel(track: Track | null, detached = false): string {
  const said = texts().media;
  if (!isVideo(track)) return said.detachPlayer;
  return detached ? said.bringVideoBack : said.detachVideo;
}

/** What is left for the user to do after « Détacher la vidéo », if anything: the browser
 * floats a video only after a gesture of the user (extension/LISEZMOI.md). */
export function detachTip(outcome: DetachOutcome): string | null {
  return outcome === "needsGesture" ? texts().media.detachTip : null;
}

/** Sessions of îlot's own video windows: Windows names them after îlot or after WebView2
 * (src-tauri/src/video.rs, `is_own_session`). */
export function isOwnApp(appId: string): boolean {
  const id = appId.toLowerCase();
  return id.includes("ilot") || id.includes("msedgewebview2");
}

/** The session played by îlot's own window of a site, for the bar of its small player. */
export function ownTrack(sessions: readonly Track[], site: VideoSite): Track | null {
  const own =
    site === "youtube"
      ? sessions.filter((t) => t.videoId !== undefined)
      : sessions.filter((t) => t.source === "Crunchyroll" && isOwnApp(t.appId));
  return own.find((t) => t.playing) ?? own[0] ?? null;
}

/** The rest of this run's history, without what is shown now: Windows gives no queue. */
export function playedBefore(recent: readonly Track[], shown: Track | null, limit = 4): Track[] {
  return recent.filter((t) => t.id !== shown?.id).slice(0, limit);
}

/** Seconds at `ratio` (0 to 1) of a track; null for a live stream, which cannot seek. */
export function seekAt(track: Track, ratio: number): number | null {
  if (track.duration <= 0) return null;
  return Math.round(Math.min(1, Math.max(0, ratio)) * track.duration);
}

/** The track as it stands right after a seek, before the core confirms it. */
export function seeked(track: Track, seconds: number, now: number): Track {
  const position = Math.max(0, track.duration > 0 ? Math.min(track.duration, seconds) : seconds);
  return { ...track, position, updatedAt: now };
}

/** Repeat goes off, then the whole list, then the track alone, as in Spotify. */
export function nextRepeat(mode: RepeatMode | undefined): RepeatMode {
  return mode === "list" ? "track" : mode === "track" ? "none" : "list";
}

/** What the repeat button says it does now. */
export function repeatLabel(mode: RepeatMode | undefined): string {
  const said = texts().media;
  return mode === "list" ? said.repeatList : mode === "track" ? said.repeatTrack : said.noRepeat;
}

/** The track as it stands right after a shuffle or repeat change, before the core confirms it. */
export function withMode(track: Track, mode: { shuffle?: boolean; repeat?: RepeatMode }): Track {
  return { ...track, ...(mode.shuffle === undefined ? {} : { shuffle: mode.shuffle }), ...(mode.repeat === undefined ? {} : { repeat: mode.repeat }) };
}

/** Quietest peak the equalizer shows, in decibels. */
export const METER_FLOOR_DB = -30;
/** Curve from the floor to 0 dB: music peaks mostly in the top 15 dB, the curve spreads them. */
const METER_CURVE = 1.6;

/**
 * Height of the equalizer (0 to 1) for a peak of the audio output (0 to 1, linear, from
 * Windows' IAudioMeterInformation): in decibels, so that quiet music still moves it, and
 * curved, so that loud music does not pin every bar at the top.
 */
export function meterLevel(peak: number): number {
  if (!Number.isFinite(peak) || peak <= 0) return 0;
  const db = 20 * Math.log10(Math.min(1, peak));
  return Math.min(1, Math.max(0, 1 - db / METER_FLOOR_DB)) ** METER_CURVE;
}

/** A meter rises at once and falls back slowly, like a needle: one step per frame. */
export function smoothLevel(shown: number, target: number): number {
  return shown + (target - shown) * (target > shown ? 0.6 : 0.14);
}

/** Frames of delay of each bar, from the left: the middle one shows the level now, its
 * neighbors a moment ago, so one level ripples across the bars. */
const BAR_DELAYS = [7, 3, 0, 2, 5, 9, 4];
/** Each bar's share of the level, so they never stand in a straight line. */
const BAR_GAINS = [0.62, 0.86, 1, 0.9, 0.74, 0.58, 0.8];

/** Heights (0 to 1) of `count` bars from the smoothed levels of the last frames, newest
 * first. Driven by the peak level only: silent music lays them flat. */
export function barHeights(history: readonly number[], count: number): number[] {
  return Array.from({ length: count }, (_, i) => {
    const delay = BAR_DELAYS[i % BAR_DELAYS.length];
    const level = history[Math.min(delay, history.length - 1)] ?? 0;
    return Math.round(level * BAR_GAINS[i % BAR_GAINS.length] * 1000) / 1000;
  });
}
