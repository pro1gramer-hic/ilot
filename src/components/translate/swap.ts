// A text replaced by its translation, or back (prompt 12, DeepL): the new words come into
// focus from a light blur instead of jumping in. In place, no slide, no scale; with reduced
// motion, a plain fade.

import { expoOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";
import { reduced } from "../../lib/motion.svelte";

const SWAP_MS = 180;
const BLUR_PX = 3;

export function swap(_node: Element, { duration = SWAP_MS }: { duration?: number } = {}): TransitionConfig {
  if (reduced()) return { duration, css: (t) => `opacity: ${t}` };
  return { duration, easing: expoOut, css: (t, u) => `opacity: ${t}; filter: blur(${(u * BLUR_PX).toFixed(2)}px)` };
}
