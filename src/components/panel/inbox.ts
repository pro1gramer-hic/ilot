// Messages îlot holds right now, for « Messages récents » and the rail's unread badge:
// the one on screen, those waiting their turn and those kept for later. Memory only.

import type { NotificationItem, Queue } from "../../lib/notifications";

/** Newest first, each message once. */
export function inbox(queue: Queue, postponed: readonly NotificationItem[]): NotificationItem[] {
  const seen = new Set<string>();
  return [queue.current, ...queue.waiting, ...postponed]
    .filter((item): item is NotificationItem => item !== null && !seen.has(item.id) && Boolean(seen.add(item.id)))
    .sort((a, b) => b.receivedAt - a.receivedAt);
}

/** Unread messages, a grouped item counting all of its messages. */
export function unreadCount(items: readonly NotificationItem[]): number {
  return items.reduce((sum, item) => sum + item.count, 0);
}
