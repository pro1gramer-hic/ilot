// Mini-player window: a separate page with its own capability (capabilities/mini.json).
import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
// The Terminal and Chaos Pixel themes write the interface in these.
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import "@fontsource/pixelify-sans/latin-600.css";
import "./lib/tokens.css";
import { mount } from "svelte";
import MiniPlayer from "./MiniPlayer.svelte";
import { inTauri, onCoreEvent } from "./lib/ipc";
import { readMiniPreview, sceneSnapshot, startFakeCore } from "./lib/media-preview";
import { stillness } from "./lib/motion.svelte";
import { media } from "./lib/stores/media.svelte";
import { settings } from "./lib/stores/settings.svelte";

mount(MiniPlayer, { target: document.getElementById("app")! });

// The record stops while the window is hidden, as it does on the island.
document.addEventListener("visibilitychange", () => {
  stillness.current = document.hidden;
  document.documentElement.toggleAttribute("data-still", document.hidden);
});

if (inTauri) {
  onCoreEvent("settings:changed", (value) => settings.apply(value));
  void Promise.all([settings.load(), media.connect()]);
} else {
  // The window is transparent around the pill: the preview puts the desktop behind it.
  document.documentElement.dataset.preview = "";
  const options = readMiniPreview(location.search);
  settings.apply({ ...settings.value, theme: options.theme });
  startFakeCore(media, sceneSnapshot(options.media, Date.now()));
  if (options.tab) media.select(options.tab);
  Object.assign(window, { __ilot: { media } });
}
