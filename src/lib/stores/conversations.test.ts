import { beforeEach, describe, expect, it, vi } from "vitest";
import type { IncomingNotification } from "../ipc";

// The store copies replies and opens apps through ipc.ts: replaced here by a fake core.
const core = vi.hoisted(() => ({
  calls: [] as unknown[][],
  refuseCopy: null as string | null,
  refuseOpen: null as string | null,
}));

vi.mock("../ipc", () => ({
  copyText: async (text: string) => {
    core.calls.push(["copy_text", text]);
    if (core.refuseCopy) throw core.refuseCopy;
  },
  openNotificationApp: async (id: string) => {
    core.calls.push(["open_notification_app", id]);
    if (core.refuseOpen) throw core.refuseOpen;
  },
}));

const { bannerTarget, conversations, replyTarget } = await import("./conversations.svelte");

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

beforeEach(() => {
  core.calls = [];
  core.refuseCopy = null;
  core.refuseOpen = null;
  conversations.clear();
});

describe("conversations store", () => {
  it("counts unread messages and opens the newest unread thread first", () => {
    conversations.receive(incoming("1", "whatsapp", "Léa", "Tu viens ?"));
    conversations.receive(incoming("2", "discord", "kenji", "PR prête"));
    expect(conversations.unread).toBe(2);
    expect(conversations.selected?.sender).toBe("kenji");
    conversations.select(conversations.list[1].key);
    expect(conversations.selected?.sender).toBe("Léa");
    expect(conversations.unread).toBe(1);
  });

  it("reads the thread of a banner marked as read", () => {
    conversations.receive(incoming("1", "whatsapp", "Léa", "Tu viens ?"));
    conversations.readNotification("1");
    expect(conversations.unread).toBe(0);
    conversations.readNotification("404");
    expect(conversations.list).toHaveLength(1);
  });

  it("copies a reply, opens the app, and keeps the reply in the thread", async () => {
    conversations.receive(incoming("1", "whatsapp", "Léa Ahouansou", "Tu viens ?"));
    const outcome = await conversations.reply(replyTarget(conversations.list[0]), "  J'arrive vers 18h ");
    expect(outcome).toEqual({ ok: true, message: "Réponse copiée\u00a0: colle-la dans WhatsApp" });
    expect(core.calls).toEqual([
      ["copy_text", "J'arrive vers 18h"],
      ["open_notification_app", "1"],
    ]);
    expect(conversations.list[0].messages.at(-1)).toMatchObject({ text: "J'arrive vers 18h", mine: true });
    expect(conversations.unread).toBe(0);
  });

  it("still says the reply is copied when the app cannot be opened", async () => {
    conversations.receive(incoming("1", "discord", "kenji", "PR prête"));
    core.refuseOpen = "Discord n'est plus ouvert.";
    const outcome = await conversations.reply(replyTarget(conversations.list[0]), "Je regarde ce soir");
    expect(outcome).toEqual({ ok: true, message: "Réponse copiée\u00a0: ouvre Discord et colle-la" });
  });

  it("keeps nothing and opens nothing when the copy fails", async () => {
    conversations.receive(incoming("1", "whatsapp", "Léa", "Tu viens ?"));
    core.refuseCopy = "Presse-papiers occupé";
    const outcome = await conversations.reply(replyTarget(conversations.list[0]), "Oui");
    expect(outcome.ok).toBe(false);
    expect(outcome.message).toContain("Presse-papiers occupé");
    expect(core.calls).toEqual([["copy_text", "Oui"]]);
    expect(conversations.list[0].messages).toHaveLength(1);
  });

  it("refuses an empty reply without touching the clipboard", async () => {
    conversations.receive(incoming("1", "whatsapp", "Léa", "Tu viens ?"));
    expect((await conversations.reply(replyTarget(conversations.list[0]), "   ")).ok).toBe(false);
    expect(core.calls).toEqual([]);
  });

  it("answers the banner's message: its notification brings the app back", () => {
    const item = { id: "9", app: "discord", appLabel: "Discord", monogram: "Di", sender: "kenji", text: "", masked: true, count: 2, receivedAt: 9 };
    expect(bannerTarget(item)).toEqual({ app: "discord", appLabel: "Discord", sender: "kenji", notificationId: "9" });
  });

  it("answers a banner whose sender is spelled differently in its thread", async () => {
    conversations.receive(incoming("1", "whatsapp", "Léa  Ahouansou", "Tu viens ?"));
    const target = { app: "whatsapp", appLabel: "WhatsApp", sender: "", notificationId: "1" };
    await conversations.reply(target, "Oui");
    expect(conversations.list[0].messages.at(-1)?.mine).toBe(true);
  });
});
