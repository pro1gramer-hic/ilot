// Continuous work time, to suggest a break after 2 h (Mochi "Fond"), once per session of
// work. A pause of 15 min or more starts the count again.

export const BREAK_AFTER_MS = 2 * 60 * 60 * 1000;
export const RESET_AFTER_IDLE_MS = 15 * 60 * 1000;

export class WorkClock {
  #since: number | null = null;
  #lastActive: number | null = null;
  #suggested = false;

  /** Called regularly with whether an agent works. Returns true once, when a break is due. */
  update(now: number, working: boolean): boolean {
    if (!working) {
      if (this.#lastActive !== null && now - this.#lastActive >= RESET_AFTER_IDLE_MS) this.reset();
      return false;
    }
    if (this.#since === null) this.#since = now;
    this.#lastActive = now;
    if (!this.#suggested && now - this.#since >= BREAK_AFTER_MS) {
      this.#suggested = true;
      return true;
    }
    return false;
  }

  reset(): void {
    this.#since = null;
    this.#lastActive = null;
    this.#suggested = false;
  }
}
