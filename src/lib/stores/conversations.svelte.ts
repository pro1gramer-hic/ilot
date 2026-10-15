// Conversations of the Messages tab (docs/09-DEMO-FINALE.md, section 8): what the apps
// sent during this run, grouped by sender, and the replies the user copied. Memory only,
// capped (notifications.ts): nothing is ever written to disk, and îlot quits with it.
//
// Answering: Windows lets no app trigger another app's « Reply » (section 10). îlot copies
// the reply, brings the conversation's app to the front, and says so. It never types into
// the other app.

import { texts } from "../i18n/live.svelte";
import { type IncomingNotification, copyText, openNotificationApp } from "../ipc";
import {
  type Conversation,
  type NotificationItem,
  addReply,
  conversationKey,
  conversationOf,
  markConversationRead,
  receiveMessage,
  replyDone,
  unreadOf,
} from "../notifications";

/** Who a reply goes to: a thread, or the banner's message. */
export type ReplyTarget = { app: string; appLabel: string; sender: string; notificationId: string };

/** What happened to a reply, to say it on screen. */
export type ReplyOutcome = { ok: boolean; message: string };

class ConversationsStore {
  /** Newest first. */
  list = $state<Conversation[]>([]);
  /** Thread open in the Messages tab. */
  selectedKey = $state<string | null>(null);
  unread = $derived(this.list.reduce((sum, c) => sum + unreadOf(c), 0));
  /** The chosen thread, or the newest one with something unread, or the newest. */
  selected = $derived(
    this.list.find((c) => c.key === this.selectedKey) ?? this.list.find((c) => unreadOf(c) > 0) ?? this.list[0] ?? null,
  );

  #replies = 0;

  receive(incoming: IncomingNotification): void {
    this.list = receiveMessage(this.list, incoming);
  }

  select(key: string): void {
    this.selectedKey = key;
    this.markRead(key);
  }

  markRead(key: string): void {
    this.list = markConversationRead(this.list, key);
  }

  /** « Marquer comme lu » or « Ouvrir » on a banner: its thread is read too. */
  readNotification(notificationId: string): void {
    const conversation = conversationOf(this.list, notificationId);
    if (conversation) this.markRead(conversation.key);
  }

  /** Copies the reply, opens the app, and keeps the reply in its thread. */
  async reply(target: ReplyTarget, text: string): Promise<ReplyOutcome> {
    const reply = text.trim();
    if (!reply) return { ok: false, message: texts().notifications.writeFirst };
    try {
      await copyText(reply);
    } catch (error) {
      return { ok: false, message: texts().notifications.notCopied(String(error)) };
    }
    const key = conversationOf(this.list, target.notificationId)?.key ?? conversationKey(target.app, target.sender);
    this.list = addReply(this.list, key, reply, Date.now(), `reply-${++this.#replies}`);
    let opened = true;
    try {
      await openNotificationApp(target.notificationId);
    } catch {
      opened = false;
    }
    return { ok: true, message: replyDone(target.appLabel, opened) };
  }

  clear(): void {
    this.list = [];
    this.selectedKey = null;
  }
}

export const conversations = new ConversationsStore();

/** Who a reply to the banner's message goes to. */
export function bannerTarget(item: NotificationItem): ReplyTarget {
  return { app: item.app, appLabel: item.appLabel, sender: item.sender, notificationId: item.id };
}

/** The thread a reply to this conversation goes to. */
export function replyTarget(conversation: Conversation): ReplyTarget {
  const { app, appLabel, sender, notificationId } = conversation;
  return { app, appLabel, sender, notificationId };
}
