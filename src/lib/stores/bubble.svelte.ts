// Mochi's speech bubble under the island: one short sentence for 3.5 s.

export const BUBBLE_MS = 3500;

class BubbleStore {
  text = $state<string | null>(null);
  #timer: ReturnType<typeof setTimeout> | undefined;

  say(text: string, ms = BUBBLE_MS): void {
    clearTimeout(this.#timer);
    this.text = text;
    this.#timer = setTimeout(() => (this.text = null), ms);
  }
}

export const bubble = new BubbleStore();
