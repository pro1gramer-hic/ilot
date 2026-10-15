// The browser preview of the chat, the vault, dropped files, e-mail and window contexts
// (prompt 12, lot 3), and of DeepL and the clipboard: a stand-in for the Rust core, with
// fake data only. It never reaches a provider, DeepL nor a mail app; answers are typed
// here, a few words at a time. Every command it hears is logged (`calls`), so that the
// interface tests can check what was asked, and when.

import { fileName } from "./island/banners";
import { type Attachment, type ChatProvider, type ContextPreview, emitPreview, setPreviewFeatures } from "./ipc";

export const DEMO_MODELS: Readonly<Record<ChatProvider, readonly string[]>> = {
  anthropic: ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"],
  gemini: ["gemini-2.5-pro", "gemini-2.5-flash"],
  openai: ["gpt-5", "gpt-5-mini", "gpt-4.1"],
  ollama: ["llama3.2:3b", "qwen2.5-coder:7b", "gemma3:4b"],
  lmstudio: ["google/gemma-3-12b"],
};

export const DEMO_QUESTION = "Résume ce compte rendu en trois points";

export const DEMO_ANSWER = [
  "Voici l'essentiel du compte rendu :",
  "",
  "1. **Lancement reporté** au 4 novembre, le temps de finir les tests de paiement.",
  "2. Léa reprend la page d'accueil ; la maquette est attendue **jeudi**.",
  "3. Le budget publicité baisse de 15 %, compensé par l'envoi de `newsletter-v2`.",
  "",
  "Prochaine réunion : mardi à 10 h.",
].join("\n");

/** The fake window of the preview, and the text « Lire la fenêtre » brings back. */
export const DEMO_WINDOW = {
  title: "Compte rendu réunion.txt - Bloc-notes",
  app: "notepad",
  text: [
    "Réunion produit, 9 octobre",
    "Présents : Léa, Karim, Inès",
    "- Lancement reporté au 4 novembre (tests de paiement à finir)",
    "- Léa reprend la page d'accueil, maquette jeudi",
    "- Budget publicité : -15 %, compensé par la newsletter v2",
    "Prochaine réunion : mardi 10 h",
  ].join("\n"),
} as const;

const SECRETS = { anthropic: "chat:anthropic", gemini: "chat:gemini", openai: "chat:openai" } as const;

/** The demo answer in English, for « Traduire » under it. */
const DEMO_ANSWER_EN = [
  "Here is the gist of the meeting notes:",
  "",
  "1. **Launch moved** to November 4, to finish the payment tests.",
  "2. Léa takes over the home page; the mockup is due **Thursday**.",
  "3. The ad budget drops by 15%, offset by sending `newsletter-v2`.",
  "",
  "Next meeting: Tuesday at 10 am.",
].join("\n");

/** What the fake clipboard holds: a note in English, to translate into French. */
export const DEMO_CLIPBOARD =
  "Quick update: the release moves to Thursday. Could you review the onboarding copy before noon and flag anything that reads oddly?";

/** The same note in French, for the English interface. */
export const DEMO_CLIPBOARD_FR =
  "Petit point : la sortie passe à jeudi. Tu peux relire les textes d'accueil avant midi et signaler ce qui sonne bizarre ?";

/** What the fake DeepL answers: by text, the translation into French or English and the
 * language it « detected ». Any other text comes back as it was. */
const DEMO_TRANSLATIONS: Readonly<Record<string, { detected: string; FR?: string; EN?: string }>> = {
  [DEMO_CLIPBOARD]: {
    detected: "EN",
    FR: "Petit point : la sortie est décalée à jeudi. Tu peux relire les textes d'accueil avant midi et signaler tout ce qui sonne bizarre ?",
  },
  [DEMO_CLIPBOARD_FR]: {
    detected: "FR",
    EN: "Quick update: the release moves to Thursday. Can you review the onboarding copy before noon and flag anything that sounds odd?",
  },
  "Pushed the new map, can you test the parser tonight? It still crashes on empty tiles.": {
    detected: "EN",
    FR: "J'ai poussé la nouvelle carte, tu peux tester le parseur ce soir ? Il plante encore sur les tuiles vides.",
  },
  [DEMO_ANSWER]: { detected: "FR", EN: DEMO_ANSWER_EN },
  "Tu rentres à quelle heure ?": { detected: "FR", EN: "What time are you coming home?" },
  "On se voit à 18h ? J'ai pris les places pour le concert.": {
    detected: "FR",
    EN: "See you at 6pm? I got the tickets for the concert.",
  },
  "Did you see the new build? The springs feel so much better now.": {
    detected: "EN",
    FR: "Tu as vu la nouvelle version ? Les ressorts sont bien plus agréables maintenant.",
  },
};

/** What the fake core does: keys saved, whether answers stream or wait. */
export type FakeOptions = { keys: readonly string[]; hold: boolean };

export class FakeFeatures {
  /** Every command heard, in order (with the request's attachment and context ids). */
  readonly calls: { command: string; args: Record<string, unknown> }[] = [];
  readonly keys: Set<string>;
  /** Answers wait (Mochi keeps thinking) instead of streaming. */
  hold: boolean;
  /** The fake clipboard's text (null: no text in it). */
  clipboard: string | null = DEMO_CLIPBOARD;
  #timers: ReturnType<typeof setTimeout>[] = [];
  #attached = 0;

  constructor({ keys, hold }: FakeOptions) {
    this.keys = new Set(keys);
    this.hold = hold;
  }

  stop(): void {
    for (const timer of this.#timers) clearTimeout(timer);
    this.#timers = [];
  }

  #later(ms: number, act: () => void): void {
    this.#timers.push(setTimeout(act, ms));
  }

  /** Types the demo answer a few words at a time, then ends it. */
  #stream(requestId: string): void {
    const words = DEMO_ANSWER.split(/(?<= )/);
    let at = 0;
    for (let i = 0; i < words.length; i += 3) {
      at += 70;
      const text = words.slice(i, i + 3).join("");
      this.#later(at, () => emitPreview("chat:delta", { requestId, text }));
    }
    this.#later(at + 60, () => emitPreview("chat:done", { requestId }));
  }

  answer(command: string, args: Record<string, unknown>): unknown {
    switch (command) {
      case "secret_store":
        this.calls.push({ command, args: { name: args.name } });
        this.keys.add(String(args.name));
        return Promise.resolve();
      case "secret_has":
        return Promise.resolve(this.keys.has(String(args.name)));
      case "secret_remove":
        this.calls.push({ command, args });
        this.keys.delete(String(args.name));
        return Promise.resolve();
      case "chat_models": {
        const provider = args.provider as ChatProvider;
        const secret = provider in SECRETS ? SECRETS[provider as keyof typeof SECRETS] : null;
        if (secret && !this.keys.has(secret)) return Promise.reject(new Error("Ajoute ta clé dans Réglages › Chat."));
        return Promise.resolve([...DEMO_MODELS[provider]]);
      }
      case "chat_send": {
        const request = args.request as { requestId: string; attachment?: string; context?: string };
        this.calls.push({ command, args: { attachment: request.attachment, context: request.context } });
        if (!this.hold) this.#stream(request.requestId);
        return Promise.resolve();
      }
      case "chat_cancel":
        this.calls.push({ command, args });
        this.stop();
        return Promise.resolve();
      case "file_attach": {
        this.calls.push({ command, args });
        const name = fileName(String(args.path));
        const kind: Attachment["kind"] = /\.(png|jpe?g|gif|webp)$/i.test(name) ? "image" : /\.(txt|md|csv|json)$/i.test(name) ? "text" : "other";
        this.#attached += 1;
        return Promise.resolve({ id: `f${this.#attached}`, name, kind } satisfies Attachment);
      }
      case "mail_app":
        return Promise.resolve({ name: "Microsoft Outlook", mapi: true });
      case "mail_send":
        this.calls.push({ command, args });
        return Promise.resolve({ way: "mapi", client: "Microsoft Outlook", note: null });
      case "context_read": {
        this.calls.push({ command, args });
        const preview: ContextPreview = { id: String(args.id), title: DEMO_WINDOW.title, app: DEMO_WINDOW.app, text: DEMO_WINDOW.text, truncated: false };
        return new Promise((resolve) => this.#later(260, () => resolve(preview)));
      }
      case "context_confirm":
      case "context_forget":
        this.calls.push({ command, args });
        return Promise.resolve();
      case "translate_text": {
        // Like the core: no key, nothing goes (and nothing is logged as sent).
        if (!this.keys.has("service:deepl")) return Promise.reject("noKey");
        this.calls.push({ command, args: { text: args.text, target: args.target } });
        const text = String(args.text);
        const known = DEMO_TRANSLATIONS[text];
        const into = String(args.target).startsWith("EN") ? "EN" : "FR";
        const translation = { text: known?.[into] ?? text, detected: known?.detected ?? into };
        return new Promise((resolve) => this.#later(320, () => resolve(translation)));
      }
      case "read_clipboard_text":
        this.calls.push({ command, args });
        return this.clipboard === null ? Promise.reject("empty") : Promise.resolve(this.clipboard);
      case "dictation_available":
        return Promise.resolve(true);
      case "dictation_start":
        this.calls.push({ command, args });
        this.#later(500, () => emitPreview("dictation:text", { text: "explique moi", final: false }));
        this.#later(1100, () => emitPreview("dictation:text", { text: "explique-moi cette erreur", final: true }));
        this.#later(1300, () => emitPreview("dictation:end", { error: null }));
        return Promise.resolve();
      case "dictation_stop":
        this.calls.push({ command, args });
        this.stop();
        emitPreview("dictation:end", { error: null });
        return Promise.resolve();
      default:
        return undefined;
    }
  }
}

/** Plugs a fake core for these features into the preview's IPC. */
export function startFakeFeatures(options: FakeOptions): FakeFeatures {
  const fake = new FakeFeatures(options);
  setPreviewFeatures((command, args) => fake.answer(command, args));
  return fake;
}

/** A fake Notepad window on the preview's desktop: the window Mochi « landed on ». */
export function showFakeWindow(on: boolean): void {
  document.querySelector("[data-fake-window]")?.remove();
  if (!on) return;
  const window = document.createElement("div");
  window.dataset.fakeWindow = "";
  window.setAttribute("aria-hidden", "true");
  Object.assign(window.style, {
    position: "fixed",
    left: "40px",
    top: "500px",
    width: "380px",
    height: "190px",
    borderRadius: "8px",
    background: "#f7f7f9",
    color: "#1f1f24",
    boxShadow: "0 18px 40px rgba(0,0,0,.45)",
    font: "13px/1.45 Consolas, monospace",
    overflow: "hidden",
    zIndex: "-1",
  });
  const bar = document.createElement("div");
  bar.textContent = DEMO_WINDOW.title;
  Object.assign(bar.style, { padding: "6px 12px", background: "#e9e9ee", font: "12px Inter, sans-serif", color: "#3a3a44" });
  const body = document.createElement("pre");
  body.textContent = DEMO_WINDOW.text;
  Object.assign(body.style, { padding: "10px 12px", margin: "0", whiteSpace: "pre-wrap" });
  window.append(bar, body);
  document.body.append(window);
}
