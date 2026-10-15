// Mochi's expressive moves: played once in the banner or the panel, then back to calm.
// The pill only ever breathes, sleeps or shakes, plus a small hop (docs/03-DESIGN.md,
// "Mochi").

import type { FaceId } from "../mochi/faces";
import type { MoveId } from "../mochi/moves";

/** Length of one expressive play, in milliseconds (whole cycles of the move). */
export const ONE_SHOT_MS: Partial<Record<MoveId, number>> = {
  bounce: 2000,
  rock: 2800,
  roll: 4000,
  peek: 3200,
  party: 2400,
  melt: 4000,
};

/** How long a face reaction to the user's answer stays. */
const REACTION_MS = 2000;

class MoodStore {
  expressive = $state<MoveId | null>(null);
  /** A face imposed for a moment (refusal → mischievous, allowed → happy). */
  face = $state<FaceId | undefined>(undefined);
  /** Counts the hops asked for: Mochi jumps a little each time it changes. */
  hops = $state(0);
  #timer: ReturnType<typeof setTimeout> | undefined;
  #faceTimer: ReturnType<typeof setTimeout> | undefined;

  /** Reaction to the user's answer to a permission: refusal → mischievous. */
  react(answer: "deny" | "always" | "allow"): void {
    this.flash(answer === "deny" ? "wink" : "happy");
  }

  /** Shows a face for a moment, over whatever move plays. */
  flash(face: FaceId, ms = REACTION_MS): void {
    clearTimeout(this.#faceTimer);
    this.face = face;
    this.#faceTimer = setTimeout(() => (this.face = undefined), ms);
  }

  /** A small jump: a task ended, or the user played with Mochi. */
  hop(): void {
    this.hops += 1;
  }

  playOnce(move: MoveId): void {
    clearTimeout(this.#timer);
    this.expressive = move;
    this.#timer = setTimeout(() => (this.expressive = null), ONE_SHOT_MS[move] ?? 2000);
  }
}

export const mood = new MoodStore();
