import { describe, expect, it } from "vitest";
import {
  type ChatMessage,
  MAX_QUESTION,
  PROVIDERS,
  blocker,
  destination,
  fileNote,
  isProvider,
  modelText,
  providerInfo,
  readyToSend,
  requestId,
  turnsOf,
  withDictation,
} from "./chat";

describe("providers", () => {
  it("three remote ones with a key in the vault, two local ones without", () => {
    expect(PROVIDERS.map((p) => p.id)).toEqual(["anthropic", "gemini", "openai", "ollama", "lmstudio"]);
    for (const p of PROVIDERS.filter((x) => !x.local)) expect(p.secret).toBe(`chat:${p.id}`);
    for (const p of PROVIDERS.filter((x) => x.local)) expect(p.secret).toBeNull();
    expect(providerInfo("lmstudio").label).toBe("LM Studio");
    expect(isProvider("ollama")).toBe(true);
    expect(isProvider("evil")).toBe(false);
  });

  it("says where a question goes", () => {
    expect(destination(providerInfo("anthropic"))).toBe("Ta question part chez Anthropic, avec ta clé. Rien n'est gardé après fermeture.");
    expect(destination(providerInfo("ollama"))).toBe("Ollama répond sur ce PC, par localhost. Rien ne sort.");
  });

  it("a remote provider without its key cannot start; a local one always can", () => {
    expect(blocker(providerInfo("gemini"), false)).toEqual({ text: "Ajoute ta clé Gemini pour discuter.", fix: "key" });
    expect(blocker(providerInfo("gemini"), true)).toBeNull();
    expect(blocker(providerInfo("gemini"), undefined)).toBeNull();
    expect(blocker(providerInfo("ollama"), false)).toBeNull();
  });
});

describe("a conversation", () => {
  const messages: ChatMessage[] = [
    { id: "u1", role: "user", text: "Bonjour" },
    { id: "r1", role: "assistant", text: "Salut !", by: "Claude" },
    { id: "u2", role: "user", text: "Et ça ?", file: "notes.md" },
    { id: "r2", role: "assistant", text: "", by: "Claude", error: "Clé refusée par Claude." },
    { id: "u3", role: "user", text: "Encore" },
    { id: "r3", role: "assistant", text: "  ", by: "Claude", pending: true },
  ];

  it("sends the questions and the answers that got some text, never a failed one", () => {
    expect(turnsOf(messages)).toEqual([
      { role: "user", text: "Bonjour" },
      { role: "assistant", text: "Salut !" },
      { role: "user", text: "Et ça ?" },
      { role: "user", text: "Encore" },
    ]);
  });

  it("a question needs text, or something joined, and stays under the core's limit", () => {
    expect(readyToSend("  ", false)).toBe(false);
    expect(readyToSend("  ", true)).toBe(true);
    expect(readyToSend("Salut", false)).toBe(true);
    expect(readyToSend("x".repeat(MAX_QUESTION + 1), true)).toBe(false);
  });

  it("request ids are what the core accepts: letters, digits and dashes", () => {
    const id = requestId(1_728_000_000_000, 0.123456);
    expect(id).toMatch(/^r-[a-z0-9]+-[a-z0-9]+$/);
    expect(id.length).toBeLessThanOrEqual(64);
    expect(requestId(1, 0.5)).not.toBe(requestId(1, 0.6));
  });

  it("names the model, or invites to pick one", () => {
    expect(modelText("gpt-5")).toBe("gpt-5");
    expect(modelText("")).toBe("Choisir un modèle");
  });

  it("says what Mochi does with a dropped file before it is read", () => {
    expect(fileNote({ id: "f1", name: "a.png", kind: "image" })).toMatch(/Image lue à l'envoi/);
    expect(fileNote({ id: "f1", name: "a.md", kind: "text" })).toMatch(/200 Ko/);
    expect(fileNote({ id: "f1", name: "a.pdf", kind: "other" })).toMatch(/ne lit que le texte et les images/);
  });
});

describe("dictation", () => {
  it("puts the words heard after what is typed, as a sentence when it starts one", () => {
    expect(withDictation("", "explique-moi cette erreur")).toBe("Explique-moi cette erreur");
    expect(withDictation("Bonjour.", "comment vas-tu")).toBe("Bonjour. Comment vas-tu");
    expect(withDictation("Peux-tu", "traduire ceci")).toBe("Peux-tu traduire ceci");
    expect(withDictation("Texte", "   ")).toBe("Texte");
  });
});
