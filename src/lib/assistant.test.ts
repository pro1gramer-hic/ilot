import { describe, expect, it } from "vitest";
import type { AgentSession } from "./agents";
import { FOCUS_DEFAULT_MINUTES, normalize, parseCommand, remainingLabel, summarize } from "./assistant";

describe("parseCommand", () => {
  it("understands music requests", () => {
    expect(parseCommand("Mets de la lo-fi")).toEqual({ kind: "music", action: "play" });
    expect(parseCommand("lofi stp")).toEqual({ kind: "music", action: "play" });
    expect(parseCommand("Musique !")).toEqual({ kind: "music", action: "play" });
    expect(parseCommand("pause musique")).toEqual({ kind: "music", action: "pause" });
    expect(parseCommand("Coupe la musique")).toEqual({ kind: "music", action: "pause" });
  });

  it("understands the summary, with or without accents", () => {
    expect(parseCommand("Résume le travail")).toEqual({ kind: "summary" });
    expect(parseCommand("resume")).toEqual({ kind: "summary" });
  });

  it("starts a focus session of the asked length", () => {
    expect(parseCommand("focus 25")).toEqual({ kind: "focus", minutes: 25 });
    expect(parseCommand("pomodoro")).toEqual({ kind: "focus", minutes: FOCUS_DEFAULT_MINUTES });
    expect(parseCommand("Focus 50 minutes")).toEqual({ kind: "focus", minutes: 50 });
    expect(parseCommand("focus 999")).toEqual({ kind: "focus", minutes: 180 });
    expect(parseCommand("arrête le focus")).toEqual({ kind: "stop-focus" });
  });

  it("toggles do not disturb", () => {
    expect(parseCommand("Ne pas déranger")).toEqual({ kind: "dnd" });
  });

  it("admits what it cannot do", () => {
    expect(parseCommand("commande une pizza")).toEqual({ kind: "unknown" });
    expect(parseCommand("")).toEqual({ kind: "unknown" });
  });

  it("normalizes case, accents and spaces", () => {
    expect(normalize("  Résumé   DU  Jour ")).toBe("resume du jour");
  });
});

describe("summarize", () => {
  const session = (status: AgentSession["status"], project: string): AgentSession => ({
    id: project,
    source: "claude-code",
    project,
    task: "",
    status,
  });

  it("says when nothing finished yet", () => {
    expect(summarize({ finished: [], failures: 0 }, [])).toBe("Aucune tâche terminée aujourd'hui pour l'instant.");
  });

  it("counts tasks, lines and time, then what still runs", () => {
    const text = summarize(
      {
        finished: [
          { source: "claude-code", project: "epilot", durationMs: 240_000, added: 12, removed: 3, at: 1 },
          { source: "gemini-cli", project: "site", durationMs: 600_000, added: 48, removed: 0, at: 2 },
        ],
        failures: 1,
      },
      [session("active", "api"), session("done", "epilot")],
    );
    expect(text).toBe(
      "Aujourd'hui : 2 tâches terminées (Claude 1, Gemini 1), 1 échec, 60 lignes ajoutées et 3 retirées, 14 min de travail d'agent. En cours : Claude sur api.",
    );
  });
});

describe("remainingLabel", () => {
  it("shows minutes, then seconds in the last minute", () => {
    expect(remainingLabel(25 * 60_000)).toBe("25 min");
    expect(remainingLabel(17 * 60_000 + 1)).toBe("18 min");
    expect(remainingLabel(45_000)).toBe("45 s");
    expect(remainingLabel(-5)).toBe("0 s");
  });
});
