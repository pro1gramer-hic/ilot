// How the hub's content changes (09, sections 1 and 8 bis): a session that moves to the
// left card, or a view that replaces another, comes into focus out of a light blur,
// rising a few pixels, on an exponential ease-out. Reduced motion keeps the fade only.

import { expoOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";
import { reduced } from "../../lib/motion.svelte";

const SWAP_MS = 320;
const BLUR_PX = 4;
const RISE_PX = 8;

/** Svelte hands the element first; this transition does not need it. */
export const swap: (node: Element) => TransitionConfig = () => {
  if (reduced()) return { duration: SWAP_MS / 2, css: (t) => `opacity: ${t}` };
  return {
    duration: SWAP_MS,
    easing: expoOut,
    css: (t, u) => `opacity: ${t}; filter: blur(${(u * BLUR_PX).toFixed(2)}px); transform: translateY(${(u * RISE_PX).toFixed(2)}px)`,
  };
};
