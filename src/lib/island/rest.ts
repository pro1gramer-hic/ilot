// The island at rest. « Transparente au repos » (prompt 12, lot 1): when nothing goes on,
// it stays in place, clickable, but turns see-through; it comes back on hover or as soon
// as something happens. And, whatever that setting, a little later its webview is asked
// for as little memory as it can. Pure rules; stores/rest.svelte.ts keeps the time.

import type { IslandMode } from "./model";

/** Nothing going on for this long (and the pointer away), the island turns see-through. */
export const REST_AFTER_MS = 4000;
/** Quiet for this long, the island's webview is asked for little memory. */
export const MEMORY_AFTER_MS = 15_000;

export type Activity = {
  mode: IslandMode;
  hubOpen: boolean;
  panelOpen: boolean;
  /** An agent works, or asks in its terminal. */
  working: boolean;
  /** Music plays. */
  playing: boolean;
  /** A permission waits (it always shows). */
  waiting: boolean;
  /** The island is moved, Mochi is pulled, a file is dragged. */
  handled: boolean;
  /** Mochi is out for a walk or follows the cursor: he comes back to the island. */
  roaming: boolean;
  /** The pointer is on or near the island. */
  near: boolean;
};

/** True when nothing goes on: the island may fade. */
export function isQuiet(a: Activity): boolean {
  return a.mode === "pill" && !a.hubOpen && !a.panelOpen && !a.working && !a.playing && !a.waiting && !a.handled && !a.roaming && !a.near;
}
