// « OK Mochi » (prompt 12, lot 5): the phrases Mochi understands after the keyword, what
// each one does, and the few counters a question to Claude may carry. Windows recognizes
// these phrases on the PC (a command grammar, src-tauri/src/voice/listen.rs): there is no
// free dictation, so Mochi understands this list and nothing else. The phrases are those
// of the interface's language (i18n/fr/voice.ts), and the core listens and speaks in it.

import { type AgentSession, sessionTool } from "./agents";
import type { Catalog } from "./i18n/fr";
import { texts, textsLocale } from "./i18n/live.svelte";
import type { VoiceGrammar } from "./ipc";
import type { ActionId } from "./shortcuts";

/** What a phrase does: an action of the island (shortcuts' list), a question for Claude
 * (sent only after a click), the week's report, or « stop listening ». */
export type VoiceCommand =
  | { kind: "action"; action: ActionId; phrases: string[]; reply: string }
  | { kind: "ask"; question: string; phrases: string[] }
  | { kind: "report"; phrases: string[]; reply: string }
  | { kind: "stop"; phrases: string[]; reply: string };

type Said = keyof Catalog["voice"]["commands"];

/** A command's phrases and Mochi's answer, in the language of the moment. */
function said<C extends { kind: "action"; action: ActionId } | { kind: "report" } | { kind: "stop" }>(command: C, key: Said): C & { phrases: string[]; reply: string } {
  return {
    ...command,
    get phrases() {
      return texts().voice.commands[key].phrases;
    },
    get reply() {
      return texts().voice.commands[key].reply;
    },
  };
}

export const VOICE_COMMANDS: readonly VoiceCommand[] = [
  said({ kind: "action", action: "toggle-island" }, "toggleIsland"),
  said({ kind: "action", action: "go-to-alert" }, "goToAlert"),
  said({ kind: "action", action: "open-chat" }, "openChat"),
  said({ kind: "action", action: "go-to-terminal" }, "goToTerminal"),
  said({ kind: "action", action: "next-pastille" }, "nextPastille"),
  said({ kind: "action", action: "mute" }, "mute"),
  said({ kind: "action", action: "mochi-to-desktop" }, "mochiToDesktop"),
  said({ kind: "action", action: "open-wardrobe" }, "openWardrobe"),
  said({ kind: "action", action: "translate-clipboard" }, "translateClipboard"),
  {
    kind: "ask",
    get question() {
      return texts().voice.summary.question;
    },
    get phrases() {
      return texts().voice.summary.phrases;
    },
  },
  said({ kind: "report" }, "report"),
  said({ kind: "stop" }, "stop"),
];

/** One spelling: lower case, straight apostrophes, single spaces. */
export function normalize(phrase: string): string {
  return phrase.toLocaleLowerCase(textsLocale()).replaceAll("’", "'").split(/\s+/).filter(Boolean).join(" ");
}

/** Commands Mochi can carry out now: an action without its real handler stays out (the
 * recognizer then never listens for its phrases). */
export function usableCommands(ready: readonly ActionId[]): VoiceCommand[] {
  return VOICE_COMMANDS.filter((c) => c.kind !== "action" || ready.includes(c.action));
}

export function commandFor(phrase: string, ready: readonly ActionId[]): VoiceCommand | null {
  const said = normalize(phrase);
  return usableCommands(ready).find((c) => c.phrases.some((p) => normalize(p) === said)) ?? null;
}

/** « OK Mochi », and « OK » with Mochi's own name when the user renamed him and the name
 * is plain words. */
export function wakeWords(name: string): string[] {
  const own = name.trim();
  const plain = /^[\p{L}][\p{L}' -]*$/u.test(own) && own.length >= 2;
  return plain && normalize(own) !== "mochi" ? ["OK Mochi", `OK ${own}`] : ["OK Mochi"];
}

/** What the core listens for, and the language it hears and speaks in. */
export function grammarFor(name: string, ready: readonly ActionId[]): VoiceGrammar {
  return { wake: wakeWords(name), phrases: usableCommands(ready).flatMap((c) => c.phrases), lang: textsLocale() };
}

export type Today = { finished: number; failures: number; decisions: number };

/** What a question to Claude carries: each agent, its project and its state, and today's
 * counters. Never a prompt, a command, a file or a message. */
export function activityContext(sessions: readonly AgentSession[], today: Today): string {
  const said = texts().voice;
  const agents = sessions.length
    ? sessions.map((s) => said.agentLine(sessionTool(s), said.status[s.status], s.project)).join(said.agentsSeparator)
    : said.noAgent;
  const { finished, failures, decisions } = today;
  return [said.agents(agents), said.today(finished, failures, decisions)].join("\n");
}
