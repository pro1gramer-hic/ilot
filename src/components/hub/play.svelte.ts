// What the hub remembers between two openings, and Mochi's answer to clicks (09, section
// 8 bis, « Trop de clics »): squashed with big eyes, then annoyed, then, at the fifth
// click within 1.6 s, out of breath for three seconds in a view of his own.

import { DIZZY_SECONDS, type PokeReaction, poke } from "../../lib/hub";
import { island } from "../../lib/island/controller.svelte";
import { sound } from "../../lib/stores/sound.svelte";

/** How long a squash lasts, and the face that follows it. */
const SQUISH_MS = 450;
const FACE_MS = 700;

class HubPlay {
  /** The session the user put on the left card (its key), kept while îlot runs. */
  chosen = $state<string | null>(null);
  /** Mochi is squashed by a click. */
  squished = $state(false);
  /** His face after a click (big eyes, then annoyed). */
  reaction = $state<Exclude<PokeReaction, "dizzy"> | null>(null);
  /** Seconds before he gets his breath back. */
  countdown = $state(0);

  #times: number[] = [];
  #squishTimer: ReturnType<typeof setTimeout> | undefined;
  #faceTimer: ReturnType<typeof setTimeout> | undefined;
  #dizzyTimer: ReturnType<typeof setInterval> | undefined;

  /** A click on Mochi in the hub. */
  poke(at = performance.now()): void {
    const { times, reaction } = poke(this.#times, at);
    this.#times = times;
    if (reaction === "dizzy") {
      sound.play("dizzy");
      this.#outOfBreath();
      return;
    }
    sound.play("click");
    clearTimeout(this.#squishTimer);
    clearTimeout(this.#faceTimer);
    this.squished = true;
    this.reaction = reaction;
    this.#squishTimer = setTimeout(() => (this.squished = false), SQUISH_MS);
    this.#faceTimer = setTimeout(() => (this.reaction = null), FACE_MS);
  }

  /** « Trop de clics d'un coup »: a countdown, then back to the home of the hub. */
  #outOfBreath(): void {
    clearInterval(this.#dizzyTimer);
    this.squished = false;
    this.reaction = null;
    this.countdown = DIZZY_SECONDS;
    island.showHub("dizzy");
    this.#dizzyTimer = setInterval(() => {
      this.countdown -= 1;
      if (this.countdown > 0) return;
      clearInterval(this.#dizzyTimer);
      if (island.hubOpen && island.hubView === "dizzy") island.showHub("home");
    }, 1000);
  }
}

export const play = new HubPlay();
