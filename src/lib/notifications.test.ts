import { describe, expect, it } from "vitest";
import type { IncomingNotification } from "./ipc";
import {
  type Conversation,
  MAX_CONVERSATIONS,
  MAX_THREAD,
  type NotificationItem,
  type Queue,
  addReply,
  advance,
  appColor,
  bannerParts,
  conversationKey,
  conversationOf,
  enqueue,
  firstName,
  hold,
  isPaused,
  lastLine,
  markConversationRead,
  merge,
  messageText,
  quickReplies,
  receiveMessage,
  replyDone,
  take,
  toItem,
  unreadOf,
} from "./notifications";

const incoming = (id: string, app: string, sender: string, text: string): IncomingNotification => ({
  id,
  app,
  appLabel: app === "whatsapp" ? "WhatsApp" : "Discord",
  monogram: app === "whatsapp" ? "Wa" : "Di",
  icon: null,
  sender,
  text,
  masked: false,
  receivedAt: Number(id),
});

const item = (id: string, app: string, sender = "Maman", text = "Coucou"): NotificationItem =>
  toItem(incoming(id, app, sender, text));

const EMPTY: Queue = { current: null, waiting: [] };

const line = (n: NotificationItem) => bannerParts(n).map((p) => p.text).join("");

describe("appColor", () => {
  it("rings a known app in its color and any other in a neutral ink", () => {
    expect(appColor("discord")).toBe("var(--app-discord)");
    expect(appColor("web")).toBe("var(--ink-3)");
  });
});

describe("toItem", () => {
  it("starts a group of one and drops a missing logo", () => {
    const n = item("1", "whatsapp");
    expect(n.count).toBe(1);
    expect("icon" in n).toBe(false);
    expect(toItem({ ...incoming("2", "discord", "kenji", "go"), icon: "data:image/png;base64,AA==" }).icon).toBe(
      "data:image/png;base64,AA==",
    );
  });
});

describe("queue", () => {
  it("shows one banner at a time, in arrival order", () => {
    let q = enqueue(EMPTY, item("1", "whatsapp"));
    q = enqueue(q, item("2", "discord", "kenji"));
    q = advance(q);
    expect(q.current?.id).toBe("1");
    expect(q.waiting.map((w) => w.id)).toEqual(["2"]);
    expect(advance(q)).toBe(q);
    q = advance(take(q, "1").queue);
    expect(q.current?.id).toBe("2");
    expect(q.waiting).toEqual([]);
  });

  it("groups the messages of one app while they wait", () => {
    let q = enqueue(EMPTY, item("1", "discord", "kenji", "on lance ?"));
    q = enqueue(q, item("2", "whatsapp"));
    q = enqueue(q, item("3", "discord", "léa", "j'arrive"));
    q = enqueue(q, item("4", "discord", "sam", "moi aussi"));
    expect(q.waiting.map((w) => [w.id, w.count])).toEqual([
      ["1", 3],
      ["2", 1],
    ]);
    expect(q.waiting[0].sender).toBe("sam");
    expect(line(q.waiting[0])).toBe("Discord · 3 nouveaux messages");
  });

  it("groups a new message into the banner on screen", () => {
    const q = enqueue(advance(enqueue(EMPTY, item("1", "discord", "kenji"))), item("2", "discord", "léa", "go"));
    expect(q.current).toMatchObject({ id: "1", count: 2, sender: "léa", text: "go" });
    expect(q.waiting).toEqual([]);
  });

  it("takes an item from the screen or from the line", () => {
    const q = advance(enqueue(enqueue(EMPTY, item("1", "whatsapp")), item("2", "discord")));
    expect(take(q, "1")).toMatchObject({ queue: { current: null, waiting: [{ id: "2" }] }, item: { id: "1" } });
    expect(take(q, "2")).toMatchObject({ queue: { current: { id: "1" }, waiting: [] }, item: { id: "2" } });
    expect(take(q, "nope").item).toBeUndefined();
  });

  it("puts the banner back in front of the line during a pause", () => {
    const q = hold(advance(enqueue(enqueue(EMPTY, item("1", "whatsapp")), item("2", "discord"))));
    expect(q.current).toBeNull();
    expect(q.waiting.map((w) => w.id)).toEqual(["1", "2"]);
    expect(hold(EMPTY)).toBe(EMPTY);
  });

  it("merges keep the group id and add counts", () => {
    const group = merge({ ...item("1", "discord"), count: 2 }, item("9", "discord", "léa", "salut"));
    expect(group).toMatchObject({ id: "1", count: 3, sender: "léa", text: "salut", receivedAt: 9 });
  });
});

describe("isPaused", () => {
  const none = { doNotDisturb: false, focus: false, sharing: false };

  it("waits during do not disturb, focus and screen sharing", () => {
    expect(isPaused(none)).toBe(false);
    expect(isPaused({ ...none, doNotDisturb: true })).toBe(true);
    expect(isPaused({ ...none, focus: true })).toBe(true);
    expect(isPaused({ ...none, sharing: true })).toBe(true);
  });
});

describe("bannerParts", () => {
  it("shows the sender then the message, on one line", () => {
    expect(bannerParts(item("1", "whatsapp", "Maman", "Tu rentres\nà quelle heure ?"))).toEqual([
      { text: "Maman", strong: true },
      { text: "\u00a0: Tu rentres à quelle heure ?" },
    ]);
  });

  it("names only the sender when the content is masked or empty", () => {
    const masked = { ...item("1", "whatsapp", "Maman", ""), masked: true };
    expect(line(masked)).toBe("Nouveau message de Maman");
    expect(bannerParts(masked)[1]).toEqual({ text: "Maman", strong: true });
    expect(line(item("2", "whatsapp", "Maman", "  "))).toBe("Nouveau message de Maman");
  });

  it("falls back to the app name without a sender", () => {
    expect(line(item("1", "discord", " ", "go"))).toBe("Discord\u00a0: go");
  });

  it("summarizes a group by app, masked or not", () => {
    expect(line({ ...item("1", "discord"), count: 3 })).toBe("Discord · 3 nouveaux messages");
    expect(line({ ...item("1", "discord"), count: 2, masked: true })).toBe("Discord · 2 nouveaux messages");
  });
});

describe("conversations", () => {
  // Real-looking notifications, as the Rust core sends them (src-tauri/src/notifications.rs).
  const lea = (id: string, text: string, at = Number(id)) => ({ ...incoming(id, "whatsapp", "Léa  Ahouansou", text), receivedAt: at });
  const kenji = incoming("5", "discord", "kenji (#général, Projet RPG)", "La PR des ressorts est prête.");
  const masked: IncomingNotification = { ...incoming("6", "whatsapp", "Maman", "secret"), masked: true };

  it("groups messages by app and sender, newest conversation first", () => {
    let list = receiveMessage([], lea("1", "Tu viens ce soir ?"));
    list = receiveMessage(list, kenji);
    list = receiveMessage(list, lea("7", "On se voit à 18h ?"));
    expect(list.map((c) => c.sender)).toEqual(["Léa Ahouansou", "kenji (#général, Projet RPG)"]);
    expect(list[0].messages.map((m) => m.text)).toEqual(["Tu viens ce soir ?", "On se voit à 18h ?"]);
    expect(list[0].notificationId).toBe("7");
    expect(list[0].updatedAt).toBe(7);
    expect(unreadOf(list[0])).toBe(2);
  });

  it("keeps one thread whatever the spacing or case of the sender", () => {
    expect(conversationKey("whatsapp", "Léa  Ahouansou")).toBe(conversationKey("whatsapp", " léa ahouansou"));
    expect(conversationKey("whatsapp", "Léa")).not.toBe(conversationKey("discord", "Léa"));
  });

  it("never keeps the text of a masked message", () => {
    const [thread] = receiveMessage([], masked);
    expect(thread.messages[0]).toMatchObject({ text: "", masked: true });
    expect(messageText(thread.messages[0], false)).toBe("Contenu masqué");
  });

  it("names a thread after its app when the sender is missing", () => {
    expect(receiveMessage([], incoming("1", "discord", "  ", "go"))[0].sender).toBe("Discord");
  });

  it("keeps the logo Windows gave, even when a later message has none", () => {
    const withIcon = { ...lea("1", "a"), icon: "data:image/png;base64,AA" };
    const list = receiveMessage(receiveMessage([], withIcon), lea("2", "b"));
    expect(list[0].icon).toBe("data:image/png;base64,AA");
    expect("icon" in receiveMessage([], lea("3", "c"))[0]).toBe(false);
  });

  it("stays within its caps: conversations and messages", () => {
    let list: Conversation[] = [];
    for (let i = 0; i < MAX_CONVERSATIONS + 5; i++) list = receiveMessage(list, incoming(String(i), "discord", `p${i}`, "x"));
    expect(list).toHaveLength(MAX_CONVERSATIONS);
    expect(list[0].sender).toBe(`p${MAX_CONVERSATIONS + 4}`);
    let one: Conversation[] = [];
    for (let i = 0; i < MAX_THREAD + 3; i++) one = receiveMessage(one, lea(String(i), `m${i}`));
    expect(one[0].messages).toHaveLength(MAX_THREAD);
    expect(one[0].messages[0].text).toBe("m3");
  });

  it("adds a copied reply on the user's side and reads the thread", () => {
    const list = receiveMessage(receiveMessage([], lea("1", "Tu viens ?")), kenji);
    const key = conversationKey("whatsapp", "Léa Ahouansou");
    const replied = addReply(list, key, "J'arrive vers 18h", 99, "r1");
    expect(replied[0].key).toBe(key);
    expect(replied[0].messages.at(-1)).toEqual({ id: "r1", text: "J'arrive vers 18h", masked: false, at: 99, mine: true, read: true });
    expect(unreadOf(replied[0])).toBe(0);
    expect(addReply(list, "nobody", "x", 1, "r2")).toEqual(list);
  });

  it("marks one thread as read, and finds a thread by notification", () => {
    const list = receiveMessage(receiveMessage([], lea("1", "a")), kenji);
    const read = markConversationRead(list, list[0].key);
    expect(unreadOf(read[0])).toBe(0);
    expect(unreadOf(read[1])).toBe(1);
    expect(read[1]).toBe(list[1]);
    expect(conversationOf(list, "1")?.sender).toBe("Léa Ahouansou");
    expect(conversationOf(list, "404")).toBeUndefined();
  });

  it("hides every received text when asked, never the user's own replies", () => {
    const key = conversationKey("whatsapp", "Léa Ahouansou");
    const list = addReply(receiveMessage([], lea("1", "Tu viens ?")), key, "Oui", 2, "r");
    expect(messageText(list[0].messages[0], true)).toBe("Contenu masqué");
    expect(messageText(list[0].messages[1], true)).toBe("Oui");
    expect(lastLine(list[0], true)).toBe("Toi\u00a0: Oui");
    expect(lastLine(receiveMessage([], lea("1", "Tu\nviens ?"))[0], false)).toBe("Tu viens ?");
    expect(lastLine(receiveMessage([], lea("1", "Tu viens ?"))[0], true)).toBe("Contenu masqué");
  });

  it("calls people by their first name", () => {
    expect(firstName("Léa Ahouansou")).toBe("Léa");
    expect(firstName("kenji (#général, Projet RPG)")).toBe("kenji");
    expect(firstName("Kenji dans #dev-ilot")).toBe("Kenji");
    expect(firstName("Awa")).toBe("Awa");
  });

  it("offers three quick replies in the tone of each app", () => {
    expect(quickReplies("whatsapp")).toEqual(["J'arrive vers 18h", "Parfait, merci !", "Je te rappelle"]);
    expect(quickReplies("telegram")).toHaveLength(3);
    expect(quickReplies("web")).toEqual(quickReplies("unknown"));
  });

  it("says that the reply was copied, never sent", () => {
    expect(replyDone("WhatsApp", true)).toBe("Réponse copiée\u00a0: colle-la dans WhatsApp");
    expect(replyDone("Discord", false)).toBe("Réponse copiée\u00a0: ouvre Discord et colle-la");
  });
});
