import { describe, expect, it } from "vitest";
import { AnswerGuard, LOOK_BEFORE_ANSWER_MS } from "./guard";

const LOOK = LOOK_BEFORE_ANSWER_MS;

describe("AnswerGuard", () => {
  it("waits for the look, then lets the shown request be answered", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "island", id: "a" }, 1000);
    guard.setWhole("island", "a", true);
    expect(guard.ready("a", "allow", 1000 + LOOK - 1)).toBe(false);
    expect(guard.ready("a", "allow", 1000 + LOOK)).toBe(true);
    expect(guard.ready("b", "deny", 1000 + LOOK), "only the request on screen").toBe(false);
  });

  it("starts again for the next request: a double click never reaches it", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "island", id: "a" }, 0);
    guard.setWhole("island", "a", true);
    guard.setWhole("island", "b", true);
    guard.show({ surface: "island", id: "b" }, 5000);
    expect(guard.ready("b", "allow", 5000 + 80)).toBe(false);
    expect(guard.ready("b", "allow", 5000 + LOOK)).toBe(true);
  });

  it("starts again when the same request moves to the panel", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "island", id: "a" }, 0);
    guard.show({ surface: "panel", id: "a" }, 3000);
    guard.setWhole("panel", "a", true);
    expect(guard.ready("a", "allow", 3000 + 10)).toBe(false);
    expect(guard.ready("a", "allow", 3000 + LOOK)).toBe(true);
  });

  it("keeps the first time when the same request is shown again", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "panel", id: "a" }, 0);
    guard.show({ surface: "panel", id: "a" }, 900);
    guard.setWhole("panel", "a", true);
    expect(guard.ready("a", "allow", LOOK)).toBe(true);
  });

  it("allows only what was shown whole; refusing needs the look alone", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "panel", id: "a" }, 0);
    expect(guard.ready("a", "allow", LOOK)).toBe(false);
    expect(guard.ready("a", "always", LOOK)).toBe(false);
    expect(guard.ready("a", "deny", LOOK)).toBe(true);
    expect(guard.ready("a", "terminal", LOOK)).toBe(true);
    expect(guard.ready("a", "deny", LOOK - 1)).toBe(false);
  });

  it("starts again for the next question of the same request", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "island", id: "q" }, 0);
    guard.setWhole("island", "q", true);
    expect(guard.ready("q", "allow", LOOK)).toBe(true);
    guard.restart("q", LOOK);
    expect(guard.ready("q", "allow", LOOK + 80), "a double Enter never answers the next question").toBe(false);
    expect(guard.ready("q", "allow", 2 * LOOK)).toBe(true);
    guard.restart("other", 10 * LOOK);
    expect(guard.ready("q", "allow", 2 * LOOK), "only the request on screen restarts").toBe(true);
  });

  it("forgets a request once it is gone, and nothing is ready off screen", () => {
    const guard = new AnswerGuard();
    guard.show({ surface: "island", id: "a" }, 0);
    guard.setWhole("island", "a", true);
    guard.forget("a");
    expect(guard.isWhole("island", "a")).toBe(false);
    guard.show(undefined, 0);
    expect(guard.ready("a", "deny", 10 * LOOK)).toBe(false);
  });
});
