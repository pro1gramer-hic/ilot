// Messaging notifications waiting for the user, in memory only (docs/05-NOTIFICATIONS.md).
//
// For the banner: `current` is the message to show now (null when there is none, or while
// do not disturb, a focus session or a screen share holds messages back). When the banner
// closes, call `dismiss(id)`; to keep the message in « Plus tard », call `later(id)`. A
// message from the same app while one is shown is grouped into `current` (same id, higher
// `count`). The banner owns its display time and the permission-first rule; Mochi's
// « Coucou » plays when `current` changes. `focusActive` is driven by phase 7.

import {
  type IncomingNotification,
  NO_NOTIFICATIONS,
  type NotificationStatus,
  notificationStatus,
  openNotificationApp,
  openNotificationSettings,
  requestNotificationAccess,
} from "../ipc";
import { MAX_POSTPONED, type NotificationItem, type Queue, advance, enqueue, hold, isPaused, take, toItem } from "../notifications";
import { conversations } from "./conversations.svelte";
import { settings } from "./settings.svelte";
import { untrack } from "svelte";

/** How long an « Ouvrir » error stays on its item. */
const NOTICE_MS = 4000;

class NotificationsStore {
  status = $state<NotificationStatus>({ ...NO_NOTIFICATIONS });
  queue = $state<Queue>({ current: null, waiting: [] });
  /** The message to show now. */
  current = $derived(this.queue.current);
  /** Messages not seen yet: the one on screen (or held by the « Pastille » mode) and
   * those waiting behind it. */
  unread = $derived((this.queue.current ? 1 : 0) + this.queue.waiting.length);
  /** « Plus tard » list of the Home tab, newest first. */
  postponed = $state<NotificationItem[]>([]);
  /** A focus session runs (set by phase 7): messages wait for its end. */
  focusActive = $state(false);
  /** The user shares the screen (switched by hand in the settings): messages wait. */
  sharing = $state(false);
  /** Last « Ouvrir » that failed, shown on its item for a few seconds. */
  notice = $state<{ id: string; message: string } | null>(null);
  paused = $derived(
    isPaused({ doNotDisturb: settings.value.doNotDisturb, focus: this.focusActive, sharing: this.sharing }),
  );

  #noticeTimer: ReturnType<typeof setTimeout> | undefined;

  constructor() {
    $effect.root(() => {
      $effect(() => {
        const paused = this.paused;
        untrack(() => (paused ? (this.queue = hold(this.queue)) : this.#show()));
      });
    });
  }

  /** A notification from the Rust core; its thread in the Messages tab keeps it too. */
  receive(incoming: IncomingNotification): void {
    conversations.receive(incoming);
    this.queue = enqueue(this.queue, toItem(incoming));
    this.#show();
  }

  /** The banner closed: the message leaves îlot (Windows keeps its own copy). */
  dismiss(id: string): void {
    this.#remove(id);
  }

  /** Keeps a message in « Plus tard » until it is read. */
  later(id: string): void {
    const item = this.#remove(id);
    if (item) this.postponed = [item, ...this.postponed.filter((p) => p.id !== id)].slice(0, MAX_POSTPONED);
  }

  /** The user opened the Messages tab from the pill's badge: every waiting message has
   * been seen there (« Pastille » mode). */
  seeAll(): void {
    this.queue = { current: null, waiting: [] };
  }

  /** Marks a « Plus tard » message as read. */
  read(id: string): void {
    this.postponed = this.postponed.filter((p) => p.id !== id);
  }

  /** « Ouvrir »: brings the sender's app or browser to the front, then forgets the message. */
  async open(id: string): Promise<void> {
    try {
      await openNotificationApp(id);
      this.#remove(id);
      this.read(id);
    } catch (error) {
      this.notice = { id, message: String(error) };
      clearTimeout(this.#noticeTimer);
      this.#noticeTimer = setTimeout(() => (this.notice = null), NOTICE_MS);
    }
  }

  async refreshStatus(): Promise<void> {
    this.status = await notificationStatus();
  }

  async requestAccess(): Promise<void> {
    this.status = await requestNotificationAccess();
  }

  openWindowsSettings(): Promise<void> {
    return openNotificationSettings();
  }

  #remove(id: string): NotificationItem | undefined {
    const { queue, item } = take(this.queue, id);
    this.queue = queue;
    this.#show();
    return item;
  }

  #show(): void {
    if (!this.paused) this.queue = advance(this.queue);
  }
}

export const notifications = new NotificationsStore();
