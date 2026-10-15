// Springs the interface reads: `value` is reactive, and all moving springs share one
// animation frame loop that stops as soon as every spring rests. With reduced motion,
// or while the island cannot be seen, a new target is reached at once.

import { prefersReducedMotion } from "svelte/motion";
import { stillness } from "./motion.svelte";
import { DAMPING, RESPONSE_S, SpringModel } from "./spring";

/** Springs on their way; plain bookkeeping, not state. */
let moving: Spring[] = [];
let frame = 0;
let last = 0;

function tick(now: number): void {
  // A long pause (hidden window, breakpoint) never turns into one huge step.
  const seconds = Math.min(0.05, (now - last) / 1000);
  last = now;
  moving = moving.filter((spring) => spring.advance(seconds));
  frame = moving.length > 0 ? requestAnimationFrame(tick) : 0;
}

function wake(spring: Spring): void {
  if (!moving.includes(spring)) moving.push(spring);
  if (frame === 0) {
    last = performance.now();
    frame = requestAnimationFrame(tick);
  }
}

export class Spring {
  value = $state(0);
  readonly #model: SpringModel;

  constructor(value: number, response = RESPONSE_S, damping = DAMPING) {
    this.#model = new SpringModel(value, response, damping);
    this.value = value;
  }

  get target(): number {
    return this.#model.target;
  }

  /** Moves toward `target` from wherever the spring is, at its current speed. */
  set(target: number): void {
    if (prefersReducedMotion.current || stillness.current) {
      this.snap(target);
      return;
    }
    if (target === this.#model.target && this.#model.resting) return;
    this.#model.target = target;
    wake(this);
  }

  /** Gives a kick, e.g. a shake or a squash, without changing the target. */
  push(velocity: number): void {
    if (prefersReducedMotion.current || stillness.current) return;
    this.#model.velocity += velocity;
    wake(this);
  }

  snap(value: number): void {
    this.#model.snap(value);
    moving = moving.filter((spring) => spring !== this);
    this.value = value;
  }

  /** One frame; false once at rest. */
  advance(seconds: number): boolean {
    const still = this.#model.step(seconds);
    this.value = this.#model.value;
    return still;
  }
}
