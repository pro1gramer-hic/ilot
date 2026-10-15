// Agent sessions shown in the island.

import { type AgentSession, countBySource, followedSession, isLive, summary } from "../agents";

class SessionsStore {
  list = $state<AgentSession[]>([]);
  counts = $derived(countBySource(this.list));
  liveCount = $derived(this.list.filter(isLive).length);
  followed = $derived(followedSession(this.list));
  summary = $derived(summary(this.list));
}

export const sessions = new SessionsStore();
