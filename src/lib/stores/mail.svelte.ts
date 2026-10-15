// The e-mail draft written in the island (prompt 12, lot 3). Nothing leaves before
// « Envoyer »: then the core hands the draft to the mail app, which opens it ready to send
// (src-tauri/src/mail.rs). The draft lives in memory only.

import { texts } from "../i18n/live.svelte";
import { type Attachment, type MailApp, mailApp, mailSend } from "../ipc";
import { island } from "../island/controller.svelte";
import { banners } from "./banners.svelte";
import { reason } from "./chat.svelte";

/** What the toast says once the draft is handed over. */
export function handedTitle(way: "mapi" | "outlook" | "mailto", client: string): string {
  return texts().notifications.mailOpened(way === "mailto" ? null : client);
}

class MailStore {
  to = $state("");
  subject = $state("");
  body = $state("");
  attachment = $state<Attachment | null>(null);
  /** The mail app named before « Envoyer ». */
  app = $state<MailApp | null>(null);
  sending = $state(false);
  error = $state<string | null>(null);

  /** « Joindre à un e-mail »: the draft opens in the hub with the file. */
  open(attachment?: Attachment): void {
    if (attachment) {
      this.attachment = attachment;
      if (!this.subject.trim()) this.subject = attachment.name;
    }
    this.error = null;
    island.openHub({ view: "mail" });
    void mailApp().then((app) => (this.app = app));
  }

  /** « Envoyer »: the only way a draft leaves the island. */
  async send(): Promise<void> {
    if (this.sending) return;
    this.sending = true;
    this.error = null;
    try {
      const handed = await mailSend({
        to: this.to,
        subject: this.subject,
        body: this.body,
        ...(this.attachment ? { attachment: this.attachment.id } : {}),
      });
      if (handed) banners.toast("ok", handedTitle(handed.way, handed.client), handed.note ?? texts().notifications.mailCheck, 4200);
      this.discard();
      island.closeHub();
    } catch (error) {
      this.error = reason(error);
    } finally {
      this.sending = false;
    }
  }

  /** « Annuler »: the draft is forgotten. */
  discard(): void {
    this.to = "";
    this.subject = "";
    this.body = "";
    this.attachment = null;
    this.error = null;
  }
}

export const mail = new MailStore();
