// Messaging notifications as the island shows them (docs/05-NOTIFICATIONS.md): one at a
// time, grouped by app, held during do not disturb, focus and screen sharing. Pure
// functions used by stores/notifications.svelte.ts; nothing here touches the disk.

import { texts } from "./i18n/live.svelte";
import type { IncomingNotification } from "./ipc";

/** « Plus tard » keeps at most this many items (memory only). */
export const MAX_POSTPONED = 20;

export type NotificationItem = {
  id: string;
  /** Rule id ("whatsapp"). */
  app: string;
  appLabel: string;
  /** App logo as a data: URL; absent for web apps and unknown logos. */
  icon?: string;
  /** Letters shown instead of the logo. */
  monogram: string;
  sender: string;
  /** Empty when masked. */
  text: string;
  masked: boolean;
  /** Messages grouped in this item (« Discord · 3 nouveaux messages »). */
  count: number;
  /** Epoch milliseconds of the latest message. */
  receivedAt: number;
};

const APP_COLORS = new Set(["whatsapp", "discord", "snapchat", "telegram", "messenger", "instagram", "teams", "slack", "gmail"]);

/** Ring around an app's icon: its own color when îlot knows it, a neutral ring otherwise. */
export function appColor(app: string): string {
  return APP_COLORS.has(app) ? `var(--app-${app})` : "var(--ink-3)";
}

export function toItem(incoming: IncomingNotification): NotificationItem {
  const { icon, ...rest } = incoming;
  return { ...rest, ...(icon ? { icon } : {}), count: 1 };
}

/** The group keeps its id; the latest message gives the sender and the text. */
export function merge(group: NotificationItem, next: NotificationItem): NotificationItem {
  return { ...next, id: group.id, count: group.count + next.count };
}

export type Queue = {
  /** Banner on screen. */
  current: NotificationItem | null;
  /** Waiting, oldest first, one item per app. */
  waiting: NotificationItem[];
};

/** Adds a message, grouped with the banner on screen or the waiting item of the same app. */
export function enqueue(queue: Queue, item: NotificationItem): Queue {
  if (queue.current?.app === item.app) return { ...queue, current: merge(queue.current, item) };
  const index = queue.waiting.findIndex((w) => w.app === item.app);
  if (index < 0) return { ...queue, waiting: [...queue.waiting, item] };
  return { ...queue, waiting: queue.waiting.map((w, i) => (i === index ? merge(w, item) : w)) };
}

/** Puts the next waiting item on screen when the screen is free. */
export function advance(queue: Queue): Queue {
  if (queue.current || queue.waiting.length === 0) return queue;
  const [next, ...rest] = queue.waiting;
  return { current: next, waiting: rest };
}

/** Takes an item out of the queue, wherever it is. */
export function take(queue: Queue, id: string): { queue: Queue; item?: NotificationItem } {
  if (queue.current?.id === id) return { queue: { ...queue, current: null }, item: queue.current };
  const item = queue.waiting.find((w) => w.id === id);
  return { queue: { ...queue, waiting: queue.waiting.filter((w) => w.id !== id) }, item };
}

/** Pause: the banner on screen goes back to the front of the line, shown again later. */
export function hold(queue: Queue): Queue {
  if (!queue.current) return queue;
  return { current: null, waiting: [queue.current, ...queue.waiting] };
}

export type PauseReasons = {
  doNotDisturb: boolean;
  /** Focus session (phase 7). */
  focus: boolean;
  /** Screen sharing, switched by hand: Windows does not report it. */
  sharing: boolean;
};

/** Messages wait (never lost) while any of these holds. Permissions never wait. */
export function isPaused(reasons: PauseReasons): boolean {
  return reasons.doNotDisturb || reasons.focus || reasons.sharing;
}

export type BannerPart = { text: string; strong?: boolean };

/** One line, single spaces: the banner has no room for line breaks. */
function oneLine(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

/**
 * The banner line: « Maman : Tu rentres ? », « Nouveau message de Maman » when masked,
 * « Discord · 3 nouveaux messages » for a group. The colon follows the language's typography.
 */
export function bannerParts(item: NotificationItem): BannerPart[] {
  const sender = oneLine(item.sender) || item.appLabel;
  if (item.count > 1) return [{ text: item.appLabel, strong: true }, { text: texts().notifications.grouped(item.count) }];
  const text = oneLine(item.text);
  if (item.masked || !text) return [{ text: texts().notifications.newFrom }, { text: sender, strong: true }];
  return [{ text: sender, strong: true }, { text: texts().notifications.said(text) }];
}

// Conversations of the Messages tab: the messages received during this run, grouped by
// app and sender, with the replies the user copied. Memory only, capped.

/** Conversations kept at most, newest first. */
export const MAX_CONVERSATIONS = 24;
/** Messages kept per conversation, the oldest go first. */
export const MAX_THREAD = 40;

export type ThreadMessage = {
  id: string;
  /** Empty when masked. */
  text: string;
  masked: boolean;
  /** Epoch milliseconds. */
  at: number;
  /** A reply of the user (copied, then pasted in the app by the user). */
  mine: boolean;
  read: boolean;
};

export type Conversation = {
  /** App and sender: one thread per person (or channel) of an app. */
  key: string;
  app: string;
  appLabel: string;
  monogram: string;
  icon?: string;
  sender: string;
  /** Latest notification of this thread, the one « Ouvrir » brings to the front. */
  notificationId: string;
  /** Oldest first. */
  messages: ThreadMessage[];
  updatedAt: number;
};

export function conversationKey(app: string, sender: string): string {
  return `${app}\u0000${oneLine(sender).toLowerCase()}`;
}

/** Puts a conversation first and keeps the list within its cap. */
function touch(list: readonly Conversation[], conversation: Conversation): Conversation[] {
  return [conversation, ...list.filter((c) => c.key !== conversation.key)].slice(0, MAX_CONVERSATIONS);
}

function append(messages: readonly ThreadMessage[], message: ThreadMessage): ThreadMessage[] {
  return [...messages, message].slice(-MAX_THREAD);
}

/** A message from the Rust core, added to its sender's thread. */
export function receiveMessage(list: readonly Conversation[], incoming: IncomingNotification): Conversation[] {
  const sender = oneLine(incoming.sender) || incoming.appLabel;
  const key = conversationKey(incoming.app, sender);
  const known = list.find((c) => c.key === key);
  const icon = incoming.icon ?? known?.icon;
  const message: ThreadMessage = {
    id: incoming.id,
    text: incoming.masked ? "" : incoming.text,
    masked: incoming.masked,
    at: incoming.receivedAt,
    mine: false,
    read: false,
  };
  return touch(list, {
    key,
    app: incoming.app,
    appLabel: incoming.appLabel,
    monogram: incoming.monogram,
    ...(icon ? { icon } : {}),
    sender,
    notificationId: incoming.id,
    messages: append(known?.messages ?? [], message),
    updatedAt: incoming.receivedAt,
  });
}

/** A reply the user copied, shown on the right of its thread; answering reads the thread. */
export function addReply(list: readonly Conversation[], key: string, text: string, at: number, id: string): Conversation[] {
  const known = list.find((c) => c.key === key);
  if (!known) return [...list];
  const read = known.messages.map((m) => (m.read ? m : { ...m, read: true }));
  return touch(list, {
    ...known,
    messages: append(read, { id, text, masked: false, at, mine: true, read: true }),
    updatedAt: at,
  });
}

export function markConversationRead(list: readonly Conversation[], key: string): Conversation[] {
  return list.map((c) =>
    c.key === key && c.messages.some((m) => !m.read) ? { ...c, messages: c.messages.map((m) => ({ ...m, read: true })) } : c,
  );
}

/** The thread holding this notification, if any. */
export function conversationOf(list: readonly Conversation[], notificationId: string): Conversation | undefined {
  return list.find((c) => c.messages.some((m) => m.id === notificationId));
}

export function unreadOf(conversation: Conversation): number {
  return conversation.messages.filter((m) => !m.mine && !m.read).length;
}

/** What a message says on screen: nothing of its content when it is masked or hidden. */
export function messageText(message: ThreadMessage, hidden: boolean): string {
  if (message.mine) return message.text;
  return hidden || message.masked || !message.text ? texts().notifications.masked : message.text;
}

/** Last line of a thread in the list: « Toi : J'arrive », or the masked content. */
export function lastLine(conversation: Conversation, hidden: boolean): string {
  const last = conversation.messages.at(-1);
  if (!last) return texts().notifications.noMessage;
  return last.mine ? texts().notifications.mine(oneLine(last.text)) : oneLine(messageText(last, hidden));
}

/** « Léa » for « Léa Ahouansou », « kenji » for « kenji (#général, Projet RPG) ». */
export function firstName(sender: string): string {
  return oneLine(sender).split(/[\s,(]/)[0] || oneLine(sender);
}

/** Apps with answers of their own; the others get the plain ones. */
const QUICK_APPS = ["whatsapp", "discord", "snapchat", "teams", "slack", "gmail"] as const;

/** Three short answers in the tone of the app. */
export function quickReplies(app: string): readonly [string, string, string] {
  const replies = texts().notifications.quickReplies;
  return replies[QUICK_APPS.find((known) => known === app) ?? "other"];
}

/** Windows lets no app answer another app's notification (docs/09-DEMO-FINALE.md, section
 * 10): îlot copies the reply and opens the conversation, and says so. */
export function replyDone(appLabel: string, opened: boolean): string {
  return texts().notifications.replyCopied(appLabel, opened);
}
