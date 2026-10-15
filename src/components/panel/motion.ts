// How a tab's content comes and goes (docs/09-DEMO-FINALE.md, section 1): it arrives out of
// a light blur, fading in from a 0.97 scale on an exponential ease-out, and leaves faster
// than it came. With reduced motion, both are plain fades.

import { expoOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";
import { ENTER_MS, EXIT_MS, reduced } from "../../lib/motion.svelte";

/** Blur at the start of the entrance, in pixels. */
const BLUR_PX = 6;
const FROM_SCALE = 0.97;

/** Svelte hands the element first; these transitions do not need it. */
type Transition = (node: Element) => TransitionConfig;

export const arrive: Transition = () => {
  if (reduced()) return { duration: ENTER_MS, css: (t) => `opacity: ${t}` };
  return {
    duration: ENTER_MS,
    easing: expoOut,
    css: (t, u) => `opacity: ${t}; filter: blur(${(u * BLUR_PX).toFixed(2)}px); transform: scale(${FROM_SCALE + (1 - FROM_SCALE) * t})`,
  };
};

export const leave: Transition = () => {
  if (reduced()) return { duration: EXIT_MS, css: (t) => `opacity: ${t}` };
  return {
    duration: EXIT_MS,
    css: (t, u) => `opacity: ${t}; filter: blur(${(u * BLUR_PX * 0.5).toFixed(2)}px)`,
  };
};
