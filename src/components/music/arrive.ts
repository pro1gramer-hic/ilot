// How content enters and leaves an opening of the island (docs/09-DEMO-FINALE.md,
// « Mouvement »): it comes into focus, from a blur, at 0.97 scale, with an exponential
// ease-out, and leaves faster than it came. With reduced motion, a plain fade.

import { expoOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";
import { ENTER_MS, EXIT_MS, STAGGER_MS, reduced } from "../../lib/motion.svelte";

/** Blur of content on its way in, in pixels. */
const BLUR_PX = 6;
/** Scale it grows from. */
const FROM_SCALE = 0.97;

export function arrive(_node: Element, { index = 0 }: { index?: number } = {}): TransitionConfig {
  const delay = index * STAGGER_MS;
  if (reduced()) return { delay, duration: ENTER_MS, css: (t) => `opacity: ${t}` };
  return {
    delay,
    duration: ENTER_MS + 2 * STAGGER_MS,
    easing: expoOut,
    css: (t, u) =>
      `opacity: ${t}; filter: blur(${(u * BLUR_PX).toFixed(2)}px); transform: scale(${(FROM_SCALE + (1 - FROM_SCALE) * t).toFixed(4)})`,
  };
}

export function depart(_node: Element, { duration = EXIT_MS }: { duration?: number } = {}): TransitionConfig {
  if (reduced()) return { duration, css: (t) => `opacity: ${t}` };
  return { duration, css: (t, u) => `opacity: ${t}; filter: blur(${(u * BLUR_PX).toFixed(2)}px)` };
}
