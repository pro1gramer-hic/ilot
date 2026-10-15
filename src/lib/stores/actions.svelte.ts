// The actions a shortcut or « OK Mochi » can start (src/lib/shortcuts.ts). The module that
// owns an action gives its handler (src/lib/actions.ts); an action without one stays out
// of the Raccourcis tab and of the phrases Mochi listens for.

import { ACTIONS, type Action, type ActionId } from "../shortcuts";

type Handler = () => void;

export class ActionRegistry {
  /** Actions with a real handler. */
  ready = $state<ActionId[]>([]);
  #handlers: Partial<Record<ActionId, Handler>> = {};

  /** The actions the tab lists, in their order. */
  available: Action[] = $derived(ACTIONS.filter((a) => this.ready.includes(a.id)));

  /** The real handler of `id`; returns what takes it back. */
  provide(id: ActionId, handler: Handler): () => void {
    this.#handlers[id] = handler;
    if (!this.ready.includes(id)) this.ready = [...this.ready, id];
    return () => {
      if (this.#handlers[id] !== handler) return;
      delete this.#handlers[id];
      this.ready = this.ready.filter((a) => a !== id);
    };
  }

  has(id: ActionId): boolean {
    return this.ready.includes(id);
  }

  /** Starts `id`; false when it has no real handler. */
  run(id: string): boolean {
    const action = ACTIONS.find((a) => a.id === id)?.id;
    if (!action || !this.has(action)) return false;
    this.#handlers[action]?.();
    return true;
  }
}

export const actions = new ActionRegistry();
