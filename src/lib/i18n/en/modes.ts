// modes, in English: the same keys as ../fr/modes.ts.

import type { Catalog } from "../fr";

export const modes: Catalog["modes"] = {
  music: { label: "Music in the folded island", ticker: "Scrolling title", disc: "Record", equalizer: "Equalizer", none: "Nothing" },
  agents: { label: "Agents in the pill", counter: "Counter", dots: "Dots only", activity: "Activity" },
  messages: { label: "Messages", preview: "Preview", sender: "Sender only", badge: "Badge" },
  done: { label: "End of a task", banner: "Banner", quiet: "Quiet" },
  mochi: { label: "Mochi outside the island", wanders: "Wanders", follows: "Follows you", stays: "Stays in the island", desktop: "On the desktop" },
  relief: { label: "Depth", deep: "3D", light: "Light", flat: "Flat" },
  position: { label: "Position", free: "Free", magnet: "Snaps to edges" },
};
