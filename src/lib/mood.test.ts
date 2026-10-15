import { describe, expect, it } from "vitest";
import type { AgentSession } from "./agents";
import { POKES, reactionTo } from "./mood";

const session = (id: string, status: AgentSession["status"]): AgentSession => ({
  id,
  source: "claude-code",
  project: "epilot",
  task: "",
  status,
});

describe("reactionTo", () => {
  it("bounces when an agent starts working", () => {
    expect(reactionTo([], [session("a", "active")])).toEqual({ move: "bounce" });
    expect(reactionTo([session("a", "done")], [session("a", "active")])).toEqual({ move: "bounce" });
  });

  it("stays calm while an agent keeps working or resumes after a permission", () => {
    expect(reactionTo([session("a", "active")], [session("a", "active")])).toBeNull();
    expect(reactionTo([session("a", "waiting")], [session("a", "active")])).toBeNull();
  });

  it("worries when an agent fails", () => {
    expect(reactionTo([session("a", "active")], [session("a", "failed")])).toEqual({ face: "worry" });
    expect(reactionTo([session("a", "failed")], [session("a", "failed")])).toBeNull();
  });
});

describe("POKES", () => {
  it("offers different short phrases", () => {
    expect(new Set(POKES.map((p) => p.text)).size).toBe(POKES.length);
    for (const poke of POKES) expect(poke.text.length).toBeLessThanOrEqual(40);
  });
});
