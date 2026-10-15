// Focus session: a timer during which message banners wait (notifications.focusActive),
// announced by Mochi when it ends. The clock ticks only while a session runs.

import { texts } from "../i18n/live.svelte";
import { bubble } from "./bubble.svelte";
import { mood } from "./mood.svelte";
import { notifications } from "./notifications.svelte";

class FocusStore {
  endsAt = $state<number | null>(null);
  now = $state(Date.now());
  active = $derived(this.endsAt !== null);
  remainingMs = $derived(this.endsAt === null ? 0 : Math.max(0, this.endsAt - this.now));
  #clock: ReturnType<typeof setInterval> | undefined;

  start(minutes: number): void {
    this.endsAt = Date.now() + minutes * 60_000;
    this.now = Date.now();
    notifications.focusActive = true;
    clearInterval(this.#clock);
    this.#clock = setInterval(() => this.#tick(), 1000);
  }

  stop(): void {
    clearInterval(this.#clock);
    this.#clock = undefined;
    this.endsAt = null;
    notifications.focusActive = false;
  }

  #tick(): void {
    this.now = Date.now();
    if (this.endsAt !== null && this.now >= this.endsAt) {
      this.stop();
      mood.flash("star", 4000);
      bubble.say(texts().mochi.say.focusDone, 6000);
    }
  }
}

export const focus = new FocusStore();
