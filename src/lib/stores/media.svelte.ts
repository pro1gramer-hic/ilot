// What plays on Windows: the session Windows calls current, every session, and the
// history of this run (memory only). The Rust core pushes changes (media:changed,
// media:sessions); between two events, the position moves forward here, once a second.

import {
  type MediaAction,
  type MediaLists,
  type MediaSnapshot,
  type RepeatMode,
  type TabSource,
  type Track,
  activeTab,
  detachTip,
  isVideo,
  pickShown,
  seeked,
  withMode,
} from "../media";
import {
  NO_VIDEO,
  type VideoState,
  detachVideo,
  dockVideo,
  getMedia,
  mediaCommand,
  mediaMode,
  mediaSeek,
  onCoreEvent,
  openCrunchyroll,
  openMediaSource,
  openMiniPlayer,
  playYoutube,
  videoState,
} from "../ipc";
import { reduced } from "../motion.svelte";

/** How long a refused command stays on screen. */
export const NOTICE_MS = 4000;
/** How long a tip (a key to press) stays on screen. */
export const TIP_MS = 8000;

export class MediaStore {
  current = $state<Track | null>(null);
  sessions = $state<Track[]>([]);
  recent = $state<Track[]>([]);
  /** Source tab chosen by the user; null follows what Windows plays. */
  filter = $state<TabSource | null>(null);
  /** Windows refuses access to the media sessions. */
  error = $state<string | null>(null);
  /** Last command the app refused, shown for a few seconds. */
  notice = $state<string | null>(null);
  /** What the user still has to do after a detach (press the browser's shortcut). */
  tip = $state<string | null>(null);
  /** Where the video is: out of the island or not, and whether the browser extension is there. */
  video = $state<VideoState>({ ...NO_VIDEO });
  /** Clock of the interface: ticks every second while something plays. */
  now = $state(Date.now());
  /** Called as the player is detached (startup.ts plays its sound there). */
  ondetach: () => void = () => {};

  playing = $derived(this.current?.playing ?? false);
  shown = $derived(pickShown(this.current, this.sessions, this.filter));
  tab = $derived(activeTab(this.shown, this.filter));

  #clock: ReturnType<typeof setInterval> | undefined;
  #noticeTimer: ReturnType<typeof setTimeout> | undefined;
  #tipTimer: ReturnType<typeof setTimeout> | undefined;

  /** Starts following the core: first the full state, then its events. */
  async connect(): Promise<void> {
    onCoreEvent("media:changed", (track) => this.setCurrent(track));
    onCoreEvent("media:sessions", (lists) => this.setLists(lists));
    onCoreEvent("video:state", (state) => this.setVideo(state));
    this.apply(await getMedia());
    // The mini-player may not read it (capabilities/mini.json): it keeps « nothing out ».
    this.setVideo(await videoState().catch(() => ({ ...NO_VIDEO })));
  }

  setVideo(state: VideoState): void {
    this.video = state;
    // The video came off (the shortcut was pressed): the tip has done its job.
    if (state.detached) this.#hint(null);
  }

  apply(snapshot: MediaSnapshot): void {
    this.setLists(snapshot);
    this.setCurrent(snapshot.current);
  }

  setCurrent(track: Track | null): void {
    // Something else started playing: the player follows it.
    if (track?.source !== this.current?.source) this.filter = null;
    this.current = track;
    this.#syncClock();
  }

  setLists({ sessions, recent, error }: MediaLists): void {
    this.sessions = sessions;
    this.recent = recent;
    this.error = error;
    this.#syncClock();
  }

  /** Shows the session of a source tab (or its empty state). */
  select(source: TabSource): void {
    this.filter = source;
  }

  /** Play, pause, next, previous on the shown session. */
  async command(action: MediaAction, track: Track | null = this.shown): Promise<void> {
    await this.#attempt(() => mediaCommand(action, track?.appId));
  }

  /** Moves the shown session to `seconds`: the wave answers at once, the core follows. */
  async seek(seconds: number, track: Track | null = this.shown): Promise<void> {
    if (!track) return;
    const now = Date.now();
    const move = (t: Track): Track => (t.appId === track.appId ? seeked(t, seconds, now) : t);
    this.sessions = this.sessions.map(move);
    if (this.current) this.current = move(this.current);
    this.now = now;
    await this.#attempt(() => mediaSeek(track.appId, seeked(track, seconds, now).position));
  }

  /** Shuffle or repeat of a session: the button answers at once, the core follows; a
   * refusal puts the old state back. */
  async setMode(mode: { shuffle?: boolean; repeat?: RepeatMode }, track: Track | null = this.shown): Promise<void> {
    if (!track) return;
    const apply = (next: (t: Track) => Track): void => {
      this.sessions = this.sessions.map((t) => (t.appId === track.appId ? next(t) : t));
      if (this.current?.appId === track.appId) this.current = next(this.current);
    };
    const { shuffle, repeat } = track;
    apply((t) => withMode(t, mode));
    if (!(await this.#attempt(() => mediaMode(track.appId, mode)))) apply((t) => ({ ...t, shuffle, repeat }));
  }

  /** Brings the app or browser of a session to the front. */
  async open(track: Track): Promise<void> {
    await this.#attempt(() => openMediaSource(track.appId));
  }

  /**
   * « Détacher » : for a video, a small player that has the right to play it (îlot's own
   * window, the browser's floating player, or the window that plays kept on top: the core
   * picks, src-tauri/src/video.rs); « Ramener la vidéo » once one is out; the music
   * mini-player otherwise. True when something came off the island.
   */
  async detach(track: Track | null = this.shown): Promise<boolean> {
    if (!track || !isVideo(track)) {
      this.ondetach();
      return this.#attempt(() => openMiniPlayer());
    }
    if (this.video.detached) return this.dock();
    let out = false;
    const done = await this.#attempt(async () => {
      const outcome = await detachVideo(track.appId, !reduced());
      this.#hint(detachTip(outcome));
      out = outcome !== "needsGesture";
      if (out) this.ondetach();
    });
    return done && out;
  }

  /** « Ramener la vidéo » : back where it came from (into the island, into its tab, or
   * the window as it was). */
  async dock(): Promise<boolean> {
    this.#hint(null);
    return this.#attempt(() => dockVideo(!reduced()));
  }

  /** Opens îlot's Crunchyroll window, where the user signs in and picks an episode. */
  async openCrunchyroll(): Promise<void> {
    await this.#attempt(() => openCrunchyroll());
  }

  async playVideo(videoId: string, start = 0): Promise<void> {
    await this.#attempt(() => playYoutube(videoId, start));
  }

  /** Runs a call to the core; a refusal is shown for a few seconds. True when it went through. */
  async #attempt(action: () => Promise<void>): Promise<boolean> {
    try {
      await action();
      this.#notify(null);
      return true;
    } catch (error) {
      this.#notify(String(error));
      return false;
    }
  }

  #hint(message: string | null): void {
    clearTimeout(this.#tipTimer);
    this.tip = message;
    if (message) this.#tipTimer = setTimeout(() => (this.tip = null), TIP_MS);
  }

  #notify(message: string | null): void {
    clearTimeout(this.#noticeTimer);
    this.notice = message;
    if (message) this.#noticeTimer = setTimeout(() => (this.notice = null), NOTICE_MS);
  }

  /** The clock runs only while a session plays: nothing ticks at rest. */
  #syncClock(): void {
    const running = this.current?.playing || this.sessions.some((t) => t.playing);
    this.now = Date.now();
    if (running && this.#clock === undefined) {
      this.#clock = setInterval(() => (this.now = Date.now()), 1000);
    } else if (!running && this.#clock !== undefined) {
      clearInterval(this.#clock);
      this.#clock = undefined;
    }
  }
}

export const media = new MediaStore();
