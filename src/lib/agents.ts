// Agent sessions as the island shows them (docs/04-CONNECTEURS-IA.md, "Compteurs"), the
// decisions journal and the connection of each AI (docs/09-DEMO-FINALE.md, sections 8, 9).

import type { Catalog } from "./i18n/fr";
import { texts, textsLocale } from "./i18n/live.svelte";
import type { ConnectorView, Decision, DecisionKind } from "./ipc";

/** îlot's first four sources, then the agents it follows for their activity only
 * (docs/AGENTS.md). Same ids as `Source` in src-tauri/src/events.rs. */
export type AiSource = "claude-code" | "gemini-cli" | "codex" | "generic" | "cursor" | "copilot" | "opencode" | "amp" | "bob" | "kiro";

export const AI_SOURCES: readonly AiSource[] = ["claude-code", "gemini-cli", "codex", "generic", "cursor", "copilot", "opencode", "amp", "bob", "kiro"];

/** The AIs that always have their dot in the pill and their line in the timeline. */
export const MAIN_SOURCES: readonly AiSource[] = ["claude-code", "gemini-cli", "codex"];

export const AI: Record<AiSource, { name: string; tool: string; color: string }> = {
  "claude-code": { name: "Claude", tool: "Claude Code", color: "var(--ai-claude)" },
  "gemini-cli": { name: "Gemini", tool: "Gemini CLI", color: "var(--ai-gemini)" },
  codex: { name: "Codex", tool: "Codex", color: "var(--ai-codex)" },
  generic: {
    get name() {
      return texts().agents.generic.name;
    },
    get tool() {
      return texts().agents.generic.tool;
    },
    color: "var(--ai-other)",
  },
  cursor: { name: "Cursor", tool: "Cursor", color: "var(--ai-cursor)" },
  copilot: { name: "Copilot", tool: "Copilot CLI", color: "var(--ai-copilot)" },
  opencode: { name: "OpenCode", tool: "OpenCode", color: "var(--ai-opencode)" },
  amp: { name: "Amp", tool: "Amp", color: "var(--ai-amp)" },
  bob: { name: "Bob", tool: "IBM Bob", color: "var(--ai-bob)" },
  kiro: { name: "Kiro", tool: "Kiro", color: "var(--ai-kiro)" },
};

/** One value per AI. */
export function bySource<T>(value: () => T): Record<AiSource, T> {
  return Object.fromEntries(AI_SOURCES.map((source) => [source, value()])) as Record<AiSource, T>;
}

/** Colors a hook may ask for with `ilot_agent` (src-tauri/src/connectors/custom_agent.rs). */
export type BadgeColor = "sky" | "teal" | "lime" | "gold" | "coral" | "pink" | "violet" | "slate";

/** The badge a hook asked for: its own name and color, its own dot and tile. */
export type AgentBadge = { id: string; name: string; color: BadgeColor };

/** Where a session runs, as far as the relay could tell (« Aller au terminal »). */
export type TerminalHost = "windows-terminal" | "vs-code" | "cursor" | "console";

export type SessionStatus = "active" | "waiting" | "paused" | "done" | "failed";

export const STATUS_LABEL: Record<SessionStatus, string> = {
  get active() {
    return texts().agents.status.active;
  },
  get waiting() {
    return texts().agents.status.waiting;
  },
  get paused() {
    return texts().agents.status.paused;
  },
  get done() {
    return texts().agents.status.done;
  },
  get failed() {
    return texts().agents.status.failed;
  },
};

/** State colors (reserved): only on the dot next to the word, never on the text. */
export type Tone = "ok" | "wait" | "err" | "idle";

export const STATUS_TONE: Record<SessionStatus, Tone> = {
  active: "ok",
  waiting: "wait",
  paused: "idle",
  done: "idle",
  failed: "err",
};

/** What the agent is doing right now, for the compact island ("Claude édite api.py +12 −3").
 * `action` is stable; `verb` is the same action already in the language of the moment
 * (src-tauri/src/events.rs). */
export type Activity = { action?: VerbKey; verb: string; target?: string; added?: number; removed?: number };

/** An entry of the catalog's verbs. */
export type VerbKey = keyof Catalog["agents"]["verbs"];

/** The verbs the core sends, and their entry in the catalog. */
const VERB_OF_CORE: Readonly<Record<string, VerbKey>> = {
  lance: "run",
  lit: "read",
  "édite": "edit",
  "écrit": "write",
  parcourt: "browse",
  cherche: "search",
  consulte: "fetch",
  "délègue": "delegate",
  planifie: "plan",
  demande: "ask",
  utilise: "use",
  "réfléchit": "think",
  travaille: "work",
};

/** The catalog's entry for a verb of the core, or for a verb as the language of the moment
 * shows it (the first word of a journal line: « Édite api.py », "Edits api.py"). */
export function verbKey(verb: string): VerbKey | undefined {
  const word = verb.toLocaleLowerCase(textsLocale());
  const verbs = texts().agents.verbs;
  return VERB_OF_CORE[word] ?? (Object.keys(verbs) as VerbKey[]).find((key) => verbs[key] === word);
}

/** What an activity does: its stable action from the core, else read from its verb. */
export function actionOf(activity: Activity | null | undefined): VerbKey | undefined {
  return activity ? (activity.action ?? verbKey(activity.verb)) : undefined;
}

/** A verb of the core in the language of the moment (« lance », "runs"); an unknown one as is. */
export function verbLabel(verb: string): string {
  const key = verbKey(verb);
  return key ? texts().agents.verbs[key] : verb;
}

/** A subagent working for a session: its `agent_type` ("Explore", "Plan"…) when known. */
export type Subagent = { id: string; kind?: string };

export type AgentSession = {
  id: string;
  source: AiSource;
  /** Name chosen for a generic tool (`ilot emit --source ci`). */
  name?: string;
  project: string;
  task: string;
  status: SessionStatus;
  activity?: Activity | null;
  /** Subagents working for it now (Claude Code, Codex, Gemini CLI, Copilot and OpenCode
   * say when they start and stop). */
  subagents?: Subagent[];
  /** The badge its hooks asked for (`ilot_agent`). */
  agent?: AgentBadge;
  /** It waits for an answer in its own terminal, which îlot cannot give. */
  inTerminal?: boolean;
  /** Where it runs (Windows Terminal, VS Code…), when the relay could tell. */
  terminal?: TerminalHost;
};

/** The color of a session's dot and bar: its badge's, else its AI's. */
export function sessionColor(session: Pick<AgentSession, "source" | "agent">): string {
  return session.agent ? `var(--agent-${session.agent.color})` : AI[session.source].color;
}

/** What groups sessions under one dot: the badge, else the AI. */
export function dotKey(session: Pick<AgentSession, "source" | "agent">): string {
  return session.agent ? `agent:${session.agent.id}` : session.source;
}

/** "1 sous-agent", "3 sous-agents". */
export function subagentsLabel(count: number): string {
  return texts().agents.subagents(count);
}

/** What the subagents are, grouped: "Explore ×2, Plan". Unknown kinds are left out. */
export function subagentKinds(subagents: readonly Subagent[]): string {
  const counts = new Map<string, number>();
  for (const { kind } of subagents) {
    if (kind) counts.set(kind, (counts.get(kind) ?? 0) + 1);
  }
  return [...counts].map(([kind, count]) => (count > 1 ? `${kind} ×${count}` : kind)).join(", ");
}

/** "3 sous-agents : Explore ×2, Plan", or the count alone when no kind is known. */
export function subagentsLine(subagents: readonly Subagent[]): string {
  const kinds = subagentKinds(subagents);
  return kinds ? texts().agents.subagentsOf(subagentsLabel(subagents.length), kinds) : subagentsLabel(subagents.length);
}

/** The name shown for a session: its badge's, the one the user gave a generic tool, or
 * the AI's. */
export function sessionName(session: Pick<AgentSession, "source" | "name" | "agent">): string {
  return session.agent?.name ?? session.name ?? AI[session.source].name;
}

/** The longer name for lists: "Claude Code", "Gemini CLI", a badge's or a tool's name. */
export function sessionTool(session: Pick<AgentSession, "source" | "name" | "agent">): string {
  return session.agent?.name ?? session.name ?? AI[session.source].tool;
}

/** The agent waits in its own terminal (Gemini's permissions, a question of Codex, an
 * approval of Copilot or OpenCode…): îlot can only say it waits. */
export function answersInTerminal(session: AgentSession): boolean {
  return session.status === "waiting" && (session.inTerminal === true || session.source === "gemini-cli");
}

/** The latest file change of a session, for the counter of the activity line
 * (« api.py +12 −3 »): only while it edits or writes, and once lines changed. */
export function editCounter(session: AgentSession): { file: string; added: number; removed: number } | null {
  const activity = session.activity;
  const editing = actionOf(activity) === "edit" || actionOf(activity) === "write";
  const added = activity?.added ?? 0;
  const removed = activity?.removed ?? 0;
  if (!editing || !activity?.target || added + removed === 0) return null;
  return { file: activity.target, added, removed };
}

/** A session counts while it works or waits for an answer. */
export function isLive(session: AgentSession): boolean {
  return session.status === "active" || session.status === "waiting";
}

export function countBySource(sessions: readonly AgentSession[]): Record<AiSource, number> {
  const counts = bySource(() => 0);
  for (const session of sessions) {
    if (isLive(session)) counts[session.source] += 1;
  }
  return counts;
}

/** The session the compact island follows: one that waits first, then the first live one. */
export function followedSession(sessions: readonly AgentSession[]): AgentSession | undefined {
  return sessions.find((s) => s.status === "waiting") ?? sessions.find(isLive);
}

/** "2 agents au travail" style summary for the expanded header. */
export function summary(sessions: readonly AgentSession[]): string {
  const counts = countBySource(sessions);
  const total = AI_SOURCES.reduce((sum, source) => sum + counts[source], 0);
  if (total === 0) return texts().agents.calm;
  const detail = AI_SOURCES.filter((s) => counts[s] > 0)
    .map((s) => `${AI[s].name} ${counts[s]}`)
    .join(", ");
  return texts().agents.atWork(total, detail);
}

/** One line for the pill's tooltip: "Claude travaille sur epilot, Gemini sur site". */
export function activityLine(sessions: readonly AgentSession[]): string {
  const waiting = sessions.find((s) => s.status === "waiting");
  if (waiting) return texts().agents.waitsOn(sessionName(waiting), waiting.project, answersInTerminal(waiting));
  const live = sessions.filter(isLive);
  if (live.length === 0) return texts().agents.nobody;
  return live.map((s, index) => texts().agents.worksOn(sessionName(s), s.project, index === 0)).join(", ");
}

/** One step of the pill's activity line (« Activité » mode): "édite api.py". */
export function tickerText(session: AgentSession): string {
  if (session.status === "waiting") return texts().agents.tickerWaiting;
  const { activity } = session;
  if (activity) return activity.target ? `${verbLabel(activity.verb)} ${activity.target}` : verbLabel(activity.verb);
  return texts().agents.tickerWorking(session.project);
}

/** The dot of an AI in the pill: lit while it works, amber while it waits for the user. */
export type DotState = "idle" | "work" | "perm";

/** One dot of the pill: an AI, or a badge a hook asked for. */
export type PillDot = { key: string; name: string; color: string; state: DotState; subagents: number };

/** The pill's dots, in a fixed order: Claude, Gemini and Codex always; any other AI and
 * any badge while one of their sessions is there. Badged sessions have their own dot. */
export function pillDots(sessions: readonly AgentSession[]): PillDot[] {
  const dots = new Map<string, PillDot>();
  const dot = (key: string, name: string, color: string): PillDot => {
    if (!dots.has(key)) dots.set(key, { key, name, color, state: "idle", subagents: 0 });
    return dots.get(key)!;
  };
  for (const source of AI_SOURCES) {
    if (MAIN_SOURCES.includes(source) || sessions.some((s) => s.source === source && !s.agent)) dot(source, AI[source].name, AI[source].color);
  }
  for (const session of sessions) {
    const shown = dot(dotKey(session), sessionName(session), sessionColor(session));
    if (session.status === "waiting") shown.state = "perm";
    else if (session.status === "active" && shown.state === "idle") shown.state = "work";
    shown.subagents += session.subagents?.length ?? 0;
  }
  return [...dots.values()];
}

/** A dot as drawn: its state, or the flash of a turn that just ended. */
export type ShownDot = Omit<PillDot, "state"> & { state: DotState | "ok" | "err" };

/** What needs the user first when the pill lacks room: a wait, a failure, work. */
const DOT_RANK: Record<ShownDot["state"], number> = { perm: 0, err: 1, work: 2, ok: 3, idle: 4 };

/** The dots that fit in the pill (`max`): when there are more, those that need the user
 * first keep their place, the others are counted (« +2 »); the order never changes. */
export function fitDots(dots: readonly ShownDot[], max: number): { shown: ShownDot[]; hidden: number } {
  if (dots.length <= max) return { shown: [...dots], hidden: 0 };
  const kept = new Set([...dots].sort((a, b) => DOT_RANK[a.state] - DOT_RANK[b.state]).slice(0, max - 1));
  return { shown: dots.filter((dot) => kept.has(dot)), hidden: dots.length - kept.size };
}

/** Sessions that failed since the previous list: the island opens « … a échoué ». */
export function newlyFailed(previous: readonly AgentSession[], next: readonly AgentSession[]): AgentSession[] {
  const before = new Map(previous.map((s) => [s.id, s.status]));
  return next.filter((s) => s.status === "failed" && before.get(s.id) !== "failed");
}

/** A state in words, with the reserved color of its dot. */
export type Shown = { label: string; tone: Tone };

/** How each decision reads in the journal (docs/09-DEMO-FINALE.md, section 9). */
export const DECISION: Record<DecisionKind, Shown> = {
  allow: {
    get label() {
      return texts().agents.decisions.allow;
    },
    tone: "ok",
  },
  always: {
    get label() {
      return texts().agents.decisions.always;
    },
    tone: "ok",
  },
  deny: {
    get label() {
      return texts().agents.decisions.deny;
    },
    tone: "err",
  },
  suspended: {
    get label() {
      return texts().agents.decisions.suspended;
    },
    tone: "err",
  },
  terminal: {
    get label() {
      return texts().agents.decisions.terminal;
    },
    tone: "idle",
  },
  expired: {
    get label() {
      return texts().agents.decisions.expired;
    },
    tone: "idle",
  },
};

/** The journal keeps this many decisions, in memory only. */
export const MAX_DECISIONS = 200;

/** Newest first, at most `max`; the same decision heard twice is kept once. */
export function addDecision(list: readonly Decision[], decision: Decision, max = MAX_DECISIONS): Decision[] {
  const same = (d: Decision): boolean =>
    d.at === decision.at && d.source === decision.source && d.command === decision.command && d.decision === decision.decision;
  if (list.some(same)) return [...list];
  return [decision, ...list].sort((a, b) => b.at - a.at).slice(0, max);
}

/** How many decisions were taken since `since` (the start of the day). */
export function decisionsSince(list: readonly Decision[], since: number): number {
  return list.filter((d) => d.at >= since).length;
}

/** Name of the AI behind a source id that the core sends as text. */
export function sourceTool(source: string): string {
  return source in AI ? AI[source as AiSource].tool : source;
}

export function sourceColor(source: string): string {
  return source in AI ? AI[source as AiSource].color : "var(--ai-other)";
}

/** Where an AI stands with îlot: not installed, installed but off, plugged, or to repair. */
export type Plug = "absent" | "off" | "on" | "repair";

export function plugState(view: ConnectorView | undefined): Plug {
  const targets = view?.targets.filter((t) => t.status !== "notInstalled") ?? [];
  if (targets.length === 0) return "absent";
  if (targets.some((t) => t.status === "needsRepair")) return "repair";
  return targets.some((t) => t.status === "connected") ? "on" : "off";
}

/** Places a switch acts on: turned on, every installed place not plugged yet (a repair
 * plugs again); turned off, every plugged one. */
export function targetsToFlip(view: ConnectorView, on: boolean): string[] {
  return view.targets
    .filter((t) => (on ? t.status === "installed" || t.status === "needsRepair" : t.status === "connected"))
    .map((t) => t.id);
}

/** State of an AI on its card, from its sessions and its connection. A tool that reports
 * on its own (`ilot emit`) shows its sessions even when nothing is plugged. */
export function agentState(sessions: readonly AgentSession[], source: AiSource, plug: Plug, suspended: boolean): Shown {
  const mine = sessions.filter((s) => s.source === source);
  const state = texts().agents.state;
  if (plug === "repair") return { label: state.repair, tone: "wait" };
  if (mine.length === 0 && plug === "absent") return { label: state.absent, tone: "idle" };
  if (mine.length === 0 && plug === "off") return { label: state.off, tone: "idle" };
  if (suspended) return { label: state.suspended, tone: "idle" };
  if (mine.some((s) => s.status === "waiting")) return { label: state.waiting, tone: "wait" };
  if (mine.some((s) => s.status === "active")) return { label: state.working, tone: "ok" };
  if (mine.some((s) => s.status === "failed")) return { label: state.failed, tone: "err" };
  if (mine.some((s) => s.status === "done")) return { label: state.done, tone: "idle" };
  return { label: state.idle, tone: "idle" };
}

/** Mochi's mood in the panel, in words and a dot: the suspension first, then what waits
 * for the user, then a failure. */
export function panelMood(sessions: readonly AgentSession[], asking: boolean, suspended: boolean): Shown {
  const mood = texts().agents.mood;
  if (suspended) return { label: mood.suspended, tone: "idle" };
  if (asking || sessions.some((s) => s.status === "waiting")) return { label: mood.waiting, tone: "wait" };
  if (sessions.some((s) => s.status === "failed")) return { label: mood.failed, tone: "err" };
  return { label: mood.fine, tone: "ok" };
}
