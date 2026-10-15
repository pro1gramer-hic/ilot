// An answer to a permission only counts after a deliberate look (docs/06-SECURITE.md):
// the request has been on screen, in the same place, for a moment, and to allow it, the
// user could see all of it. Each new request, or the same one moving from the island to
// the panel, starts the wait again: a double click, a double Enter or a held key never
// reaches the next request.

/** Answers to a permission, as the core receives them. */
export type PermissionAnswer = "deny" | "always" | "allow" | "terminal";

/** Long enough for a double click, a key's auto-repeat and the island opening. */
export const LOOK_BEFORE_ANSWER_MS = 700;

export type Surface = "island" | "panel";

export class AnswerGuard {
  #shown: { surface: Surface; id: string; at: number } | undefined;
  /** "surface:id" of the requests shown in full there (the opened island or the panel card, read to its end). */
  #whole = new Set<string>();

  /** The request whose answer buttons are on screen now, and where (or none). */
  show(on: { surface: Surface; id: string } | undefined, now: number): void {
    if (on?.surface === this.#shown?.surface && on?.id === this.#shown?.id) return;
    this.#shown = on && { ...on, at: now };
  }

  /** The same request shows something new to answer (the next question of an agent):
   * the look starts again, as for a new request. */
  restart(id: string, now: number): void {
    if (this.#shown?.id === id) this.#shown = { ...this.#shown, at: now };
  }

  setWhole(surface: Surface, id: string, whole: boolean): void {
    if (whole) this.#whole.add(`${surface}:${id}`);
    else this.#whole.delete(`${surface}:${id}`);
  }

  isWhole(surface: Surface, id: string): boolean {
    return this.#whole.has(`${surface}:${id}`);
  }

  /** Refusing, or leaving the answer to the terminal, needs the look; allowing needs the
   * look at the whole request. */
  ready(id: string, answer: PermissionAnswer, now: number): boolean {
    const shown = this.#shown;
    if (shown?.id !== id || now - shown.at < LOOK_BEFORE_ANSWER_MS) return false;
    return answer === "deny" || answer === "terminal" || this.isWhole(shown.surface, id);
  }

  forget(id: string): void {
    this.#whole.delete(`island:${id}`);
    this.#whole.delete(`panel:${id}`);
  }
}
