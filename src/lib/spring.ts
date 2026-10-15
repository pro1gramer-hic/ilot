// The demo's interruptible spring (docs/09-DEMO-FINALE.md, section 1): a damped
// oscillator toward a target, defined by its response (0.34 s) and its damping ratio
// (0.82). A new target never makes it jump: position and velocity carry on. Same
// integration as `class Sp` in reference/ilot-demo-finale.html (4 ms steps).

export const RESPONSE_S = 0.34;
export const DAMPING = 0.82;
/** Integration step, in seconds. */
const STEP_S = 0.004;
/** Close enough to rest: under these, the spring snaps to its target and stops. */
const REST_DISTANCE = 0.01;
const REST_SPEED = 0.01;

/** Stiffness of a spring that answers in `response` seconds. */
export function stiffnessOf(response: number): number {
  return (2 * Math.PI / response) ** 2;
}

export class SpringModel {
  value: number;
  target: number;
  velocity = 0;
  readonly #stiffness: number;
  readonly #friction: number;

  constructor(value: number, response = RESPONSE_S, damping = DAMPING) {
    this.value = value;
    this.target = value;
    this.#stiffness = stiffnessOf(response);
    this.#friction = 2 * damping * Math.sqrt(this.#stiffness);
  }

  get resting(): boolean {
    return this.value === this.target && this.velocity === 0;
  }

  /** Jumps to `value` at rest (reduced motion, first placement). */
  snap(value: number): void {
    this.value = value;
    this.target = value;
    this.velocity = 0;
  }

  /** Advances by `seconds`; returns true while it still moves. */
  step(seconds: number): boolean {
    const steps = Math.max(1, Math.ceil(seconds / STEP_S));
    const h = seconds / steps;
    for (let i = 0; i < steps; i++) {
      const acceleration = -this.#stiffness * (this.value - this.target) - this.#friction * this.velocity;
      this.velocity += acceleration * h;
      this.value += this.velocity * h;
    }
    if (Math.abs(this.value - this.target) < REST_DISTANCE && Math.abs(this.velocity) < REST_SPEED) {
      this.snap(this.target);
      return false;
    }
    return true;
  }
}
