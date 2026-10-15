import { describe, expect, it } from "vitest";
import { MAX_OTHER_CHARS, NEW_DRAFT, type Question, answerOf, pick, stepLabel, type } from "./questions";

// The documented AskUserQuestion example (https://code.claude.com/docs/en/hooks#askuserquestion).
const single: Question = {
  question: "Which framework?",
  header: "Framework",
  options: [
    { label: "React", description: "Component library" },
    { label: "Vue", description: "Progressive framework" },
  ],
  multiSelect: false,
  freeText: true,
};
const multiple: Question = { ...single, question: "Which checks?", options: [{ label: "Tests" }, { label: "Lint" }, { label: "Types" }], multiSelect: true };

describe("questions", () => {
  it("has no answer until the user picks one", () => {
    expect(answerOf(NEW_DRAFT, single)).toBeNull();
    expect(answerOf(pick(NEW_DRAFT, single, "Vue"), single)).toEqual({ picked: ["Vue"] });
  });

  it("keeps one choice for a simple question, several for a multiple one", () => {
    const twice = pick(pick(NEW_DRAFT, single, "React"), single, "Vue");
    expect(twice.picked).toEqual(["Vue"]);
    let draft = pick(pick(NEW_DRAFT, multiple, "Types"), multiple, "Tests");
    expect(answerOf(draft, multiple), "in the order offered").toEqual({ picked: ["Tests", "Types"] });
    draft = pick(draft, multiple, "Types");
    expect(answerOf(draft, multiple)).toEqual({ picked: ["Tests"] });
    expect(pick(NEW_DRAFT, single, "Angular"), "only what is offered").toEqual(NEW_DRAFT);
  });

  it("takes a typed answer where the agent allows one", () => {
    const typed = type(pick(NEW_DRAFT, single, "React"), single, "  Svelte ");
    expect(answerOf(typed, single)).toEqual({ picked: [], other: "Svelte" });
    expect(answerOf(type(pick(NEW_DRAFT, multiple, "Lint"), multiple, "Perf"), multiple)).toEqual({ picked: ["Lint"], other: "Perf" });
    expect(type(NEW_DRAFT, { ...single, freeText: false }, "x")).toEqual(NEW_DRAFT);
    expect(type(NEW_DRAFT, single, "y".repeat(900)).other).toHaveLength(MAX_OTHER_CHARS);
    expect(answerOf(type(NEW_DRAFT, single, "   "), single)).toBeNull();
  });

  it("says where the user is", () => {
    expect(stepLabel(1, 3)).toBe("Question 2 sur 3");
    expect(stepLabel(0, 1)).toBe("");
  });
});
