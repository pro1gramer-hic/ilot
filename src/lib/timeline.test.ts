import { describe, expect, it } from "vitest";
import type { AgentSession } from "./agents";
import {
  HOUR_MS,
  MAX_STEPS,
  activeToday,
  addMark,
  busyShare,
  clockLabel,
  decide,
  doing,
  durationLabel,
  elapsedLabel,
  emptyTimeline,
  hasActivity,
  position,
  prune,
  record,
  startOfDay,
  stepText,
} from "./timeline";

const MIN = 60_000;
/** Noon, so that a few hours either way stay on the same day. */
const NOON = new Date(2026, 9, 11, 12, 0, 0).getTime();

const claude = (status: AgentSession["status"], extra: Partial<AgentSession> = {}): AgentSession => ({
  id: "claude-1",
  source: "claude-code",
  project: "epilot",
  task: "corrige le fuseau horaire",
  status,
  activity: { verb: "édite", target: "api.py" },
  ...extra,
});

describe("timeline", () => {
  it("opens a bar when an AI starts working and closes it when it stops", () => {
    let t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    expect(t.lanes["claude-code"].segments).toEqual([{ start: NOON, end: null, label: "Édite api.py" }]);
    expect(t.since).toEqual({ "claude-1": NOON });
    t = record(t, [claude("active")], [claude("active", { activity: { verb: "lit", target: "README.md" } })], NOON + MIN);
    // Still at work: the same bar, with what it does now.
    expect(t.lanes["claude-code"].segments).toEqual([{ start: NOON, end: null, label: "Lit README.md" }]);
    expect(t.since).toEqual({ "claude-1": NOON });
    t = record(t, [claude("active")], [claude("done")], NOON + 5 * MIN);
    expect(t.lanes["claude-code"].segments).toEqual([{ start: NOON, end: NOON + 5 * MIN, label: "Lit README.md" }]);
    expect(t.since).toEqual({});
  });

  it("never changes the timeline it was given", () => {
    const before = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    const copy = structuredClone(before);
    record(before, [claude("active")], [claude("done")], NOON + MIN);
    addMark(before, "codex", "done", "ilot", NOON + MIN);
    expect(before).toEqual(copy);
  });

  it("turns a new failure into a mark, once", () => {
    let t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    t = record(t, [claude("active")], [claude("failed")], NOON + MIN);
    t = record(t, [claude("failed")], [claude("failed")], NOON + 2 * MIN);
    expect(t.lanes["claude-code"].marks).toEqual([{ at: NOON + MIN, kind: "failed", label: "corrige le fuseau horaire" }]);
  });

  it("notes each visible change as a step, newest first, a few per AI", () => {
    let t = emptyTimeline(NOON);
    let previous: AgentSession[] = [];
    for (let i = 0; i < MAX_STEPS + 2; i++) {
      const next = [claude("active", { activity: { verb: "édite", target: `f${i}.py` } })];
      t = record(t, previous, next, NOON + i * MIN);
      previous = next;
    }
    // Nothing visible changed: no new step.
    t = record(t, previous, previous, NOON + 20 * MIN);
    const steps = t.lanes["claude-code"].steps;
    expect(steps).toHaveLength(MAX_STEPS);
    expect(steps[0]).toEqual({ at: NOON + (MAX_STEPS + 1) * MIN, text: `Édite f${MAX_STEPS + 1}.py` });
  });

  it("says what a session does in a few words", () => {
    expect(doing(claude("active"))).toBe("Édite api.py");
    expect(doing(claude("active", { activity: null }))).toBe("Corrige le fuseau horaire");
    expect(stepText(claude("waiting"))).toBe("Attend ton accord : corrige le fuseau horaire");
    expect(stepText(claude("done"))).toBe("A terminé : corrige le fuseau horaire");
    expect(stepText(claude("failed"))).toBe("A échoué : corrige le fuseau horaire");
    expect(stepText(claude("paused"))).toBe("En pause : corrige le fuseau horaire");
  });

  it("puts the user's verdict on the latest matching permission", () => {
    let t = addMark(emptyTimeline(NOON), "claude-code", "permission", "npm test", NOON);
    t = addMark(t, "claude-code", "permission", "npm test", NOON + MIN);
    const decision = { at: NOON + 2 * MIN, source: "claude-code", project: "epilot", command: "npm test", decision: "allow" as const };
    t = decide(t, decision);
    expect(t.lanes["claude-code"].marks.map((m) => m.verdict)).toEqual([undefined, "allow"]);
    // An unknown AI or command changes nothing.
    expect(decide(t, { ...decision, source: "aider" })).toBe(t);
    expect(decide(t, { ...decision, command: "ls" })).toBe(t);
  });

  it("forgets what left the hour, and keeps the same object otherwise", () => {
    let t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    t = record(t, [claude("active")], [claude("done")], NOON + MIN);
    t = addMark(t, "claude-code", "done", "epilot", NOON + MIN);
    expect(prune(t, NOON + 30 * MIN)).toBe(t);
    const later = prune(t, NOON + MIN + HOUR_MS + 1);
    expect(hasActivity(later.lanes["claude-code"])).toBe(false);
    // The latest actions stay on the card.
    expect(later.lanes["claude-code"].steps).toHaveLength(2);
  });

  it("keeps a bar still at work, however old its start", () => {
    const t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    expect(prune(t, NOON + 3 * HOUR_MS).lanes["claude-code"].segments).toHaveLength(1);
    expect(busyShare(t.lanes["claude-code"], NOON + 3 * HOUR_MS)).toBe(1);
  });

  it("measures the share of the last hour spent at work", () => {
    let t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    t = record(t, [claude("active")], [claude("done")], NOON + 15 * MIN);
    expect(busyShare(t.lanes["claude-code"], NOON + 30 * MIN)).toBe(0.25);
    // Only what falls inside the hour counts.
    expect(busyShare(t.lanes["claude-code"], NOON + HOUR_MS + 5 * MIN)).toBeCloseTo(10 / 60);
    expect(busyShare(emptyTimeline(NOON).lanes.codex, NOON)).toBe(0);
  });

  it("counts today's active time once, however many AIs work at once", () => {
    const gemini: AgentSession = { ...claude("active"), id: "gemini-1", source: "gemini-cli" };
    let t = record(emptyTimeline(NOON), [], [claude("active"), gemini], NOON);
    t = record(t, [claude("active"), gemini], [claude("done"), gemini], NOON + 10 * MIN);
    expect(activeToday(t.day, NOON + 20 * MIN)).toBe(20 * MIN);
    t = record(t, [claude("done"), gemini], [claude("done"), { ...gemini, status: "done" }], NOON + 30 * MIN);
    expect(activeToday(t.day, NOON + 45 * MIN)).toBe(30 * MIN);
  });

  it("starts a new day from zero", () => {
    let t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    t = record(t, [claude("active")], [claude("done")], NOON + 30 * MIN);
    const tomorrow = NOON + 24 * HOUR_MS;
    expect(activeToday(t.day, tomorrow)).toBe(0);
    t = record(t, [claude("done")], [claude("active")], tomorrow);
    expect(activeToday(t.day, tomorrow + MIN)).toBe(MIN);
  });

  it("restarts a stretch still open at midnight", () => {
    const t = record(emptyTimeline(NOON), [], [claude("active")], NOON);
    const tomorrow = NOON + 24 * HOUR_MS;
    expect(startOfDay(NOON)).toBe(NOON - 12 * HOUR_MS);
    expect(activeToday(t.day, tomorrow)).toBe(12 * HOUR_MS);
    expect(activeToday(record(t, [claude("active")], [claude("active")], tomorrow).day, tomorrow)).toBe(12 * HOUR_MS);
  });

  it("places a time on the hour axis", () => {
    expect(position(NOON, NOON)).toBe(1);
    expect(position(NOON - 30 * MIN, NOON)).toBe(0.5);
    expect(position(NOON - 2 * HOUR_MS, NOON)).toBe(0);
    expect(position(NOON + MIN, NOON)).toBe(1);
  });

  it("writes durations and times for people", () => {
    expect(durationLabel(4 * MIN)).toBe("4 min");
    expect(durationLabel(65 * MIN)).toBe("1 h 05");
    expect(elapsedLabel(161_000)).toBe("02:41");
    expect(elapsedLabel(HOUR_MS + 161_000)).toBe("1:02:41");
    expect(clockLabel(NOON + 7 * MIN)).toBe("12:07");
  });
});
