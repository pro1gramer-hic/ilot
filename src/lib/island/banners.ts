// The island's temporary openings (docs/09-DEMO-FINALE.md, section 4): an agent
// finished or failed, a message came, the music started or changed track, an action
// answered with a toast. One at a time, in a queue; finished agents shown together merge
// into one. Hovering pauses its time line, and so does anything that covers it (a
// permission, which always goes first, the quick menu, a reply being typed): it then
// resumes with the time it had left.

import { type AgentBadge, type AiSource, sessionName } from "../agents";
import { texts } from "../i18n/live.svelte";
import type { Tab } from "./model";
import type { Attachment } from "../ipc";

/** How long each opening stays (a toast may ask for its own time). */
export const BANNER_MS = {
  finished: 5000,
  failed: 7000,
  message: 7000,
  music: 6000,
  track: 4000,
  file: 9000,
  toast: 2800,
  /** « OK Mochi » waits this long for a click before nothing is sent. */
  voice: 15000,
  /** The clipboard waits this long for « Traduire » (prompt 12, DeepL). */
  translation: 20000,
} as const;

export type Finished = {
  source: AiSource;
  /** Name chosen for a generic tool. */
  name?: string;
  project: string;
  durationMs: number;
  added: number;
  removed: number;
  /** The session, to open its latest change from the banner. */
  sessionId?: string;
  /** The badge its hooks asked for (`ilot_agent`). */
  agent?: AgentBadge;
  /** The file of its latest change, when the turn changed lines. */
  file?: string;
};

export type Failed = { source: AiSource; name?: string; agent?: AgentBadge; project: string; detail: string };

export type ToastTone = "ok" | "err" | "acc";

/** A message refers to the notification shown now (stores/notifications.svelte.ts); the
 * music openings show the track playing now (stores/media.svelte.ts). */
export type Banner =
  | { kind: "finished"; items: Finished[] }
  | { kind: "failed"; failed: Failed }
  | { kind: "message"; id: string }
  | { kind: "music" }
  | { kind: "track" }
  /** A file Mochi swallowed (its path is copied): ask Mochi about it, or mail it. */
  | { kind: "file"; attachment: Attachment }
  /** `tab`: a click on the island opens this tab of the panel (the Monday report). */
  | { kind: "toast"; tone: ToastTone; title: string; sub?: string; ms?: number; tab?: Tab }
  /** « OK Mochi »: the question for Claude waiting for a click, then its answer
   * (stores/voice.svelte.ts). */
  | { kind: "voice" }
  /** « Traduire le presse-papiers »: the copied text waiting for « Traduire », then its
   * translation (stores/translate.svelte.ts). */
  | { kind: "translation" };

/** How long `banner` stays on screen. */
export function durationOf(banner: Banner): number {
  return banner.kind === "toast" && banner.ms ? banner.ms : BANNER_MS[banner.kind];
}

/** Why a banner waits: the cursor rests on it, another shape covers it, or the user is
 * typing a reply to it. */
export type Hold = "hover" | "cover" | "reply";

export class BannerQueue {
  #queue: Banner[] = [];
  #timer: ReturnType<typeof setTimeout> | undefined;
  #deadline = 0;
  #remaining = 0;
  #holds = new Set<Hold>();
  readonly #onChange: (current: Banner | undefined) => void;
  readonly #onDismiss: (banner: Banner) => void;

  /** `onDismiss` hears every banner that leaves the screen (timeout, click). */
  constructor(onChange: (current: Banner | undefined) => void, onDismiss: (banner: Banner) => void = () => {}) {
    this.#onChange = onChange;
    this.#onDismiss = onDismiss;
  }

  get current(): Banner | undefined {
    return this.#queue[0];
  }

  /** True while the time line of the banner on screen stands still. */
  get paused(): boolean {
    return this.#holds.size > 0;
  }

  /** Adds a banner. A finished agent joins a finished banner that waits. */
  push(banner: Banner): void {
    const first = this.#queue[0];
    const pending = banner.kind === "finished" ? this.#queue.find((b) => b.kind === "finished") : undefined;
    if (banner.kind === "finished" && pending?.kind === "finished") pending.items.push(...banner.items);
    else this.#queue.push(banner);
    // A new banner on screen, or one more agent in the banner on screen: full time again.
    if (this.#queue[0] !== first || (pending !== undefined && pending === first)) this.#changed();
  }

  /** Closes the banner on screen (click, timeout). */
  dismissCurrent(): void {
    const banner = this.#queue.shift();
    if (banner) this.#onDismiss(banner);
    this.#changed();
  }

  /** Drops a message banner whose notification went away (read, held for focus…). */
  removeMessage(id: string): void {
    const index = this.#queue.findIndex((b) => b.kind === "message" && b.id === id);
    if (index === -1) return;
    this.#queue.splice(index, 1);
    if (index === 0) this.#changed();
  }

  clear(): void {
    this.#queue = [];
    this.#changed();
  }

  /** Pauses (or resumes) the time line of the banner on screen. */
  hold(reason: Hold, on: boolean): void {
    if (on === this.#holds.has(reason)) return;
    if (on) this.#holds.add(reason);
    else this.#holds.delete(reason);
    this.#arm();
  }

  /** The cursor rests on the banner: it stays; on leaving, it resumes where it was. */
  hover(inside: boolean): void {
    this.hold("hover", inside);
  }

  #changed(): void {
    clearTimeout(this.#timer);
    this.#timer = undefined;
    this.#remaining = this.#queue[0] ? durationOf(this.#queue[0]) : 0;
    this.#arm();
    this.#onChange(this.#queue[0]);
  }

  #arm(): void {
    if (this.#timer !== undefined) {
      clearTimeout(this.#timer);
      this.#timer = undefined;
      this.#remaining = Math.max(0, this.#deadline - Date.now());
    }
    if (!this.#queue[0] || this.paused) return;
    this.#deadline = Date.now() + this.#remaining;
    this.#timer = setTimeout(() => this.dismissCurrent(), this.#remaining);
  }
}

/** "4 min", "moins d'une minute", "1 h 05". */
export function formatDuration(ms: number): string {
  const minutes = Math.floor(ms / 60_000);
  const duration = texts().timeline.duration;
  if (minutes < 1) return duration.lessThanMinute;
  if (minutes < 60) return duration.minutes(minutes);
  return duration.hours(Math.floor(minutes / 60), String(minutes % 60).padStart(2, "0"));
}

/** "Claude a terminé · epilot · 4 min" (the diff is shown apart), or "2 agents ont terminé". */
export function finishedText(items: readonly Finished[]): string {
  if (items.length !== 1) return texts().notifications.finishedMany(items.length);
  const [item] = items;
  return texts().notifications.finished(sessionName(item), item.project, formatDuration(item.durationMs));
}

export type Toast = Extract<Banner, { kind: "toast" }>;

/** What the island says once the user answered a permission (the answer, never more). */
export function answerToast(answer: "allow" | "deny" | "always" | "terminal", tool: string, detail: string): Toast {
  const said = texts().notifications;
  switch (answer) {
    case "allow":
      return { kind: "toast", tone: "ok", title: said.allowed.title, sub: said.allowed.sub(tool), ms: 2600 };
    case "deny":
      return { kind: "toast", tone: "err", title: said.denied.title, sub: said.denied.sub(tool), ms: 2600 };
    case "always":
      return { kind: "toast", tone: "ok", title: said.always.title, sub: said.always.sub(detail), ms: 3000 };
    case "terminal":
      return { kind: "toast", tone: "acc", title: said.inTerminal.title, sub: said.inTerminal.sub(tool), ms: 3000 };
  }
}

/** The agent's questions were answered from the island. */
export function questionToast(tool: string): Toast {
  const { answered } = texts().notifications;
  return { kind: "toast", tone: "ok", title: answered.title, sub: answered.sub(tool), ms: 2600 };
}

/** The last part of a Windows or Linux path: what the island shows of a dropped file. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** A file dropped on the island: only its path is copied, the island never reads it. */
export function dropToast(path: string): Toast {
  return {
    kind: "toast",
    tone: "acc",
    title: texts().notifications.dropped.title(fileName(path)),
    sub: texts().notifications.dropped.sub,
    ms: 3400,
  };
}
