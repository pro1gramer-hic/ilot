import { describe, expect, it } from "vitest";
import { focusIsland, takingFocus } from "./ipc";

describe("taking the keyboard", () => {
  it("counts a blur right after asking as part of it, not a later one", async () => {
    expect(takingFocus()).toBe(false);
    const asked = performance.now();
    await focusIsland();
    expect(takingFocus(asked + 100)).toBe(true);
    expect(takingFocus(asked + 1000)).toBe(false);
  });
});
