// The hub (docs/09-DEMO-FINALE.md, section 8 bis): the compact panel that comes out of the
// island on a click. Pure rules: which sessions it shows (the agents of the sessions store
// and the music playing), the one it follows, the live session's four steps, Mochi's
// answer to pokes, the intro's beats, and the browser preview's staged session. The
// components in src/components/hub draw them; nothing here touches a store.

import { type AgentSession, type AiSource, type BadgeColor, type VerbKey, sessionColor, sessionTool, subagentsLabel, actionOf, verbKey } from "./agents";
import { liveList, texts } from "./i18n/live.svelte";
import { type Track, sourceLabel } from "./media";
import type { FaceId } from "./mochi/faces";
import { type MoveId, OWN_MOVE_S } from "./mochi/moves";
import type { PaletteId } from "./mochi/palettes";

/** What the hub shows: its home, the latest messages, the pastilles, a session followed
 * live, Mochi out of breath after too many clicks, the chat, or an e-mail draft (prompt 12). */
export type HubView = "home" | "messages" | "pastilles" | "live" | "dizzy" | "chat" | "mail";

/** A session's state, as its icon and its badge show it. */
export type HubStatus = "work" | "perm" | "err" | "ok" | "play" | "idle";

/** Each AI has its Mochi color; the music has its own. */
const SOURCE_PALETTE: Readonly<Record<AiSource, PaletteId>> = {
  "claude-code": "peche",
  "gemini-cli": "ciel",
  codex: "menthe",
  generic: "nuit",
  cursor: "ivoire",
  copilot: "lagon",
  opencode: "anis",
  amp: "fuchsia",
  bob: "pervenche",
  kiro: "glycine",
};

/** A badge's Mochi, in the hue of its color. */
const BADGE_PALETTE: Readonly<Record<BadgeColor, PaletteId>> = {
  sky: "ciel",
  teal: "lagon",
  lime: "anis",
  gold: "citron",
  coral: "peche",
  pink: "fuchsia",
  violet: "glycine",
  slate: "nuit",
};

/** The Mochi of a session's tile: its badge's colors, else its AI's. */
export function sessionPalette(session: Pick<AgentSession, "source" | "agent">): PaletteId {
  return session.agent ? BADGE_PALETTE[session.agent.color] : SOURCE_PALETTE[session.source];
}
const MUSIC_PALETTE: PaletteId = "citron";

const MUSIC_COLOR: Readonly<Record<Track["source"], string>> = {
  Spotify: "var(--src-spotify)",
  YouTube: "var(--src-youtube)",
  Crunchyroll: "var(--src-crunchyroll)",
  other: "var(--ink-3)",
};

type Common = { key: string; name: string; tool: string; status: HubStatus; palette: PaletteId; color: string };
export type HubSession = (Common & { kind: "agent"; session: AgentSession }) | (Common & { kind: "music"; track: Track });

function agentStatus(session: AgentSession): HubStatus {
  switch (session.status) {
    case "active":
      return "work";
    case "waiting":
      return "perm";
    case "failed":
      return "err";
    case "done":
      return "ok";
    case "paused":
      return "idle";
  }
}

/** The hub's sessions: every agent session, in the store's order, then the music shown
 * (the one Windows plays, or the last one paused). */
export function hubSessions(list: readonly AgentSession[], track: Track | null): HubSession[] {
  const agents: HubSession[] = list.map((session) => ({
    kind: "agent",
    key: `agent:${session.id}`,
    name: session.project,
    tool: sessionTool(session),
    status: agentStatus(session),
    palette: sessionPalette(session),
    color: sessionColor(session),
    session,
  }));
  if (!track) return agents;
  return [
    ...agents,
    {
      kind: "music",
      key: "music",
      name: track.title,
      tool: sourceLabel(track),
      status: track.playing ? "play" : "idle",
      palette: MUSIC_PALETTE,
      color: MUSIC_COLOR[track.source],
      track,
    },
  ];
}

/** What needs the user first, when nothing was chosen: a request, work going on, a
 * failure, the music, then the rest. */
const PRIORITY: readonly HubStatus[] = ["perm", "work", "err", "play", "ok", "idle"];

/** The session on the left card: the one the user chose while it is still there, else
 * the one that matters most now. */
export function pickFollowed(list: readonly HubSession[], chosen: string | null): HubSession | undefined {
  const picked = list.find((s) => s.key === chosen);
  if (picked) return picked;
  for (const status of PRIORITY) {
    const found = list.find((s) => s.status === status);
    if (found) return found;
  }
  return undefined;
}

/** Icons that fit beside the « + » tile (two rows of three). */
const MAX_TILES = 5;

/** The icons on the right: all of them when they fit; otherwise the followed one and those
 * that matter most, kept in their own order so that nothing moves around. */
export function visibleTiles(list: readonly HubSession[], followed: string | undefined, max = MAX_TILES): HubSession[] {
  if (list.length <= max) return [...list];
  const rank = (s: HubSession): number => (s.key === followed ? -1 : PRIORITY.indexOf(s.status));
  const kept = new Set([...list].sort((a, b) => rank(a) - rank(b)).slice(0, max));
  return list.filter((s) => kept.has(s));
}

/** Name and state of an icon, for its tooltip (« epilot, travail en cours ») and, with the
 * tool, for screen readers (« epilot, Claude Code, travail en cours »). */
/** Subagents working for a session (none for the music). */
export function tileSubagents(session: HubSession): number {
  return session.kind === "agent" ? (session.session.subagents?.length ?? 0) : 0;
}

function withSubagents(text: string, session: HubSession): string {
  const count = tileSubagents(session);
  return count > 0 ? `${text}, ${subagentsLabel(count)}` : text;
}

export function tileTip(session: HubSession): string {
  return withSubagents(`${session.name}, ${texts().agents.hub[session.status]}`, session);
}

export function tileLabel(session: HubSession): string {
  return withSubagents(`${session.name}, ${session.tool}, ${texts().agents.hub[session.status]}`, session);
}

/** The count of the top bar. */
export function workingLabel(count: number, suspended: boolean): string {
  if (suspended) return texts().agents.suspended;
  return count > 0 ? texts().agents.working(count) : texts().agents.noneWorking;
}

/** How Mochi stands for a session: his move and, for some states, his face. */
export function mochiFor(status: HubStatus): { move: MoveId; face?: FaceId } {
  switch (status) {
    case "err":
    case "perm":
      return { move: "shake", face: "worry" };
    case "ok":
      return { move: "bounce", face: "happy" };
    case "play":
      return { move: "rock", face: "happy" };
    default:
      return { move: "breathe" };
  }
}

/** What the left card says about a session. */
type FocusKind = "music" | "waiting" | "failed" | "done" | "command" | "action" | "idle";

export function focusKind(session: HubSession): FocusKind {
  if (session.kind === "music") return "music";
  switch (session.status) {
    case "perm":
      return "waiting";
    case "err":
      return "failed";
    case "ok":
      return "done";
    case "work":
      return actionOf(session.session.activity) === "run" ? "command" : "action";
    default:
      return "idle";
  }
}

/** A session can be followed live while its agent works or waits. */
export function canFollow(session: HubSession | undefined): boolean {
  return session?.kind === "agent" && (session.status === "work" || session.status === "perm");
}

// ---------- the live session: Lire, Éditer, Commande, Terminé ----------

export const LIVE_STEPS: readonly string[] = liveList(4, (step) => texts().agents.liveSteps[step]);

export type StepState = "todo" | "now" | "done";

/** The step of each verb the core sends (src-tauri/src/connectors/activity.rs). */
const STEP_OF_VERB: Partial<Record<VerbKey, number>> = {
  read: 0,
  browse: 0,
  search: 0,
  fetch: 0,
  think: 0,
  plan: 0,
  delegate: 0,
  edit: 1,
  write: 1,
  run: 2,
};

const stepOf = (key: VerbKey | undefined): number | null => (key === undefined ? null : (STEP_OF_VERB[key] ?? null));

/** The step of what an agent does now (its stable action from the core). */
export function stepOfActivity(activity: AgentSession["activity"]): number | null {
  return stepOf(actionOf(activity));
}

/** The step of a line of the agent's journal (« Édite api.py », "Runs npm test"). */
export function stepOfLine(text: string): number | null {
  return stepOf(verbKey(text.split(/\s/)[0]));
}

/** The four steps of a session: the one it is on now, those it went through lately
 * (its journal), all of them once it is done. Nothing is ticked that îlot did not see. */
export function liveSteps(session: AgentSession, journal: readonly string[]): StepState[] {
  if (session.status === "done") return LIVE_STEPS.map(() => "done");
  const now = session.status === "active" ? stepOfActivity(session.activity) : null;
  const seen = new Set(journal.map(stepOfLine));
  return LIVE_STEPS.map((_, index) => (index === now ? "now" : index < 3 && seen.has(index) ? "done" : "todo"));
}

/** « 2/4 »: the step on now, or how far the session went. */
export function stepCount(states: readonly StepState[]): string {
  const now = states.indexOf("now");
  const reached = now >= 0 ? now + 1 : states.filter((s) => s === "done").length;
  return `${reached}/${LIVE_STEPS.length}`;
}

// ---------- too many clicks ----------

/** Five clicks within 1.6 s make Mochi dizzy for three seconds. */
export const POKE_WINDOW_MS = 1600;
const POKE_LIMIT = 5;
export const DIZZY_SECONDS = 3;

/** Squashed with big eyes, then annoyed, then dizzy. */
export type PokeReaction = "squish" | "annoyed" | "dizzy";

export function poke(times: readonly number[], at: number): { times: number[]; reaction: PokeReaction } {
  const recent = [...times.filter((t) => at - t < POKE_WINDOW_MS), at];
  if (recent.length >= POKE_LIMIT) return { times: [], reaction: "dizzy" };
  return { times: recent, reaction: recent.length >= 3 ? "annoyed" : "squish" };
}

// ---------- the intro: a starry sky, Mochi wakes up, catches a ball, sways ----------

type IntroPhase = "sky" | "blink" | "breathe" | "ball" | "catch" | "sway" | "done" | "hello";

/** Start of each beat, in milliseconds from the hub's opening. */
const INTRO_BEATS: readonly [IntroPhase, number, MoveId, FaceId | undefined][] = [
  ["sky", 0, "still", "sleep"],
  ["blink", 900, "still", "blink"],
  ["breathe", 1060, "breathe", undefined],
  ["ball", 1560, "watch", undefined],
  ["catch", 2910, "bounce", "happy"],
  ["sway", 3810, "rock", "wink"],
  ["done", 4510, "breathe", undefined],
];

/** Length of the intro; the ball takes its flight time to reach Mochi. */
export const INTRO_MS = 4510;
export const BALL_MS = 1350;
/** After the launch intro, the hub folds back on its own unless the user touches it. */
export const INTRO_LINGER_MS = 2600;

export function introAt(t: number): { phase: IntroPhase; move: MoveId; face?: FaceId } {
  let beat = INTRO_BEATS[0];
  for (const candidate of INTRO_BEATS) if (t >= candidate[1]) beat = candidate;
  const [phase, , move, face] = beat;
  return face ? { phase, move, face } : { phase, move };
}

/** At launch only (prompt 12, lot 1), the intro ends with Mochi's greeting: once he
 * caught the ball, he waves hello for one cycle of « Salut » in place of the sway (the
 * greeting sways too). « Revoir l'intro » stays as it was. */
export const GREETING_MS = OWN_MOVE_S.hello * 1000;
/** The greeting starts where the sway did. */
const GREETING_AT = INTRO_BEATS.find(([phase]) => phase === "sway")![1];

export function introLength(greet: boolean): number {
  return greet ? GREETING_AT + GREETING_MS : INTRO_MS;
}

export function introBeat(t: number, greet: boolean): { phase: IntroPhase; move: MoveId; face?: FaceId } {
  return greet && t >= GREETING_AT ? { phase: "hello", move: "hello" } : introAt(t);
}

/** After the greeting, the hub lingers that much less: îlot's launch takes as long as it
 * did, the intro and the home together. */
export const LAUNCH_LINGER_MS = INTRO_LINGER_MS - (introLength(true) - INTRO_MS);

// ---------- the preview's staged session (fake data, browser preview only) ----------

/** A piece of a code line and its color: keyword, string, comment or plain. */
export type CodeToken = readonly [kind: "" | "k" | "s" | "c", text: string];

/** The demo's live session, played only by the browser preview: îlot never sees the
 * files an agent edits, so the app shows the agent's journal instead. */
export type HubStage = {
  /** Session it stages (`AgentSession.id`). */
  sessionId: string;
  file: string;
  path: string;
  /** Short language badge of the file tab. */
  lang: string;
  lines: readonly { n: number; tokens: readonly CodeToken[] }[];
  /** Lines replaced: struck in red, the new text typed in green below. */
  edits: readonly { line: number; text: string }[];
  /** The two lines of code on the left card. */
  snippet: readonly number[];
  terminal: readonly { text: string; tone?: "prompt" | "ok" | "pass" | "dim" }[];
  /** Called once the staged run is over (the preview updates its fake stores). */
  finish?: () => void;
};

/** Timing of the staged run, as in the reference. */
export const LIVE_MS = { readLine: 170, strike: 380, typeChar: 22, betweenEdits: 250, beforeTerminal: 350, prompt: 600, line: 360, party: 1700 };

type LiveFrame = {
  step: number;
  /** Line read now (its number), during the first step. */
  reading: number | null;
  edits: { line: number; struck: boolean; typed: string | null }[];
  /** The terminal slides up for the third step, and shows this many lines. */
  terminalOpen: boolean;
  terminalLines: number;
  done: boolean;
  /** The party is over: back to the home of the hub. */
  over: boolean;
};

/** The staged run at `t` ms. `instant` types each new line at once (reduced motion). */
export function liveFrame(stage: HubStage, t: number, instant = false): LiveFrame {
  const readEnd = stage.lines.length * LIVE_MS.readLine;
  let cursor = readEnd;
  const edits = stage.edits.map((edit, index) => {
    const struckAt = cursor;
    const typeAt = struckAt + LIVE_MS.strike;
    const typeEnd = typeAt + edit.text.length * LIVE_MS.typeChar;
    cursor = typeEnd + (index === stage.edits.length - 1 ? LIVE_MS.beforeTerminal : LIVE_MS.betweenEdits);
    const shown = instant ? edit.text.length : Math.min(edit.text.length, Math.floor((t - typeAt) / LIVE_MS.typeChar) + 1);
    return { line: edit.line, struck: t >= struckAt, typed: t >= typeAt ? edit.text.slice(0, shown) : null };
  });
  const terminalAt = cursor;
  let terminalLines = 0;
  for (const line of stage.terminal) {
    if (t >= cursor) terminalLines += 1;
    cursor += line.tone === "prompt" ? LIVE_MS.prompt : LIVE_MS.line;
  }
  const doneAt = cursor;
  const step = t < readEnd ? 0 : t < terminalAt ? 1 : t < doneAt ? 2 : 3;
  return {
    step,
    reading: t < readEnd ? (stage.lines[Math.floor(t / LIVE_MS.readLine)]?.n ?? null) : null,
    edits,
    terminalOpen: t >= terminalAt,
    terminalLines,
    done: t >= doneAt,
    over: t >= doneAt + LIVE_MS.party,
  };
}

/** States of the four steps in a staged frame. */
export function frameSteps(frame: LiveFrame): StepState[] {
  return LIVE_STEPS.map((_, index) => (frame.done || index < frame.step ? "done" : index === frame.step ? "now" : "todo"));
}
