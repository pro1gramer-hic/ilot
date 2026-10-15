// The last hour of every AI, for the activity timeline of the panel (« Aujourd'hui ») and
// the agent cards: when each AI worked (bars), what ended or asked (marks), its latest
// actions, and today's active time. A session carries no time, so the interface notes
// it as events arrive, in memory only. Pure: the store in stores/timeline.svelte.ts
// keeps the value.

import { AI_SOURCES, type AgentSession, type AiSource, bySource, verbLabel } from "./agents";
import { texts, textsLocale } from "./i18n/live.svelte";
import type { Decision, DecisionKind } from "./ipc";

export const HOUR_MS = 3_600_000;
/** Latest actions kept per AI for its card. */
export const MAX_STEPS = 4;

export type MarkKind = "done" | "failed" | "permission";

/** A stretch of work; `end` is null while it goes on. */
export type Segment = { start: number; end: number | null; label: string };
export type Mark = { at: number; kind: MarkKind; label: string; verdict?: DecisionKind };
export type Step = { at: number; text: string };
export type Lane = { segments: Segment[]; marks: Mark[]; steps: Step[] };

export type Timeline = {
  lanes: Record<AiSource, Lane>;
  /** When each working session started its current stretch, by session id. */
  since: Record<string, number>;
  /** Today's active time (at least one AI at work): closed stretches and the open one. */
  day: { date: string; closedMs: number; openSince: number | null };
};

const dayOf = (at: number): string => new Date(at).toDateString();

/** Local midnight of the day of `at`. */
export function startOfDay(at: number): number {
  return new Date(at).setHours(0, 0, 0, 0);
}

function emptyLanes(): Record<AiSource, Lane> {
  return bySource((): Lane => ({ segments: [], marks: [], steps: [] }));
}

/** An AI's line; empty for an AI a timeline built before it existed does not know. */
export function laneOf(timeline: Timeline, source: AiSource): Lane {
  return timeline.lanes[source] ?? { segments: [], marks: [], steps: [] };
}

export function emptyTimeline(now: number): Timeline {
  return { lanes: emptyLanes(), since: {}, day: { date: dayOf(now), closedMs: 0, openSince: null } };
}

const capitalize = (text: string): string => text.charAt(0).toLocaleUpperCase(textsLocale()) + text.slice(1);

/** What a session does, in a few words: « Édite api.py », else its task. */
export function doing(session: AgentSession): string {
  const { activity, task } = session;
  if (activity?.verb) return capitalize([verbLabel(activity.verb), activity.target].filter(Boolean).join(" "));
  return capitalize(task);
}

/** One line for the card's journal when a session changes. */
export function stepText(session: AgentSession): string {
  const steps = texts().timeline.steps;
  switch (session.status) {
    case "waiting":
      return steps.waiting(session.task);
    case "done":
      return steps.done(session.task);
    case "failed":
      return steps.failed(session.task);
    case "paused":
      return steps.paused(session.task);
    case "active":
      return doing(session);
  }
}

const isWorking = (session: AgentSession): boolean => session.status === "active";

/** What can be seen of a session: same signature, nothing to note. */
function signature(session: AgentSession): string {
  const { status, task, activity } = session;
  return JSON.stringify([status, task, activity?.verb, activity?.target]);
}

/** The sessions went from `previous` to `next` at `at`: bars open and close, failures
 * become marks, every visible change becomes a step of its AI. */
export function record(timeline: Timeline, previous: readonly AgentSession[], next: readonly AgentSession[], at: number): Timeline {
  const before = new Map(previous.map((session) => [session.id, session]));
  const lanes = emptyLanes();
  for (const source of AI_SOURCES) {
    const old = laneOf(timeline, source);
    const working = next.filter((session) => session.source === source && isWorking(session));
    const segments = old.segments.map((segment) => ({ ...segment }));
    const open = segments.find((segment) => segment.end === null);
    if (working.length > 0 && open) open.label = doing(working[0]);
    else if (working.length > 0) segments.push({ start: at, end: null, label: doing(working[0]) });
    else if (open) open.end = at;
    const mine = next.filter((session) => session.source === source);
    const failed = mine.filter((s) => s.status === "failed" && before.get(s.id)?.status !== "failed");
    const moved = mine.filter((s) => {
      const was = before.get(s.id);
      return !was || signature(was) !== signature(s);
    });
    lanes[source] = {
      segments,
      marks: [...old.marks, ...failed.map((s) => ({ at, kind: "failed" as const, label: s.task }))],
      steps: [...moved.map((s) => ({ at, text: stepText(s) })), ...old.steps].slice(0, MAX_STEPS),
    };
  }
  const since: Record<string, number> = {};
  for (const session of next.filter(isWorking)) since[session.id] = timeline.since[session.id] ?? at;
  return prune({ lanes, since, day: countDay(timeline.day, next.some(isWorking), at) }, at);
}

/** Today's active time moves on with the « at least one AI at work » state. */
function countDay(day: Timeline["day"], working: boolean, at: number): Timeline["day"] {
  const date = dayOf(at);
  let { closedMs, openSince } = day;
  if (date !== day.date) {
    // A new day: yesterday's time is forgotten, an open stretch restarts at midnight.
    closedMs = 0;
    openSince = openSince === null ? null : startOfDay(at);
  }
  if (working && openSince === null) openSince = at;
  if (!working && openSince !== null) {
    closedMs += at - openSince;
    openSince = null;
  }
  return { date, closedMs, openSince };
}

/** Active time of the day at `now`, in milliseconds. */
export function activeToday(day: Timeline["day"], now: number): number {
  if (day.date !== dayOf(now)) return day.openSince === null ? 0 : now - startOfDay(now);
  return day.closedMs + (day.openSince === null ? 0 : now - day.openSince);
}

/** A mark on an AI's line: a finished task, a permission asked. */
export function addMark(timeline: Timeline, source: AiSource, kind: MarkKind, label: string, at: number): Timeline {
  const lane = laneOf(timeline, source);
  return prune({ ...timeline, lanes: { ...timeline.lanes, [source]: { ...lane, marks: [...lane.marks, { at, kind, label }] } } }, at);
}

/** The user's verdict lands on the latest permission mark of the same command. */
export function decide(timeline: Timeline, decision: Decision): Timeline {
  if (!AI_SOURCES.includes(decision.source as AiSource)) return timeline;
  const source = decision.source as AiSource;
  const lane = laneOf(timeline, source);
  let index = lane.marks.length - 1;
  while (index >= 0 && !(lane.marks[index].kind === "permission" && lane.marks[index].label === decision.command && !lane.marks[index].verdict)) {
    index -= 1;
  }
  if (index < 0) return timeline;
  const marks = lane.marks.map((m, i) => (i === index ? { ...m, verdict: decision.decision } : m));
  return { ...timeline, lanes: { ...timeline.lanes, [source]: { ...lane, marks } } };
}

/** Forgets what left the hour (the same object when nothing did). */
export function prune(timeline: Timeline, now: number): Timeline {
  const from = now - HOUR_MS;
  const stale = AI_SOURCES.some((source) => {
    const lane = laneOf(timeline, source);
    return lane.segments.some((s) => s.end !== null && s.end < from) || lane.marks.some((m) => m.at < from);
  });
  if (!stale) return timeline;
  const lanes = emptyLanes();
  for (const source of AI_SOURCES) {
    const lane = laneOf(timeline, source);
    lanes[source] = {
      segments: lane.segments.filter((s) => s.end === null || s.end >= from),
      marks: lane.marks.filter((m) => m.at >= from),
      steps: lane.steps,
    };
  }
  return { ...timeline, lanes };
}

/** Share of the last hour an AI spent at work, from 0 to 1. */
export function busyShare(lane: Lane, now: number): number {
  const from = now - HOUR_MS;
  const worked = lane.segments.reduce((sum, s) => sum + Math.max(0, Math.min(s.end ?? now, now) - Math.max(s.start, from)), 0);
  return Math.min(1, worked / HOUR_MS);
}

/** Place of a time on the hour axis: 0 an hour ago, 1 now (clamped). */
export function position(at: number, now: number): number {
  return Math.min(1, Math.max(0, 1 - (now - at) / HOUR_MS));
}

/** Whether an AI has anything to show on the timeline. */
export function hasActivity(lane: Lane): boolean {
  return lane.segments.length > 0 || lane.marks.length > 0;
}

/** « 4 min », « 1 h 05 »: a duration for people. */
export function durationLabel(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / 60_000));
  const duration = texts().timeline.duration;
  if (minutes < 60) return duration.minutes(minutes);
  return duration.hours(Math.floor(minutes / 60), String(minutes % 60).padStart(2, "0"));
}

/** « 02:41 », then « 1:02:41 » past an hour: time elapsed on a running task. */
export function elapsedLabel(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  const two = (n: number): string => String(n).padStart(2, "0");
  const h = Math.floor(seconds / 3600);
  const rest = `${two(Math.floor((seconds % 3600) / 60))}:${two(seconds % 60)}`;
  return h > 0 ? `${h}:${rest}` : rest;
}

/** « 14:07 » ("2:07 PM"): the clock time of an event, as the language writes it. */
export function clockLabel(at: number): string {
  return new Intl.DateTimeFormat(textsLocale(), { timeStyle: "short" }).format(at);
}
