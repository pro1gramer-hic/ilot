// Mochi's animation clock: 30 frames per second at most, like the final demo's loop.
// Every Mochi of a page shares one timer, so all of them are drawn in the same task, and
// nothing runs at all when none is playing (no wake-up on every display refresh either).

/** Mochi's animation rate, as in reference/ilot-demo-finale.html. */
export const FPS = 30;

const PERIOD_MS = 1000 / FPS;

export interface Clock {
  /** Milliseconds, monotonic. */
  now(): number;
  setTimeout(callback: () => void, ms: number): number;
  clearTimeout(id: number): void;
}

export const browserClock: Clock = {
  now: () => performance.now(),
  setTimeout: (callback, ms) => setTimeout(callback, ms),
  clearTimeout: (id) => clearTimeout(id),
};

/** Quantizes elapsed time to the 30 frames/s clock: floor(ms / 1000 × 30) / 30, as in the demo. */
export function sampleTime(elapsedMs: number): { frame: number; t: number } {
  const frame = Math.floor((elapsedMs / 1000) * FPS);
  return { frame, t: frame / FPS };
}

/** One timer for many players, waking up on the boundaries of the 30 frames/s grid. */
export class Ticker {
  readonly #clock: Clock;
  readonly #players = new Set<(now: number) => void>();
  #timer: number | undefined;

  constructor(clock: Clock = browserClock) {
    this.#clock = clock;
  }

  /** Calls `play(now)` at once, then on every tick until the returned function is called. */
  add(play: (now: number) => void): () => void {
    this.#players.add(play);
    play(this.#clock.now());
    if (this.#timer === undefined) this.#schedule();
    return () => {
      this.#players.delete(play);
      if (this.#players.size === 0 && this.#timer !== undefined) {
        this.#clock.clearTimeout(this.#timer);
        this.#timer = undefined;
      }
    };
  }

  #schedule(): void {
    let delay = PERIOD_MS - (this.#clock.now() % PERIOD_MS);
    // Woken right on a boundary: the next tick is the following one, not one at once.
    if (delay < PERIOD_MS / 4) delay += PERIOD_MS;
    this.#timer = this.#clock.setTimeout(this.#tick, delay);
  }

  #tick = (): void => {
    this.#timer = undefined;
    const now = this.#clock.now();
    for (const play of [...this.#players]) play(now);
    if (this.#players.size > 0 && this.#timer === undefined) this.#schedule();
  };
}

/** The page's ticker, shared by every Mochi. */
const ticker = new Ticker();

/**
 * Calls `draw(t)` right away, then once per new frame, with `t` the quantized seconds
 * elapsed since `origin` (a clock `now()` value). Returns a function that stops it.
 *
 * Frames are counted from halfway between two ticks, at most one frame before `origin`:
 * a tick that fires a few milliseconds late still lands in the frame it was meant for, so
 * each tick shows exactly the next frame, never the same one twice.
 *
 * `every` keeps one frame in that many (3: 10 frames/s), for slow moves that play for
 * hours: each frame drawn makes the whole window composited again.
 */
export function playFrames(draw: (t: number) => void, origin: number, on: Ticker = ticker, every = 1): () => void {
  const start = (Math.round(origin / PERIOD_MS) - 0.5) * PERIOD_MS;
  let lastFrame = Number.NaN;
  return on.add((now) => {
    const frame = Math.floor(sampleTime(now - start).frame / every) * every;
    if (frame === lastFrame) return;
    lastFrame = frame;
    draw(frame / FPS);
  });
}
