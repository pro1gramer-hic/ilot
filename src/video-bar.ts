// Commands bar of the small video player: a page of its own, in a window with its own
// capability (capabilities/video-bar.json). ?site=youtube|crunchyroll names its player.
// The browser preview adds &media=…&theme=… and paints the picture under the bar.
import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
// The Terminal and Chaos Pixel themes write the interface in these.
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import "@fontsource/pixelify-sans/latin-600.css";
import "./lib/tokens.css";
import { mount } from "svelte";
import VideoBar from "./VideoBar.svelte";
import type { VideoSite } from "./lib/ipc";
import { inTauri, onCoreEvent } from "./lib/ipc";
import { ownTrack } from "./lib/media";
import { readMiniPreview, sceneSnapshot, startFakeCore } from "./lib/media-preview";
import { media } from "./lib/stores/media.svelte";
import { settings } from "./lib/stores/settings.svelte";

const site: VideoSite = new URLSearchParams(location.search).get("site") === "crunchyroll" ? "crunchyroll" : "youtube";

mount(VideoBar, { target: document.getElementById("app")!, props: { site } });

if (inTauri) {
  onCoreEvent("settings:changed", (value) => settings.apply(value));
  void Promise.all([settings.load(), media.connect()]);
} else {
  const options = readMiniPreview(location.search);
  settings.apply({ ...settings.value, theme: options.theme });
  const snapshot = sceneSnapshot(options.media, Date.now());
  // The preview plays îlot's own window: the session of the bar's site becomes îlot's.
  const mine = site === "youtube" ? "YouTube" : "Crunchyroll";
  const own = snapshot.sessions.map((t) => (t.source === mine ? { ...t, app: "îlot", appId: "ilot.exe", videoId: site === "youtube" ? "dQw4w9WgXcQ" : undefined } : t));
  startFakeCore(media, { ...snapshot, sessions: own, current: own.find((t) => t.playing) ?? null });
  // The bottom edge of the video under the bar: its picture stands in for it.
  const picture = ownTrack(own, site)?.artwork;
  document.documentElement.dataset.preview = "";
  if (picture) document.body.style.background = `url("${picture}") center bottom / 100% auto no-repeat, var(--opaque)`;
  Object.assign(window, { __ilot: { media } });
}
