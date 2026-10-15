import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEMO_QUESTIONS } from "../fixtures";
import { LOOK_BEFORE_ANSWER_MS } from "../island/guard";

// An agent's questions in the island (Claude Code's AskUserQuestion): never answered on
// their own, each question needs its own look, and only the user's choices are sent.
const core = vi.hoisted(() => ({ answers: [] as unknown[][], permissions: [] as unknown[][] }));

vi.mock("../ipc", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../ipc")>()),
  answerQuestions: async (...args: unknown[]) => {
    core.answers.push(args);
  },
  answerPermission: async (...args: unknown[]) => {
    core.permissions.push(args);
  },
}));

const { permissions } = await import("./permissions.svelte");
const { banners } = await import("./banners.svelte");

let now = 10_000;

beforeEach(() => {
  vi.spyOn(performance, "now").mockImplementation(() => now);
  core.answers.length = 0;
  core.permissions.length = 0;
  permissions.clear();
  banners.queue.clear();
  permissions.add({ ...structuredClone(DEMO_QUESTIONS), receivedAt: Date.now() });
  permissions.guard.show({ surface: "island", id: "question-2" }, now);
  permissions.guard.setWhole("island", "question-2", true);
});

afterEach(() => vi.restoreAllMocks());

const look = (): void => {
  now += LOOK_BEFORE_ANSWER_MS;
};

describe("an agent's questions", () => {
  it("are answered one after the other, each after its own look", () => {
    permissions.pick("question-2", "Luxon");
    expect(permissions.submitQuestion("question-2"), "too soon").toBe(false);
    look();
    expect(permissions.submitQuestion("question-2")).toBe(true);
    expect(permissions.draft("question-2").step).toBe(1);
    // A double Enter never answers the next question.
    permissions.pick("question-2", "Tests");
    expect(permissions.submitQuestion("question-2")).toBe(false);
    look();
    permissions.pick("question-2", "Types");
    permissions.type("question-2", "Perf");
    expect(permissions.submitQuestion("question-2")).toBe(true);
    expect(core.answers).toEqual([["question-2", [{ picked: ["Luxon"] }, { picked: ["Tests", "Types"], other: "Perf" }]]]);
    expect(permissions.queue).toEqual([]);
    expect(banners.current).toMatchObject({ kind: "toast", title: "Réponse envoyée" });
  });

  it("send nothing without a choice, nor a question shown in part", () => {
    look();
    expect(permissions.submitQuestion("question-2"), "no choice").toBe(false);
    permissions.guard.setWhole("island", "question-2", false);
    permissions.pick("question-2", "Vue");
    expect(permissions.submitQuestion("question-2"), "not read to the end").toBe(false);
    expect(core.answers).toEqual([]);
  });

  it("know neither « Autoriser » nor « Toujours »: Enter answers, Escape gives the terminal", () => {
    look();
    expect(permissions.respond("question-2", "always")).toBe(false);
    expect(permissions.respond("question-2", "allow"), "Enter with nothing picked").toBe(false);
    permissions.pick("question-2", "date-fns");
    expect(permissions.respond("question-2", "allow")).toBe(true);
    expect(permissions.draft("question-2").step).toBe(1);
    look();
    expect(permissions.respond("question-2", "deny")).toBe(true);
    expect(core.permissions).toEqual([["question-2", "terminal"]]);
    expect(core.answers).toEqual([]);
    expect(banners.current).toMatchObject({ kind: "toast", title: "Réponds dans le terminal" });
  });

  it("are left to the terminal once too late", () => {
    permissions.clear();
    permissions.add({ ...structuredClone(DEMO_QUESTIONS), receivedAt: Date.now() - 60_000 });
    permissions.guard.show({ surface: "island", id: "question-2" }, 0);
    permissions.guard.setWhole("island", "question-2", true);
    return new Promise<void>((done) =>
      setTimeout(() => {
        look();
        permissions.pick("question-2", "Vue");
        expect(permissions.isLate("question-2")).toBe(true);
        expect(permissions.submitQuestion("question-2")).toBe(false);
        done();
      }, 0),
    );
  });
});
