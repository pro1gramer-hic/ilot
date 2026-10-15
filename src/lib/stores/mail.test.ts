import { beforeEach, describe, expect, it, vi } from "vitest";
import type { MailDraft } from "../ipc";

// The e-mail store hands drafts to the core through ipc.ts: replaced here, so that the
// test sees exactly when a draft leaves.
const core = vi.hoisted(() => ({ sent: [] as MailDraft[], refuse: null as string | null, hubs: [] as unknown[], toasts: [] as string[] }));

vi.mock("../ipc", () => ({
  mailApp: async () => ({ name: "Microsoft Outlook", mapi: true }),
  mailSend: async (draft: MailDraft) => {
    if (core.refuse) throw core.refuse;
    core.sent.push(draft);
    return { way: "mapi", client: "Microsoft Outlook", note: null };
  },
}));
vi.mock("../island/controller.svelte", () => ({
  island: { openHub: (options: unknown) => core.hubs.push(options), closeHub: () => core.hubs.push("closed") },
}));
vi.mock("./banners.svelte", () => ({ banners: { toast: (_tone: string, title: string) => core.toasts.push(title) } }));
vi.mock("./chat.svelte", () => ({ reason: (error: unknown) => String(error) }));

const { handedTitle, mail } = await import("./mail.svelte");

beforeEach(() => {
  core.sent = [];
  core.refuse = null;
  core.hubs = [];
  core.toasts = [];
  mail.discard();
});

describe("the e-mail draft", () => {
  it("opens in the hub with the file, and nothing leaves while it is written", () => {
    mail.open({ id: "f1", name: "rapport.pdf", kind: "other" });
    expect(core.hubs).toEqual([{ view: "mail" }]);
    expect(mail.subject).toBe("rapport.pdf");
    mail.to = "lea@exemple.fr";
    mail.body = "Bonjour";
    expect(core.sent).toEqual([]);
  });

  it("leaves on « Envoyer » only, once, then the hub folds and a toast says where", async () => {
    mail.open({ id: "f1", name: "rapport.pdf", kind: "other" });
    mail.to = "lea@exemple.fr";
    await mail.send();
    expect(core.sent).toEqual([{ to: "lea@exemple.fr", subject: "rapport.pdf", body: "", attachment: "f1" }]);
    expect(core.toasts).toEqual(["Message ouvert dans Microsoft Outlook"]);
    expect(core.hubs.at(-1)).toBe("closed");
    expect(mail.to).toBe("");
  });

  it("a refusal stays on the draft, which is kept", async () => {
    core.refuse = "Adresse invalide : oops";
    mail.to = "oops";
    await mail.send();
    expect(mail.error).toBe("Adresse invalide : oops");
    expect(mail.to).toBe("oops");
    expect(mail.sending).toBe(false);
  });

  it("names the way the draft went", () => {
    expect(handedTitle("mapi", "Microsoft Outlook")).toBe("Message ouvert dans Microsoft Outlook");
    expect(handedTitle("mailto", "ta messagerie")).toBe("Message ouvert dans ta messagerie");
  });
});
