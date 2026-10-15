// The last hour of every AI (lib/timeline.ts), in memory only: fed by startup.ts from the
// core's events, or by the browser preview. `now` moves on while the panel shows it.

import type { AgentSession, AiSource } from "../agents";
import type { Decision } from "../ipc";
import { type MarkKind, type Timeline, addMark, decide, emptyTimeline, prune, record } from "../timeline";

class TimelineStore {
  value = $state.raw<Timeline>(emptyTimeline(Date.now()));
  now = $state(Date.now());

  /** The session list goes from `previous` to `next`. */
  record(previous: readonly AgentSession[], next: readonly AgentSession[], at = Date.now()): void {
    this.value = record(this.value, previous, next, at);
    this.now = at;
  }

  mark(source: AiSource, kind: MarkKind, label: string, at = Date.now()): void {
    this.value = addMark(this.value, source, kind, label, at);
    this.now = at;
  }

  decide(decision: Decision): void {
    this.value = decide(this.value, decision);
  }

  /** Replaces everything (browser preview). */
  replace(timeline: Timeline, at = Date.now()): void {
    this.value = timeline;
    this.now = at;
  }

  /** One second later: bars grow, what left the hour goes. */
  tick(at = Date.now()): void {
    this.value = prune(this.value, at);
    this.now = at;
  }
}

export const timeline = new TimelineStore();
