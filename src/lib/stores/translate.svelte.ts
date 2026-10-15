// DeepL in the interface (prompt 12): whether a key is saved, the translations asked for
// (in memory only, by what was translated and into which language), and « Traduire le
// presse-papiers », which shows the copied text on the island before anything leaves.
// Nothing reaches DeepL without a click on « Traduire »; nothing is written to disk.

import { textsLocale } from "../i18n/live.svelte";
import {
  type ClipboardFailure,
  type TranslateFailure,
  copyText,
  focusIsland,
  readClipboardText,
  secretHas,
  secretRemove,
  secretStore,
  translateText,
} from "../ipc";
import { island } from "../island/controller.svelte";
import { DEEPL_SECRET, MAX_TRANSLATE_CHARS, failureOf, targetOf, translatable } from "../translate";
import { banners } from "./banners.svelte";
import { settings } from "./settings.svelte";

/** A text's « Traduire »: asked, translated (shown, or its original shown again), refused,
 * or, without a key, the button offering to add one. */
export type TranslationState =
  | { status: "pending" }
  | { status: "done"; text: string; detected: string; original: boolean }
  | { status: "error"; failure: TranslateFailure }
  | { status: "offer" };

/** The clipboard on the island: waiting for « Traduire », on its way, translated, refused;
 * or no key (the clipboard was not read), or nothing to translate in it. */
export type ClipboardAsk = {
  status: "confirm" | "sending" | "done" | "error" | "noKey" | "unreadable";
  /** The copied text (empty when it was not read). */
  text: string;
  /** The language it goes into, as DeepL codes it. */
  target: string;
  translation: string;
  detected: string;
  failure: TranslateFailure | ClipboardFailure | null;
  /** « Voir l'original » on the translation. */
  original: boolean;
};

/** Translations kept in memory at most: the oldest go first. */
const KEPT = 60;
const CLIPBOARD_FAILURES: readonly ClipboardFailure[] = ["empty", "tooLong", "busy"];

function reasonOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class TranslateStore {
  /** Whether a DeepL key is in the vault (unknown until asked). */
  hasKey = $state<boolean | undefined>(undefined);
  /** By `key(id)`: the translations asked for during this run. */
  items = $state<Record<string, TranslationState>>({});
  /** Replaced whole (`$state.raw`): `sendClipboard` compares it. */
  clipboard = $state.raw<ClipboardAsk | null>(null);
  /** Réglages › Chat opened to add a key: its Traduction section comes into view. */
  wantsKey = $state(false);
  #order: string[] = [];

  /** The language texts go into now. */
  target(): string {
    return targetOf(settings.value.translate.target, textsLocale());
  }

  /** A translation belongs to a text and a language: another language asks again. */
  key(id: string): string {
    return `${this.target()}|${id}`;
  }

  state(id: string): TranslationState | undefined {
    return this.items[this.key(id)];
  }

  /** What to show for `id`: its translation while shown, else `original`. */
  shown(id: string, original: string): string {
    const state = this.state(id);
    return state?.status === "done" && !state.original ? state.text : original;
  }

  async refreshKey(): Promise<boolean> {
    this.hasKey = await secretHas(DEEPL_SECRET);
    return this.hasKey;
  }

  /** « Enregistrer la clé »: straight to the vault. Returns why it was refused, or null. */
  async saveKey(key: string): Promise<string | null> {
    try {
      await secretStore(DEEPL_SECRET, key);
      this.hasKey = true;
      for (const [id, state] of Object.entries(this.items)) if (state.status === "offer") delete this.items[id];
      return null;
    } catch (error) {
      return reasonOf(error);
    }
  }

  /** « Effacer »: the key leaves the vault. */
  async removeKey(): Promise<string | null> {
    try {
      await secretRemove(DEEPL_SECRET);
      this.hasKey = false;
      return null;
    } catch (error) {
      return reasonOf(error);
    }
  }

  /** The button's click: offers the key when there is none, toggles a translation already
   * there, else asks the core (again, after a refusal). A hidden text never leaves. */
  async press(id: string, text: string, hidden = false): Promise<void> {
    if (!translatable(text, hidden)) return;
    const key = this.key(id);
    const known = this.items[key];
    if (known?.status === "pending") return;
    if (known?.status === "offer") {
      this.addKey();
      return;
    }
    if (known?.status === "done") {
      this.items[key] = { ...known, original: !known.original };
      return;
    }
    const hasKey = this.hasKey ?? (await this.refreshKey());
    if (!hasKey) {
      this.#set(key, { status: "offer" });
      return;
    }
    const target = this.target();
    this.#set(key, { status: "pending" });
    try {
      const translation = await translateText(text.trim(), target);
      this.#set(key, { status: "done", text: translation.text, detected: translation.detected, original: false });
    } catch (error) {
      const failure = failureOf(error);
      if (failure === "noKey") this.hasKey = false;
      this.#set(key, failure === "noKey" ? { status: "offer" } : { status: "error", failure });
    }
  }

  /** Réglages › Chat, where the key goes; its Traduction section comes into view. */
  addKey(): void {
    this.wantsKey = true;
    island.openPanel("chat");
  }

  #set(key: string, state: TranslationState): void {
    this.items[key] = state;
    this.#order = [...this.#order.filter((k) => k !== key), key];
    while (this.#order.length > KEPT) {
      const oldest = this.#order.shift();
      if (oldest) delete this.items[oldest];
    }
  }

  /** « Traduire le presse-papiers » (shortcut, « OK Mochi »). Without a key, the island
   * offers to add one and the clipboard stays unread. Else the core reads it now, and the
   * island shows exactly what would leave, waiting for « Traduire ». */
  async startClipboard(): Promise<void> {
    const ask: ClipboardAsk = { status: "confirm", text: "", target: this.target(), translation: "", detected: "", failure: null, original: false };
    if (!(await this.refreshKey())) {
      this.#showClipboard({ ...ask, status: "noKey" });
      return;
    }
    try {
      const text = await readClipboardText();
      if (text.trim().length > MAX_TRANSLATE_CHARS) this.#showClipboard({ ...ask, status: "unreadable", failure: "tooLong" });
      else if (!translatable(text)) this.#showClipboard({ ...ask, status: "unreadable", failure: "empty" });
      else this.#showClipboard({ ...ask, text });
    } catch (error) {
      const failure = CLIPBOARD_FAILURES.find((code) => code === error) ?? "empty";
      this.#showClipboard({ ...ask, status: "unreadable", failure });
    }
  }

  /** The user's click on « Traduire »: the text shown goes to DeepL, the translation comes
   * back on the island. */
  async sendClipboard(): Promise<void> {
    const asked = this.clipboard;
    if (asked?.status !== "confirm") return;
    const sending: ClipboardAsk = { ...asked, status: "sending" };
    this.clipboard = sending;
    banners.hold("reply", true);
    let next: ClipboardAsk;
    try {
      const translation = await translateText(asked.text.trim(), asked.target);
      next = { ...asked, status: "done", translation: translation.text, detected: translation.detected };
    } catch (error) {
      const failure = failureOf(error);
      if (failure === "noKey") this.hasKey = false;
      next = failure === "noKey" ? { ...asked, status: "noKey", text: "" } : { ...asked, status: "error", failure };
    } finally {
      banners.hold("reply", false);
    }
    // Closed meanwhile: the translation is dropped.
    if (this.clipboard !== sending) return;
    this.#showClipboard(next);
  }

  /** « Réessayer » after a refusal: a new click, the same text. */
  async retryClipboard(): Promise<void> {
    const ask = this.clipboard;
    if (ask?.status !== "error") return;
    this.clipboard = { ...ask, status: "confirm", failure: null };
    await this.sendClipboard();
  }

  toggleClipboardOriginal(): void {
    const ask = this.clipboard;
    if (ask?.status === "done") this.clipboard = { ...ask, original: !ask.original };
  }

  /** « Copier »: the translation goes back to the clipboard. */
  async copyClipboard(): Promise<void> {
    const ask = this.clipboard;
    if (ask?.status === "done") await copyText(ask.translation);
  }

  /** « Annuler », « Fermer », Échap: the opening closes and the text is forgotten. */
  closeClipboard(): void {
    this.forgetClipboard();
    if (banners.current?.kind === "translation") banners.queue.dismissCurrent();
  }

  /** The clipboard's text and translation leave memory (the opening went away). */
  forgetClipboard(): void {
    this.clipboard = null;
  }

  /** The opening again, with its full time, and the keyboard on it: the shortcut asked. */
  #showClipboard(ask: ClipboardAsk): void {
    this.clipboard = ask;
    if (banners.current?.kind === "translation") banners.queue.dismissCurrent();
    banners.queue.push({ kind: "translation" });
    void focusIsland();
  }
}

export const translations = new TranslateStore();
