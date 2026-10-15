// The chat's state (prompt 12, lot 3). The conversation lives in memory only, for as long
// as îlot runs: nothing here is written to disk.
//
// Contract for the other lots:
// - `chat.open(target?: { provider: ChatProvider; model: string })` opens the hub on the
//   chat, on that provider and model when given (lot 4: a click on the Ollama pill calls
//   `chat.open({ provider: "ollama", model })`; lot 5: the shortcut calls `chat.open()`).
//   Without a target, the chat keeps the model picked in it, or takes the default one of
//   Réglages › Chat.
// - `chat.thinking` is true while a model answers: Mochi thinks in the pill and the hub.
// - `chat.askAbout(attachment)` opens the chat with a dropped file joined to the question.

import { type ChatMessage, isProvider, providerInfo, readyToSend, requestId, turnsOf, withDictation } from "../chat";
import { texts, textsLocale } from "../i18n/live.svelte";
import {
  type Attachment,
  type ChatProvider,
  type ContextPreview,
  type ContextProposal,
  chatCancel,
  chatModels,
  chatSend,
  contextConfirm,
  contextForget,
  contextRead,
  dictationAvailable,
  dictationStart,
  dictationStop,
  onCoreEvent,
  secretHas,
  secretRemove,
  secretStore,
} from "../ipc";
import { island } from "../island/controller.svelte";
import { mic } from "./mic.svelte";
import { settings } from "./settings.svelte";

export type ModelList = { list: string[]; loading: boolean; error: string | null };

/** The text of a refusal from the core (a string) or of an exception. */
export function reason(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

export class ChatStore {
  provider = $state<ChatProvider>("anthropic");
  model = $state("");
  messages = $state<ChatMessage[]>([]);
  draft = $state("");
  /** A dropped file joined to the next question. */
  attachment = $state<Attachment | null>(null);
  /** A window's text the user previewed and joined to the next question. */
  window = $state<{ id: string; title: string; app: string } | null>(null);
  /** A window Mochi landed on, waiting for « Lire la fenêtre ». */
  proposal = $state<ContextProposal | null>(null);
  /** Its text, read and shown, waiting for « Joindre au chat ». */
  preview = $state<ContextPreview | null>(null);
  reading = $state(false);
  contextError = $state<string | null>(null);
  /** A model is answering. */
  thinking = $state(false);
  /** Whether a key is saved, by provider (the local ones need none). */
  keys = $state<Partial<Record<ChatProvider, boolean>>>({});
  models = $state<Partial<Record<ChatProvider, ModelList>>>({});
  /** Windows has its French recognizer: the mic shows. */
  canDictate = $state(false);
  get listening(): boolean {
    return mic.dictating;
  }

  set listening(on: boolean) {
    mic.dictating = on;
  }
  /** Words being spoken, before the phrase settles. */
  heard = $state("");
  dictationError = $state<string | null>(null);
  #request: string | null = null;
  /** The user picked a model in the chat: the default of Réglages no longer applies. */
  #picked = false;
  #connected = false;

  /** Listens to the core: streamed answers, windows offered, dictation. */
  connect(): void {
    if (this.#connected) return;
    this.#connected = true;
    onCoreEvent("chat:delta", ({ requestId: id, text }) => this.#append(id, text));
    onCoreEvent("chat:done", ({ requestId: id }) => this.#end(id));
    onCoreEvent("chat:error", ({ requestId: id, message }) => this.#end(id, message));
    onCoreEvent("context:proposed", (proposal) => this.propose(proposal));
    onCoreEvent("dictation:text", ({ text, final }) => this.#hear(text, final));
    onCoreEvent("dictation:end", ({ error }) => {
      this.listening = false;
      this.heard = "";
      this.dictationError = error;
    });
  }

  /** Whether Windows has a recognizer for the interface's language: asked the first time
   * the chat shows, and again when the language changes. */
  #askedDictation: string | null = null;

  #checkDictation(): void {
    const locale = textsLocale();
    if (this.#askedDictation === locale) return;
    this.#askedDictation = locale;
    void dictationAvailable().then((available) => (this.canDictate = available));
  }

  /** Opens the hub on the chat; see the contract at the top of this file. */
  open(target?: { provider: ChatProvider; model: string }): void {
    if (target && isProvider(target.provider)) {
      this.choose(target.provider, target.model);
    } else if (!this.#picked && this.messages.length === 0) {
      this.provider = settings.value.chat.provider;
      this.model = settings.value.chat.model;
    }
    island.openHub({ view: "chat" });
    void this.refreshKey(this.provider);
    this.#checkDictation();
  }

  /** Another provider or model for the next questions (a click on the model's name). */
  choose(provider: ChatProvider, model: string): void {
    this.#picked = true;
    this.provider = provider;
    this.model = model;
    void this.refreshKey(provider);
  }

  async refreshKey(provider: ChatProvider): Promise<void> {
    const info = providerInfo(provider);
    this.keys[provider] = info.secret ? await secretHas(info.secret) : true;
  }

  /** « Enregistrer la clé »: straight to the vault; the field is emptied by the caller. */
  async saveKey(provider: ChatProvider, key: string): Promise<string | null> {
    const secret = providerInfo(provider).secret;
    if (!secret) return null;
    try {
      await secretStore(secret, key);
      this.keys[provider] = true;
      delete this.models[provider];
      return null;
    } catch (error) {
      return reason(error);
    }
  }

  /** « Effacer »: the key leaves the vault. */
  async removeKey(provider: ChatProvider): Promise<string | null> {
    const secret = providerInfo(provider).secret;
    if (!secret) return null;
    try {
      await secretRemove(secret);
      this.keys[provider] = false;
      delete this.models[provider];
      return null;
    } catch (error) {
      return reason(error);
    }
  }

  /** The provider's models, read from it once (again with `force`). */
  async loadModels(provider: ChatProvider, force = false): Promise<void> {
    const known = this.models[provider];
    if (known?.loading || (known && !known.error && !force)) return;
    this.models[provider] = { list: known?.list ?? [], loading: true, error: null };
    try {
      const list = await chatModels(provider);
      this.models[provider] = { list, loading: false, error: list.length ? null : texts().common.chat.noModel };
    } catch (error) {
      this.models[provider] = { list: [], loading: false, error: reason(error) };
    }
  }

  /** Sends the question, with the joined file and window. */
  async send(): Promise<void> {
    const text = this.draft.trim();
    const joined = this.attachment !== null || this.window !== null;
    if (this.thinking || !readyToSend(text, joined)) return;
    const provider = this.provider;
    if (!this.model) {
      await this.loadModels(provider);
      this.model = this.models[provider]?.list[0] ?? "";
    }
    const id = requestId(Date.now(), Math.random());
    const question: ChatMessage = {
      id: `u${id}`,
      role: "user",
      text,
      ...(this.attachment ? { file: this.attachment.name } : {}),
      ...(this.window ? { window: this.window.title } : {}),
    };
    const turns = turnsOf([...this.messages, question]);
    this.messages = [...this.messages, question, { id, role: "assistant", text: "", by: providerInfo(provider).label, pending: true }];
    const attachment = this.attachment?.id;
    const context = this.window?.id;
    this.draft = "";
    this.attachment = null;
    this.window = null;
    if (!this.model) {
      this.#request = id;
      this.thinking = true;
      this.#end(id, texts().common.chat.modelFirst);
      return;
    }
    this.#request = id;
    this.thinking = true;
    try {
      await chatSend({ requestId: id, provider, model: this.model, turns, ...(attachment ? { attachment } : {}), ...(context ? { context } : {}) });
    } catch (error) {
      this.#end(id, reason(error));
    }
    // The window's text went with this question (or failed to): the core forgets it.
    if (context) void contextForget(context);
  }

  /** « Arrêter »: the answer stops where it is. */
  stop(): void {
    const id = this.#request;
    if (!id) return;
    void chatCancel(id);
    const answer = this.messages.find((m) => m.id === id);
    if (answer) answer.stopped = true;
    this.#end(id);
  }

  /** « Nouvelle conversation »: everything said is forgotten. */
  clear(): void {
    this.stop();
    this.messages = [];
    this.attachment = null;
    this.removeWindow();
  }

  /** « Demander à Mochi » on a dropped file. */
  askAbout(attachment: Attachment): void {
    this.attachment = attachment;
    this.open();
  }

  detach(): void {
    this.attachment = null;
  }

  /** Mochi landed on a window: the chat offers it (nothing is read yet). */
  propose(proposal: ContextProposal): void {
    if (this.proposal && this.proposal.id !== proposal.id) void contextForget(this.proposal.id);
    this.proposal = proposal;
    this.preview = null;
    this.contextError = null;
    island.openHub({ view: "chat" });
  }

  /** « Lire la fenêtre »: its text comes back for a preview; nothing is sent. */
  async readContext(): Promise<void> {
    const proposal = this.proposal;
    if (!proposal || proposal.blocked || this.reading) return;
    this.reading = true;
    this.contextError = null;
    try {
      this.preview = (await contextRead(proposal.id)) ?? null;
    } catch (error) {
      this.contextError = reason(error);
    } finally {
      this.reading = false;
    }
  }

  /** « Joindre au chat »: the previewed text goes with the next question. */
  async joinContext(): Promise<void> {
    const preview = this.preview;
    if (!preview) return;
    try {
      await contextConfirm(preview.id);
      this.removeWindow();
      this.window = { id: preview.id, title: preview.title, app: preview.app };
      this.proposal = null;
      this.preview = null;
    } catch (error) {
      this.contextError = reason(error);
    }
  }

  /** « Non merci »: the window is forgotten, read or not. */
  dismissContext(): void {
    if (this.proposal) void contextForget(this.proposal.id);
    this.proposal = null;
    this.preview = null;
    this.contextError = null;
  }

  /** The window's chip removed before sending. */
  removeWindow(): void {
    if (this.window) void contextForget(this.window.id);
    this.window = null;
  }

  /** The mic: starts listening, or stops. */
  async toggleDictation(): Promise<void> {
    if (this.listening) {
      await dictationStop();
      return;
    }
    this.listening = true;
    this.heard = "";
    this.dictationError = null;
    try {
      await dictationStart();
    } catch (error) {
      this.listening = false;
      this.dictationError = reason(error);
    }
  }

  #hear(text: string, final: boolean): void {
    if (final) {
      this.draft = withDictation(this.draft, text);
      this.heard = "";
    } else {
      this.heard = text;
    }
  }

  #append(id: string, text: string): void {
    const answer = this.messages.find((m) => m.id === id);
    if (answer && !answer.stopped) answer.text += text;
  }

  #end(id: string, error?: string): void {
    const answer = this.messages.find((m) => m.id === id);
    if (answer) {
      answer.pending = false;
      if (error && !answer.stopped) answer.error = error;
    }
    if (this.#request === id) {
      this.#request = null;
      this.thinking = false;
    }
  }
}

export const chat = new ChatStore();
