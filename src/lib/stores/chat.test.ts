import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ChatRequest } from "../ipc";

// The chat store talks to the core through ipc.ts and opens the hub through the island:
// both replaced here, so that every request the interface makes is seen, and when.
const core = vi.hoisted(() => ({
  calls: [] as [string, unknown][],
  listeners: new Map<string, (payload: unknown) => void>(),
  keys: new Set<string>(),
  refuseSend: null as string | null,
  hubs: [] as unknown[],
}));

vi.mock("../ipc", () => {
  const log =
    (name: string) =>
    async (...args: unknown[]) => {
      core.calls.push([name, args.length > 1 ? args : args[0]]);
    };
  return {
    onCoreEvent: (name: string, handler: (payload: unknown) => void) => {
      core.listeners.set(name, handler);
      return () => {};
    },
    chatSend: async (request: ChatRequest) => {
      core.calls.push(["chat_send", request]);
      if (core.refuseSend) throw core.refuseSend;
    },
    chatCancel: log("chat_cancel"),
    chatModels: async (provider: string) => {
      core.calls.push(["chat_models", provider]);
      return provider === "ollama" ? ["llama3.2:3b", "gemma3:4b"] : ["claude-opus-5"];
    },
    secretStore: async (name: string, secret: string) => {
      core.calls.push(["secret_store", name]);
      if (!secret.trim()) throw "La clé est vide";
      core.keys.add(name);
    },
    secretHas: async (name: string) => core.keys.has(name),
    secretRemove: async (name: string) => {
      core.calls.push(["secret_remove", name]);
      core.keys.delete(name);
    },
    contextRead: async (id: string) => {
      core.calls.push(["context_read", id]);
      return { id, title: "Rapport.docx - Word", app: "WINWORD", text: "Chiffres du trimestre", truncated: false };
    },
    contextConfirm: log("context_confirm"),
    contextForget: log("context_forget"),
    dictationAvailable: async () => true,
    dictationStart: log("dictation_start"),
    dictationStop: log("dictation_stop"),
  };
});

vi.mock("../island/controller.svelte", () => ({
  island: { openHub: (options: unknown) => core.hubs.push(options) },
}));

vi.mock("./settings.svelte", () => ({
  settings: { value: { chat: { provider: "anthropic", model: "claude-sonnet-5" } } },
}));

const { ChatStore } = await import("./chat.svelte");

const emit = (name: string, payload: unknown): void => core.listeners.get(name)?.(payload);
const sent = (): ChatRequest[] => core.calls.filter(([name]) => name === "chat_send").map(([, r]) => r as ChatRequest);
const named = (name: string): unknown[] => core.calls.filter(([n]) => n === name).map(([, a]) => a);

let chat: InstanceType<typeof ChatStore>;

beforeEach(() => {
  core.calls = [];
  core.listeners.clear();
  core.keys = new Set(["chat:anthropic"]);
  core.refuseSend = null;
  core.hubs = [];
  chat = new ChatStore();
  chat.connect();
});

describe("opening the chat", () => {
  it("opens the hub on the default of Réglages, or on the target given (lot 4: an Ollama pill)", () => {
    chat.open();
    expect(core.hubs).toEqual([{ view: "chat" }]);
    expect([chat.provider, chat.model]).toEqual(["anthropic", "claude-sonnet-5"]);
    chat.open({ provider: "ollama", model: "llama3.2:3b" });
    expect([chat.provider, chat.model]).toEqual(["ollama", "llama3.2:3b"]);
    chat.open();
    expect(chat.provider).toBe("ollama");
  });
});

describe("a question", () => {
  it("leaves with the conversation, streams back, and Mochi thinks meanwhile", async () => {
    chat.open();
    chat.draft = "Bonjour";
    await chat.send();
    const [request] = sent();
    expect(request).toMatchObject({ provider: "anthropic", model: "claude-sonnet-5", turns: [{ role: "user", text: "Bonjour" }] });
    expect(request.attachment).toBeUndefined();
    expect(request.context).toBeUndefined();
    expect(chat.draft).toBe("");
    expect(chat.thinking).toBe(true);
    emit("chat:delta", { requestId: request.requestId, text: "Sal" });
    emit("chat:delta", { requestId: request.requestId, text: "ut !" });
    emit("chat:delta", { requestId: "another", text: "rien" });
    expect(chat.messages.at(-1)).toMatchObject({ role: "assistant", text: "Salut !", pending: true, by: "Claude" });
    emit("chat:done", { requestId: request.requestId });
    expect(chat.thinking).toBe(false);
    expect(chat.messages.at(-1)?.pending).toBe(false);

    chat.draft = "Et ensuite ?";
    await chat.send();
    expect(sent()[1].turns).toEqual([
      { role: "user", text: "Bonjour" },
      { role: "assistant", text: "Salut !" },
      { role: "user", text: "Et ensuite ?" },
    ]);
  });

  it("nothing leaves without text or something joined, nor while an answer streams", async () => {
    chat.draft = "   ";
    await chat.send();
    expect(sent()).toEqual([]);
    chat.draft = "Un";
    await chat.send();
    chat.draft = "Deux";
    await chat.send();
    expect(sent().length).toBe(1);
  });

  it("a refusal of the core shows on the answer and Mochi stops thinking", async () => {
    core.refuseSend = "Ajoute ta clé Claude dans Réglages › Chat.";
    chat.draft = "Bonjour";
    await chat.send();
    expect(chat.messages.at(-1)).toMatchObject({ error: "Ajoute ta clé Claude dans Réglages › Chat.", pending: false });
    expect(chat.thinking).toBe(false);
  });

  it("« Arrêter » cancels the request and keeps what came", async () => {
    chat.draft = "Raconte";
    await chat.send();
    const id = sent()[0].requestId;
    emit("chat:delta", { requestId: id, text: "Il était" });
    chat.stop();
    emit("chat:delta", { requestId: id, text: " une fois" });
    expect(named("chat_cancel")).toEqual([id]);
    expect(chat.messages.at(-1)).toMatchObject({ text: "Il était", stopped: true, pending: false });
    expect(chat.thinking).toBe(false);
  });

  it("without a model, the first the provider offers is used", async () => {
    chat.choose("ollama", "");
    chat.draft = "Salut";
    await chat.send();
    expect(sent()[0]).toMatchObject({ provider: "ollama", model: "llama3.2:3b" });
  });
});

describe("a dropped file", () => {
  it("joins the question only when it is sent: nothing reads it before", async () => {
    chat.askAbout({ id: "f1", name: "notes.md", kind: "text" });
    expect(core.hubs).toEqual([{ view: "chat" }]);
    expect(sent()).toEqual([]);
    await chat.send();
    expect(sent()[0].attachment).toBe("f1");
    expect(chat.messages[0]).toMatchObject({ file: "notes.md", text: "" });
    expect(chat.attachment).toBeNull();
  });
});

describe("a window offered as context", () => {
  const proposal = { id: "w1", title: "Rapport.docx - Word", app: "WINWORD", blocked: null };

  it("is read on « Lire », joined on « Joindre », and sent with the next question only", async () => {
    emit("context:proposed", proposal);
    expect(chat.proposal).toEqual(proposal);
    expect(core.hubs).toEqual([{ view: "chat" }]);
    expect(core.calls).toEqual([]);
    await chat.readContext();
    expect(chat.preview?.text).toBe("Chiffres du trimestre");
    expect(named("context_confirm")).toEqual([]);
    chat.draft = "Avant de joindre";
    await chat.send();
    expect(sent()[0].context).toBeUndefined();
    emit("chat:done", { requestId: sent()[0].requestId });
    await chat.joinContext();
    expect(named("context_confirm")).toEqual(["w1"]);
    expect(chat.window).toEqual({ id: "w1", title: "Rapport.docx - Word", app: "WINWORD" });
    chat.draft = "Résume";
    await chat.send();
    expect(sent()[1].context).toBe("w1");
    expect(chat.messages.at(-2)).toMatchObject({ window: "Rapport.docx - Word" });
    expect(chat.window).toBeNull();
    expect(named("context_forget")).toEqual(["w1"]);
  });

  it("« Non merci » forgets it unread; a protected one cannot even be read", async () => {
    emit("context:proposed", proposal);
    chat.dismissContext();
    expect(named("context_forget")).toEqual(["w1"]);
    expect(named("context_read")).toEqual([]);
    emit("context:proposed", { id: "w2", title: "", app: "firefox", blocked: "Banque ou paiement : îlot ne lit pas cette fenêtre." });
    await chat.readContext();
    expect(named("context_read")).toEqual([]);
    expect(chat.preview).toBeNull();
  });
});

describe("keys", () => {
  it("go to the vault and never stay in the store", async () => {
    const key = "sk-ant-api03-SECRET-0123456789";
    core.keys.clear();
    await chat.refreshKey("anthropic");
    expect(chat.keys.anthropic).toBe(false);
    expect(await chat.saveKey("anthropic", key)).toBeNull();
    expect(named("secret_store")).toEqual(["chat:anthropic"]);
    expect(chat.keys.anthropic).toBe(true);
    expect(JSON.stringify({ keys: chat.keys, draft: chat.draft, messages: chat.messages, models: chat.models })).not.toContain("SECRET");
    expect(await chat.saveKey("anthropic", "  ")).toBe("La clé est vide");
    expect(await chat.removeKey("anthropic")).toBeNull();
    expect(chat.keys.anthropic).toBe(false);
    expect(await chat.saveKey("ollama", key)).toBeNull();
    expect(named("secret_store")).toEqual(["chat:anthropic", "chat:anthropic"]);
  });
});

describe("dictation", () => {
  it("guesses show while spoken, the settled phrase goes in the field", async () => {
    chat.draft = "Bonjour.";
    await chat.toggleDictation();
    expect(chat.listening).toBe(true);
    emit("dictation:text", { text: "explique", final: false });
    expect(chat.heard).toBe("explique");
    emit("dictation:text", { text: "explique-moi cette erreur", final: true });
    expect(chat.draft).toBe("Bonjour. Explique-moi cette erreur");
    await chat.toggleDictation();
    expect(named("dictation_stop").length).toBe(1);
    emit("dictation:end", { error: null });
    expect(chat.listening).toBe(false);
  });
});
