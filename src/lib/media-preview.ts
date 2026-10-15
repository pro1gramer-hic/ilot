// Browser preview of the player: fake sessions that answer the buttons the way the Rust
// core would, so the preview and Playwright run the real store and components.

import { DEMO_MEDIA_SESSIONS, DEMO_QUEUE } from "./fixtures";
import { type DetachOutcome, NO_VIDEO, type VideoState, setPreviewCore } from "./ipc";
import { type MediaAction, type MediaSnapshot, type RepeatMode, type TabSource, type Track, seeked, withMode } from "./media";
import type { MediaStore } from "./stores/media.svelte";
import { DEFAULT_THEME, THEMES, type ThemeId } from "./themes";

export const MEDIA_SCENES = ["spotify", "youtube", "crunchyroll", "none", "error"] as const;

export type MediaScene = (typeof MEDIA_SCENES)[number];

const PLAYING: Record<"spotify" | "youtube" | "crunchyroll", TabSource> = {
  spotify: "Spotify",
  youtube: "YouTube",
  crunchyroll: "Crunchyroll",
};

export const PREVIEW_ERROR = "Windows refuse l'accès aux médias en cours. Redémarre îlot pour réessayer.";

export function readScene(value: string | null): MediaScene {
  return MEDIA_SCENES.find((s) => s === value) ?? "spotify";
}

/** Fake media state of a scene, with positions true at `now`. */
export function sceneSnapshot(scene: MediaScene, now: number): MediaSnapshot {
  if (scene === "none") return { current: null, sessions: [], recent: [], error: null };
  if (scene === "error") return { current: null, sessions: [], recent: [], error: PREVIEW_ERROR };
  const playing = PLAYING[scene];
  const sessions = DEMO_MEDIA_SESSIONS.map((t) => ({ ...t, playing: t.source === playing, updatedAt: now }));
  const recent = DEMO_QUEUE.map((t) => ({ ...t, updatedAt: now }));
  return { current: sessions.find((t) => t.playing) ?? null, sessions, recent, error: null };
}

export function applyCommand(snapshot: MediaSnapshot, action: MediaAction, appId: string | undefined, now: number): MediaSnapshot {
  const targetId = appId ?? snapshot.current?.appId;
  const change = (t: Track): Track => {
    if (t.appId !== targetId) return t;
    const playing = action === "play" ? true : action === "pause" ? false : action === "toggle" ? !t.playing : t.playing;
    const position = action === "next" || action === "previous" ? 0 : t.position + (t.playing ? (now - t.updatedAt) / 1000 : 0);
    return { ...t, playing, position, updatedAt: now };
  };
  const sessions = snapshot.sessions.map(change);
  const current = sessions.find((t) => t.appId === targetId && t.playing) ?? (snapshot.current && change(snapshot.current));
  return { ...snapshot, sessions, current };
}

/** The fake core moves a session to `seconds` (media_seek). */
export function applySeek(snapshot: MediaSnapshot, appId: string, seconds: number, now: number): MediaSnapshot {
  const move = (t: Track): Track => (t.appId === appId ? seeked(t, seconds, now) : t);
  return { ...snapshot, sessions: snapshot.sessions.map(move), current: snapshot.current && move(snapshot.current) };
}

/** Spotify as it would be if it let îlot change its shuffle and repeat (`on`), or not:
 * nothing documents which one Spotify does, the preview shows both (?modes=off). */
export function withPlayModes(snapshot: MediaSnapshot, on: boolean): MediaSnapshot {
  const change = (t: Track): Track =>
    t.source !== "Spotify"
      ? t
      : on
        ? { ...t, controls: { ...t.controls, shuffle: true, repeat: true }, shuffle: false, repeat: "none" }
        : { ...t, controls: { ...t.controls, shuffle: false, repeat: false } };
  return { ...snapshot, sessions: snapshot.sessions.map(change), current: snapshot.current && change(snapshot.current) };
}

/** The fake core changes shuffle or repeat (media_mode), when the session allows it. */
export function applyMode(snapshot: MediaSnapshot, appId: string, mode: { shuffle?: boolean; repeat?: RepeatMode }): MediaSnapshot {
  const change = (t: Track): Track => (t.appId === appId ? withMode(t, mode) : t);
  return { ...snapshot, sessions: snapshot.sessions.map(change), current: snapshot.current && change(snapshot.current) };
}

/**
 * « Détacher la vidéo » as the core answers it (src-tauri/src/video.rs, `choose`): îlot's
 * own window for its YouTube video; with the browser extension, Crunchyroll waits for the
 * user's key press; any other video window is kept on top.
 */
export function previewDetach(track: Track | undefined, video: VideoState): { outcome: DetachOutcome; video: VideoState } {
  if (!track) return { outcome: "miniPlayer", video };
  if (track.videoId) return { outcome: "window", video: { ...video, detached: "window", site: "youtube" } };
  if (track.source === "Crunchyroll" && video.extension) return { outcome: "needsGesture", video };
  return { outcome: "onTop", video: { ...video, detached: "onTop", site: null } };
}

/** Plays the core in the preview: media commands change the fake state and push it back;
 * `video` says whether the browser extension is there. */
export function startFakeCore(store: MediaStore, snapshot: MediaSnapshot, video: VideoState = NO_VIDEO): void {
  let state = snapshot;
  let out = { ...video };
  /** Spotify's own volume in the preview. */
  let spotifyLevel = 0.7;
  const tell = (next: VideoState): void => {
    out = next;
    store.setVideo(next);
  };
  store.setVideo(out);
  setPreviewCore((command, args) => {
    if (command === "video_state") return out;
    if (command === "detach_video") {
      const answer = previewDetach(state.sessions.find((t) => t.appId === args.appId), out);
      tell(answer.video);
      return answer.outcome;
    }
    if (command === "dock_video") tell({ ...out, detached: null, site: null });
    if (command === "get_media") return state;
    if (command === "media_command") {
      state = applyCommand(state, args.action as MediaAction, args.appId as string | undefined, Date.now());
      store.apply(state);
    }
    if (command === "media_seek") {
      state = applySeek(state, args.appId as string, args.seconds as number, Date.now());
      store.apply(state);
    }
    if (command === "media_mode") {
      const target = state.sessions.find((t) => t.appId === args.appId);
      const allowed = (args.shuffle === null || target?.controls.shuffle) && (args.repeat === null || target?.controls.repeat);
      if (!allowed) throw new Error("L'app n'a pas accepté la commande.");
      const mode = {
        ...(args.shuffle === null ? {} : { shuffle: args.shuffle as boolean }),
        ...(args.repeat === null ? {} : { repeat: args.repeat as RepeatMode }),
      };
      state = applyMode(state, args.appId as string, mode);
      store.apply(state);
    }
    if (command === "app_volume") return state.sessions.some((t) => t.source === "Spotify") ? spotifyLevel : null;
    if (command === "set_app_volume") {
      spotifyLevel = Math.min(1, Math.max(0, args.level as number));
      return spotifyLevel;
    }
    return undefined;
  });
  store.apply(state);
}

export type MiniPreviewOptions = { theme: ThemeId; media: MediaScene; tab: TabSource | null };

/** Mini-player preview: ?media=youtube&tab=YouTube&theme=neon. */
export function readMiniPreview(search: string): MiniPreviewOptions {
  const params = new URLSearchParams(search);
  const tab = params.get("tab");
  return {
    theme: THEMES.find((t) => t.id === params.get("theme"))?.id ?? DEFAULT_THEME,
    media: readScene(params.get("media")),
    tab: (["Spotify", "YouTube", "Crunchyroll"] as const).find((s) => s === tab) ?? null,
  };
}
