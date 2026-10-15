import { describe, expect, it } from "vitest";
import type { AgentSession } from "./agents";
import { DEFAULT_SETTINGS } from "./ipc";
import { activityContext, commandFor, grammarFor, normalize, usableCommands, wakeWords } from "./voice";

const session = (source: AgentSession["source"], project: string, status: AgentSession["status"], task: string): AgentSession => ({
  id: `${source}:${project}`,
  source,
  project,
  task,
  status,
  activity: { verb: "lance", target: "rm -rf build", added: 3, removed: 1 },
  subagents: [],
});

describe("« OK Mochi »", () => {
  it("is off by default", () => {
    expect(DEFAULT_SETTINGS.voice.listen).toBe(false);
  });

  it("understands its phrases however Windows spells them", () => {
    expect(normalize("Qu’est-ce  qui M'attend")).toBe("qu'est-ce qui m'attend");
    const ready = ["toggle-island", "go-to-alert"] as const;
    expect(commandFor("Montre l'alerte", ready)).toMatchObject({ kind: "action", action: "go-to-alert" });
    expect(commandFor("résume l’activité", ready)).toMatchObject({ kind: "ask" });
    expect(commandFor("arrête d'écouter", ready)).toMatchObject({ kind: "stop" });
    expect(commandFor("commande-moi une pizza", ready)).toBeNull();
  });

  it("never listens for an action that has no real handler yet", () => {
    const ready = ["toggle-island", "go-to-alert"] as const;
    expect(commandFor("coupe le son", ready)).toBeNull();
    expect(usableCommands(ready).some((c) => c.kind === "action" && c.action === "mute")).toBe(false);
    expect(grammarFor("Mochi", ready).phrases).not.toContain("coupe le son");
    expect(grammarFor("Mochi", [...ready, "mute"]).phrases).toContain("coupe le son");
  });

  it("answers to Mochi's own name too", () => {
    expect(wakeWords("Mochi")).toEqual(["OK Mochi"]);
    expect(wakeWords("Pixel")).toEqual(["OK Mochi", "OK Pixel"]);
    expect(wakeWords("R2-D2")).toEqual(["OK Mochi"]);
  });

  it("gives Claude agents, projects, states and counters only", () => {
    const context = activityContext(
      [session("claude-code", "epilot", "active", "corrige le mot de passe de prod"), session("codex", "site", "waiting", "secret")],
      { finished: 2, failures: 1, decisions: 3 },
    );
    expect(context).toBe(
      "Agents : Claude Code au travail sur epilot ; Codex attend une réponse sur site.\nAujourd'hui : 2 tâches terminées, 1 échec, 3 décisions.",
    );
    for (const leak of ["mot de passe", "secret", "rm -rf", "build"]) expect(context).not.toContain(leak);
    expect(activityContext([], { finished: 0, failures: 0, decisions: 1 })).toContain("aucun agent ouvert");
  });
});
