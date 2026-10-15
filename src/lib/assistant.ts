// Mochi's request bar: a few local commands recognized by simple keywords. No AI model,
// no network (prompts/07-phase-7-assistant.md).

import { AI, type AgentSession, type AiSource, isLive } from "./agents";
import type { Catalog } from "./i18n/fr";
import { texts } from "./i18n/live.svelte";
import type { Finished } from "./island/banners";

export type Command =
  | { kind: "music"; action: "play" | "pause" }
  | { kind: "summary" }
  | { kind: "focus"; minutes: number }
  | { kind: "stop-focus" }
  | { kind: "dnd" }
  | { kind: "unknown" };

export const FOCUS_DEFAULT_MINUTES = 25;
const FOCUS_MAX_MINUTES = 180;

/** Lower case, no accents, single spaces: "Mets  la LO-FI" → "mets la lo-fi". */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

export function parseCommand(text: string): Command {
  const t = normalize(text);
  if (/\b(arrete|stop|fin)\b.*\b(focus|pomodoro)\b/.test(t)) return { kind: "stop-focus" };
  if (/\b(focus|pomodoro|concentr)/.test(t)) {
    const minutes = Number(t.match(/\b(\d{1,3})\b/)?.[1] ?? FOCUS_DEFAULT_MINUTES);
    return { kind: "focus", minutes: Math.min(FOCUS_MAX_MINUTES, Math.max(1, minutes)) };
  }
  if (/\bne pas deranger\b|\bdnd\b|\bsilence\b/.test(t)) return { kind: "dnd" };
  if (/\b(pause|stop|coupe|arrete)\b.*\b(musique|son|lecture|lo-?fi)\b|\b(musique|lecture) en pause\b/.test(t)) {
    return { kind: "music", action: "pause" };
  }
  if (/\blo-?fi\b|\bmusique\b|\bjoue\b|\breprends\b/.test(t)) return { kind: "music", action: "play" };
  if (/\bresum/.test(t)) return { kind: "summary" };
  return { kind: "unknown" };
}

/** A suggestion: its label in the language of the moment, and the request it makes. */
function suggestion(key: keyof Catalog["mochi"]["assistant"]["suggestions"], text: string): { label: string; text: string } {
  return {
    get label() {
      return texts().mochi.assistant.suggestions[key];
    },
    text,
  };
}

export const SUGGESTIONS: readonly { label: string; text: string }[] = [
  suggestion("lofi", "lo-fi"),
  suggestion("summary", "résume"),
  suggestion("focus", "focus 25"),
  suggestion("quiet", "ne pas déranger"),
];

export const HELP =
  "Je ne sais pas encore faire ça. Je sais : lancer ou mettre en pause la musique, résumer le travail des agents, lancer un focus de 25 min et passer en ne pas déranger.";

/** What happened today, for the summary (memory only). */
export type DayLog = { finished: (Finished & { at: number })[]; failures: number };

/** One or two sentences built from today's events. */
export function summarize(log: DayLog, sessions: readonly AgentSession[]): string {
  const parts: string[] = [];
  const said = texts().mochi.assistant;
  if (log.finished.length === 0 && log.failures === 0) {
    parts.push(said.noTask);
  } else {
    const bySource = new Map<AiSource, number>();
    for (const f of log.finished) bySource.set(f.source, (bySource.get(f.source) ?? 0) + 1);
    const who = [...bySource].map(([source, n]) => `${AI[source].name} ${n}`).join(", ");
    const added = log.finished.reduce((sum, f) => sum + f.added, 0);
    const removed = log.finished.reduce((sum, f) => sum + f.removed, 0);
    const minutes = Math.round(log.finished.reduce((sum, f) => sum + f.durationMs, 0) / 60_000);
    let line = said.today(log.finished.length, who);
    if (log.failures > 0) line += said.failures(log.failures);
    line += said.lines(added, removed, minutes);
    parts.push(line);
  }
  const live = sessions.filter(isLive);
  if (live.length > 0) {
    parts.push(said.live(live.map((s) => said.on(AI[s.source].name, s.project)).join(", ")));
  }
  return parts.join(" ");
}

/** Time left in a focus session: "18 min", or "45 s" in the last minute. */
export function remainingLabel(ms: number): string {
  const duration = texts().timeline.duration;
  return ms >= 60_000 ? duration.minutes(Math.ceil(ms / 60_000)) : duration.seconds(Math.ceil(Math.max(0, ms) / 1000));
}
