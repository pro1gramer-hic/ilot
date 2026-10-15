// How the island moves (docs/03-DESIGN.md, "Mouvement"): physical springs from
// svelte/motion for sizes and positions, short fades with a 4 to 6 px slide for content,
// transform and opacity only. With reduced motion, everything becomes a plain fade.

import { cubicIn, cubicOut, expoOut } from "svelte/easing";
import { prefersReducedMotion } from "svelte/motion";
import type { TransitionConfig } from "svelte/transition";

/** Springs, as svelte/motion options. `island` overshoots a little, then settles. */
export const SPRINGS = {
  island: { stiffness: 0.11, damping: 0.56 },
  /** Mochi gliding between the pill and the panel header (FLIP). */
  glide: { stiffness: 0.13, damping: 0.66 },
  /** Indicators that slide under the chosen tab or segment. */
  indicator: { stiffness: 0.2, damping: 0.72 },
  /** Small physical answers: a squash, a jump, a press. */
  bounce: { stiffness: 0.22, damping: 0.42 },
} as const;

/** Gap between two items that enter one after the other. */
export const STAGGER_MS = 30;
/** How far content slides while it fades in. */
export const SHIFT_PX = 5;
export const ENTER_MS = 260;
export const EXIT_MS = 120;

/** True when the user asked for less motion: springs jump, content only fades. */
export function reduced(): boolean {
  return prefersReducedMotion.current;
}

/** Content entering: a fade with a short slide up, delayed by its place in the line. */
export function rise(
  _node: Element,
  { index = 0, delay = 0, shift = SHIFT_PX, axis = "y" }: { index?: number; delay?: number; shift?: number; axis?: "x" | "y" } = {},
): TransitionConfig {
  const start = delay + index * STAGGER_MS;
  if (reduced()) return { delay: start, duration: ENTER_MS, css: (t) => `opacity: ${t}` };
  const move = axis === "x" ? "translateX" : "translateY";
  return {
    delay: start,
    duration: ENTER_MS,
    easing: cubicOut,
    css: (t, u) => `opacity: ${t}; transform: ${move}(${(u * shift).toFixed(2)}px)`,
  };
}

/** Content leaving: faster than it came, no slide. */
export function vanish(_node: Element, { duration = EXIT_MS }: { duration?: number } = {}): TransitionConfig {
  return { duration, css: (t) => `opacity: ${t}` };
}

/** An opening's content (docs/09-DEMO-FINALE.md, « Mouvement »): it comes out of a
 * blur, from 0.97, on an exponential ease-out, a moment after the shape starts moving. */
export const BLOOM_MS = 420;
export const WILT_MS = 140;
const BLUR_PX = 6;

export function bloom(_node: Element, { delay = 60 }: { delay?: number } = {}): TransitionConfig {
  if (reduced()) return { delay, duration: ENTER_MS, css: (t) => `opacity: ${t}` };
  return {
    delay,
    duration: BLOOM_MS,
    easing: expoOut,
    css: (t, u) => `opacity: ${Math.min(1, t * 1.3)}; filter: blur(${(u * BLUR_PX).toFixed(2)}px); transform: scale(${1 - 0.03 * u})`,
  };
}

/** …and leaves faster than it came, into the same blur. */
export function wilt(_node: Element, { duration = WILT_MS }: { duration?: number } = {}): TransitionConfig {
  if (reduced()) return { duration, css: (t) => `opacity: ${t}` };
  return {
    duration,
    easing: cubicIn,
    css: (t, u) => `opacity: ${t}; filter: blur(${(u * BLUR_PX).toFixed(2)}px); transform: scale(${1 - 0.03 * u})`,
  };
}

/** Options to set a spring: instant under reduced motion. */
export function springTo(): { instant: boolean } {
  return { instant: reduced() };
}

/** Whether the island is hidden (page hidden, or hidden for a full-screen app): loops
 * stop. Set by the root, read by components that run their own animations. */
class Stillness {
  current = $state(false);
}

export const stillness = new Stillness();
