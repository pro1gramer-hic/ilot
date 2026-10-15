// The user's safety controls in the panel: the decisions journal (`decisions()` and
// `decision:added`, kept by the core in memory, never on disk) and « Tout suspendre »
// (`suspend_agents`, echoed by `agents:suspended`). îlot never decides anything here: it
// shows what the user decided, and the suspension only ever refuses.

import { addDecision } from "../agents";
import { type Decision, decisions as journal, suspendAgents } from "../ipc";
import { sound } from "./sound.svelte";

class DecisionsStore {
  /** Newest first. */
  list = $state.raw<Decision[]>([]);
  /** Every agent is paused and every request refused until the user resumes. */
  suspended = $state(false);
  error = $state<string | null>(null);

  async load(): Promise<void> {
    for (const decision of await journal()) this.add(decision);
  }

  add(decision: Decision): void {
    this.list = addDecision(this.list, decision);
  }

  /** « Tout suspendre » / « Tout reprendre ». Shown at once; the core's echo confirms. */
  async toggleSuspended(): Promise<void> {
    sound.play("suspend");
    const on = !this.suspended;
    this.suspended = on;
    try {
      await suspendAgents(on);
      this.error = null;
    } catch (error) {
      this.suspended = !on;
      this.error = String(error);
    }
  }
}

export const decisions = new DecisionsStore();
