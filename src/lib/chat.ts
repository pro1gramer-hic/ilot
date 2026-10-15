// The chat in the island (prompt 12, lot 3): its five providers and the rules of a
// conversation, as pure functions. The questions leave from the Rust core (src-tauri/src/chat),
// never from this page; keys stay in the Windows Credential Manager (src-tauri/src/vault.rs).

import { texts, textsLocale } from "./i18n/live.svelte";
import type { Attachment, ChatProvider, ChatTurn } from "./ipc";

export type ProviderInfo = {
  id: ChatProvider;
  /** Its name in the island. */
  label: string;
  /** Who answers: a company with the user's key, or this PC. */
  maker: string;
  local: boolean;
  /** The vault entry of its key (none for the local ones). */
  secret: string | null;
  /** What a key looks like, as a placeholder. */
  keyHint: string;
  /** Where the user creates a key. */
  keyPage: string | null;
};

/** A provider that runs on this PC: no key, nothing leaves. */
function onThisPc(id: ChatProvider, label: string): ProviderInfo {
  return {
    id,
    label,
    get maker() {
      return texts().common.chat.thisPc;
    },
    local: true,
    secret: null,
    keyHint: "",
    keyPage: null,
  };
}

export const PROVIDERS: readonly ProviderInfo[] = [
  { id: "anthropic", label: "Claude", maker: "Anthropic", local: false, secret: "chat:anthropic", keyHint: "sk-ant-…", keyPage: "console.anthropic.com" },
  { id: "gemini", label: "Gemini", maker: "Google", local: false, secret: "chat:gemini", keyHint: "AIza…", keyPage: "aistudio.google.com" },
  { id: "openai", label: "OpenAI", maker: "OpenAI", local: false, secret: "chat:openai", keyHint: "sk-…", keyPage: "platform.openai.com" },
  onThisPc("ollama", "Ollama"),
  onThisPc("lmstudio", "LM Studio"),
];

export function providerInfo(id: ChatProvider): ProviderInfo {
  return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[0];
}

export function isProvider(value: unknown): value is ChatProvider {
  return PROVIDERS.some((p) => p.id === value);
}

/** One line of the conversation, kept in memory only. */
export type ChatMessage = {
  /** The user's lines: `u…`; an answer: the id of its request. */
  id: string;
  role: "user" | "assistant";
  text: string;
  /** Who answered (« Claude », « Ollama »), on an answer. */
  by?: string;
  /** Joined to a question: the file's name, the window's title. */
  file?: string;
  window?: string;
  /** Why an answer stopped. */
  error?: string;
  /** Still streaming. */
  pending?: boolean;
  /** Stopped by the user. */
  stopped?: boolean;
};

/** The longest question (the core refuses more). */
export const MAX_QUESTION = 32_000;

/** What the provider is sent: the questions and the answers that got some text. */
export function turnsOf(messages: readonly ChatMessage[]): ChatTurn[] {
  return messages
    .filter((m) => m.role === "user" || (m.text.trim() !== "" && !m.error))
    .map((m) => ({ role: m.role, text: m.text }));
}

/** A request id the core accepts: letters, digits and dashes. */
export function requestId(now: number, random: number): string {
  return `r-${Math.floor(now).toString(36)}-${Math.floor(random * 1e9).toString(36)}`;
}

/** A question can leave: some text, or something joined to ask about. */
export function readyToSend(text: string, joined: boolean): boolean {
  const length = text.trim().length;
  return length <= MAX_QUESTION && (length > 0 || joined);
}

/** Where the question goes, said in the empty chat and in Réglages. */
export function destination(info: ProviderInfo): string {
  return info.local ? texts().common.chat.local(info.label) : texts().common.chat.remote(info.maker);
}

/** The chat cannot start yet: why, in one sentence, and what fixes it. */
export function blocker(info: ProviderInfo, hasKey: boolean | undefined): { text: string; fix: "key" | null } | null {
  if (info.local || hasKey !== false) return null;
  return { text: texts().common.chat.addKey(info.label), fix: "key" };
}

/** What Mochi can do with a dropped file, said before it is sent. */
export function fileNote(attachment: Attachment): string {
  const said = texts().common.chat;
  switch (attachment.kind) {
    case "image":
      return said.imageNote;
    case "text":
      return said.textNote;
    default:
      return said.otherNote;
  }
}

/** The model button: the model's id, or an invitation to pick one. */
export function modelText(model: string): string {
  return model || texts().common.chat.pickModel;
}

/** Text heard by the dictation, put after what is typed. */
export function withDictation(typed: string, heard: string): string {
  const said = heard.trim();
  if (!said) return typed;
  const sentence = said.charAt(0).toLocaleUpperCase(textsLocale()) + said.slice(1);
  if (!typed.trim()) return sentence;
  return `${typed.replace(/\s+$/, "")} ${typed.trim().match(/[.!?…]$/) ? sentence : said}`;
}
