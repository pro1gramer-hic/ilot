// The mini-player window thrown across the screen (docs/09-DEMO-FINALE.md, section 5): it
// follows the pointer, keeps a little of its speed when let go, never leaves the screen,
// and melts back into the island when dropped close to it. Pure geometry, in screen
// pixels (logical); MiniPlayer.svelte moves the window.

export type Point = { x: number; y: number };
export type Sample = Point & { t: number };
export type Bounds = { left: number; top: number; right: number; bottom: number };

/** Pointer samples kept to measure the speed at release, in milliseconds. */
export const SAMPLE_MS = 100;
/** How far a throw carries: the release speed (px/s) times this, in seconds. */
export const THROW_S = 0.08;
/** Dropped this close to the island (center to center), the player goes back into it. */
export const DOCK_RADIUS = 120;

/** Keeps the samples of the last moments of a drag, newest last. */
export function keepRecent(samples: readonly Sample[], next: Sample): Sample[] {
  return [...samples, next].filter((s) => next.t - s.t <= SAMPLE_MS);
}

/** Speed at release, in pixels per second, from the samples of the last moments. */
export function velocityOf(samples: readonly Sample[]): Point {
  if (samples.length < 2) return { x: 0, y: 0 };
  const first = samples[0];
  const last = samples[samples.length - 1];
  const seconds = (last.t - first.t) / 1000;
  if (seconds <= 0) return { x: 0, y: 0 };
  return { x: (last.x - first.x) / seconds, y: (last.y - first.y) / seconds };
}

/** Where a window of `size` thrown from `at` comes to rest, kept inside `bounds`. */
export function throwTarget(at: Point, velocity: Point, size: { width: number; height: number }, bounds: Bounds): Point {
  const clamp = (value: number, min: number, max: number): number => Math.min(Math.max(min, max), Math.max(min, value));
  return {
    x: Math.round(clamp(at.x + velocity.x * THROW_S, bounds.left, bounds.right - size.width)),
    y: Math.round(clamp(at.y + velocity.y * THROW_S, bounds.top, bounds.bottom - size.height)),
  };
}

/** True when the player was let go close enough to the island to go back into it. */
export function nearHome(center: Point, home: Point, radius = DOCK_RADIUS): boolean {
  return Math.hypot(center.x - home.x, center.y - home.y) < radius;
}
