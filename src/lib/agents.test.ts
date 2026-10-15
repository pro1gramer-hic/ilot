import { describe, expect, it } from "vitest";
import {
  type AgentSession,
  DECISION,
  activityLine,
  addDecision,
  agentState,
  answersInTerminal,
  bySource,
  countBySource,
  decisionsSince,
  dotKey,
  editCounter,
  fitDots,
  followedSession,
  type ShownDot,
  isLive,
  newlyFailed,
  panelMood,
  pillDots,
  plugState,
  sessionColor,
  sessionName,
  sourceColor,
  sourceTool,
  STATUS_TONE,
  sessionTool,
  subagentKinds,
  subagentsLabel,
  subagentsLine,
  summary,
  targetsToFlip,
  tickerText,
} from "./agents";
import type { ConnectorView, Decision } from "./ipc";

const session = (id: string, source: AgentSession["source"], status: AgentSession["status"]): AgentSession => ({
  id,
  source,
  project: "epilot",
  task: "corrige le fuseau horaire",
  status,
});

describe("agents", () => {
  it("counts only working or waiting sessions", () => {
    expect(isLive(session("a", "codex", "active"))).toBe(true);
    expect(isLive(session("a", "codex", "waiting"))).toBe(true);
    expect(isLive(session("a", "codex", "paused"))).toBe(false);
    expect(isLive(session("a", "codex", "done"))).toBe(false);
    expect(isLive(session("a", "codex", "failed"))).toBe(false);
  });

  it("counts sessions per AI and keeps every AI in the result", () => {
    const counts = countBySource([
      session("1", "claude-code", "active"),
      session("2", "claude-code", "waiting"),
      session("3", "gemini-cli", "active"),
      session("4", "codex", "done"),
    ]);
    expect(counts).toEqual({ ...bySource(() => 0), "claude-code": 2, "gemini-cli": 1 });
    expect(Object.keys(counts)).toHaveLength(10);
  });

  it("follows the session that waits for an answer first", () => {
    const sessions = [session("1", "gemini-cli", "active"), session("2", "claude-code", "waiting")];
    expect(followedSession(sessions)?.id).toBe("2");
  });

  it("follows the first live session otherwise", () => {
    const sessions = [session("1", "codex", "done"), session("2", "gemini-cli", "active")];
    expect(followedSession(sessions)?.id).toBe("2");
    expect(followedSession([session("1", "codex", "done")])).toBeUndefined();
  });

  it("summarizes the work in French", () => {
    expect(summary([])).toBe("Tout est calme");
    expect(summary([session("1", "codex", "active")])).toBe("1 agent au travail · Codex 1");
    expect(
      summary([session("1", "claude-code", "active"), session("2", "gemini-cli", "waiting")]),
    ).toBe("2 agents au travail · Claude 1, Gemini 1");
  });

  it("sums up the work in one line for the tooltip", () => {
    expect(activityLine([])).toBe("Personne ne travaille");
    expect(
      activityLine([
        { ...session("1", "claude-code", "active"), project: "epilot" },
        { ...session("2", "gemini-cli", "active"), project: "site" },
        session("3", "codex", "done"),
      ]),
    ).toBe("Claude travaille sur epilot, Gemini sur site");
    expect(
      activityLine([session("1", "gemini-cli", "active"), { ...session("2", "claude-code", "waiting"), project: "api" }]),
    ).toBe("Claude attend ton OK sur api");
  });

  it("counts three AIs and a generic tool at work at once", () => {
    const counts = countBySource([
      session("1", "claude-code", "active"),
      session("2", "gemini-cli", "waiting"),
      session("3", "codex", "active"),
      { ...session("4", "generic", "active"), name: "ci" },
      { ...session("5", "generic", "done"), name: "tests" },
    ]);
    expect(counts).toEqual({ ...bySource(() => 0), "claude-code": 1, "gemini-cli": 1, codex: 1, generic: 1 });
  });

  it("names a generic tool as the user did", () => {
    const ci = { ...session("1", "generic", "active"), name: "ci", project: "site" };
    expect(sessionName(ci)).toBe("ci");
    expect(sessionTool(ci)).toBe("ci");
    expect(sessionName(session("2", "generic", "active"))).toBe("Autre");
    expect(sessionTool(session("3", "gemini-cli", "active"))).toBe("Gemini CLI");
    expect(activityLine([ci])).toBe("ci travaille sur site");
  });

  it("says when Gemini waits for an answer in its own terminal", () => {
    const gemini = { ...session("1", "gemini-cli", "waiting"), project: "site" };
    expect(answersInTerminal(gemini)).toBe(true);
    expect(answersInTerminal(session("2", "codex", "waiting"))).toBe(false);
    expect(answersInTerminal(session("3", "gemini-cli", "active"))).toBe(false);
    expect(activityLine([gemini])).toBe("Gemini attend ton OK sur site, dans son terminal");
  });

  it("gives each state its reserved color, set apart for what waits for the user", () => {
    expect(STATUS_TONE).toEqual({ active: "ok", waiting: "wait", paused: "idle", done: "idle", failed: "err" });
  });
});

const decision = (at: number, command: string, kind: Decision["decision"] = "allow"): Decision => ({
  at,
  source: "claude-code",
  project: "epilot",
  command,
  decision: kind,
});

describe("subagents", () => {
  const three = [
    { id: "a1", kind: "Explore" },
    { id: "a2", kind: "Explore" },
    { id: "a3", kind: "Plan" },
  ];

  it("say how many work, singular and plural", () => {
    expect(subagentsLabel(1)).toBe("1 sous-agent");
    expect(subagentsLabel(3)).toBe("3 sous-agents");
  });

  it("group their kinds and leave unknown ones out", () => {
    expect(subagentKinds(three)).toBe("Explore ×2, Plan");
    expect(subagentKinds([{ id: "x" }])).toBe("");
    expect(subagentsLine(three)).toBe("3 sous-agents : Explore ×2, Plan");
    expect(subagentsLine([{ id: "x" }, { id: "y" }])).toBe("2 sous-agents");
  });

  it("add up per AI over all its sessions", () => {
    const sessions = [
      { ...session("c1", "claude-code", "active"), subagents: three },
      { ...session("c2", "claude-code", "done"), subagents: [{ id: "b1" }] },
      { ...session("x1", "codex", "active"), subagents: [] },
      session("g1", "gemini-cli", "active"),
    ];
    const helpers = Object.fromEntries(pillDots(sessions).map((dot) => [dot.key, dot.subagents]));
    expect(helpers).toEqual({ "claude-code": 4, "gemini-cli": 0, codex: 0 });
  });
});

describe("decisions journal", () => {
  it("keeps the newest first, each decision once, and a bounded list", () => {
    let list = addDecision([], decision(10, "npm test"));
    list = addDecision(list, decision(30, "git status"));
    list = addDecision(list, decision(20, "ls"));
    list = addDecision(list, decision(30, "git status"));
    expect(list.map((d) => d.at)).toEqual([30, 20, 10]);
    expect(addDecision(list, decision(40, "pwd"), 2).map((d) => d.at)).toEqual([40, 30]);
  });

  it("counts the decisions of the day", () => {
    expect(decisionsSince([decision(5, "a"), decision(15, "b"), decision(25, "c")], 10)).toBe(2);
  });

  it("names every verdict, a refusal by suspension included, with its reserved color", () => {
    expect(DECISION.allow).toEqual({ label: "Autorisée", tone: "ok" });
    expect(DECISION.deny).toEqual({ label: "Refusée", tone: "err" });
    expect(DECISION.suspended).toEqual({ label: "Refusée (suspension)", tone: "err" });
    expect(Object.keys(DECISION).sort()).toEqual(["allow", "always", "deny", "expired", "suspended", "terminal"]);
  });

  it("names and colors a source the core sends as text", () => {
    expect(sourceTool("codex")).toBe("Codex");
    expect(sourceColor("gemini-cli")).toBe("var(--ai-gemini)");
    expect(sourceTool("aider")).toBe("aider");
    expect(sourceColor("aider")).toBe("var(--ai-other)");
  });
});

const connector = (...statuses: ConnectorView["targets"][number]["status"][]): ConnectorView => ({
  id: "claude-code",
  name: "Claude",
  caps: { activity: true, permissions: true },
  targets: statuses.map((status, index) => ({ id: `t${index}`, label: `Lieu ${index}`, status })),
});

describe("connection of an AI", () => {
  it("says where an AI stands", () => {
    expect(plugState(undefined)).toBe("absent");
    expect(plugState(connector("notInstalled", "notInstalled"))).toBe("absent");
    expect(plugState(connector("installed", "notInstalled"))).toBe("off");
    expect(plugState(connector("installed", "connected"))).toBe("on");
    expect(plugState(connector("connected", "needsRepair"))).toBe("repair");
  });

  it("plugs every installed place, repairs included, and unplugs every plugged one", () => {
    const view = connector("installed", "connected", "needsRepair", "notInstalled");
    expect(targetsToFlip(view, true)).toEqual(["t0", "t2"]);
    expect(targetsToFlip(view, false)).toEqual(["t1"]);
  });

  it("shows the state of an AI on its card", () => {
    const list = [session("1", "claude-code", "active"), session("2", "gemini-cli", "waiting"), session("3", "codex", "failed")];
    expect(agentState(list, "claude-code", "on", false)).toEqual({ label: "Travail en cours", tone: "ok" });
    expect(agentState(list, "gemini-cli", "on", false)).toEqual({ label: "Attend ton accord", tone: "wait" });
    expect(agentState(list, "codex", "on", false)).toEqual({ label: "Échec", tone: "err" });
    expect(agentState(list, "generic", "on", false)).toEqual({ label: "En veille", tone: "idle" });
    expect(agentState(list, "claude-code", "on", true)).toEqual({ label: "Suspendu", tone: "idle" });
    expect(agentState([], "claude-code", "off", false)).toEqual({ label: "Déconnecté", tone: "idle" });
    expect(agentState([], "claude-code", "absent", false)).toEqual({ label: "Non installé", tone: "idle" });
    expect(agentState(list, "claude-code", "repair", false)).toEqual({ label: "À réparer", tone: "wait" });
    // A tool that reports by itself shows its work, plugged or not.
    expect(agentState(list, "claude-code", "off", false)).toEqual({ label: "Travail en cours", tone: "ok" });
  });

  it("gives Mochi's mood: suspended, then waiting, then a failure", () => {
    const failed = [session("1", "codex", "failed")];
    expect(panelMood([session("1", "codex", "active")], false, false)).toEqual({ label: "Tout roule", tone: "ok" });
    expect(panelMood(failed, false, false)).toEqual({ label: "Un agent a échoué", tone: "err" });
    expect(panelMood(failed, true, false)).toEqual({ label: "Attend ton accord", tone: "wait" });
    expect(panelMood([session("1", "gemini-cli", "waiting")], false, false).label).toBe("Attend ton accord");
    expect(panelMood(failed, true, true)).toEqual({ label: "Suspendu", tone: "idle" });
  });
});

describe("the pill", () => {
  it("tells what each agent does, one at a time (« Activité »)", () => {
    expect(tickerText({ ...session("1", "claude-code", "active"), activity: { verb: "édite", target: "api.py" } })).toBe("édite api.py");
    expect(tickerText({ ...session("1", "claude-code", "active"), activity: { verb: "réfléchit" } })).toBe("réfléchit");
    expect(tickerText(session("1", "codex", "active"))).toBe("travaille sur epilot");
    expect(tickerText(session("1", "codex", "waiting"))).toBe("attend ton accord");
  });

  it("lights each AI's dot: amber while it waits, lit while it works", () => {
    const dots = pillDots([
      session("1", "claude-code", "active"),
      session("2", "claude-code", "waiting"),
      session("3", "codex", "active"),
      session("4", "gemini-cli", "done"),
    ]);
    expect(Object.fromEntries(dots.map((dot) => [dot.key, dot.state]))).toEqual({ "claude-code": "perm", "gemini-cli": "idle", codex: "work" });
  });

  it("gives the three AIs a dot always, any other agent or badge while it is there", () => {
    const qwen = { id: "qwen", name: "Qwen Code", color: "teal" as const };
    const dots = pillDots([
      session("1", "kiro", "active"),
      { ...session("2", "claude-code", "waiting"), agent: qwen },
      session("3", "generic", "done"),
      { ...session("4", "cursor", "active"), subagents: [{ id: "s" }] },
    ]);
    expect(dots.map((dot) => dot.key)).toEqual(["claude-code", "gemini-cli", "codex", "generic", "cursor", "kiro", "agent:qwen"]);
    const badge = dots.at(-1)!;
    expect(badge).toEqual({ key: "agent:qwen", name: "Qwen Code", color: "var(--agent-teal)", state: "perm", subagents: 0 });
    expect(dots.find((dot) => dot.key === "claude-code")!.state, "a badged session is not Claude's").toBe("idle");
    expect(dots.find((dot) => dot.key === "cursor")).toMatchObject({ color: "var(--ai-cursor)", state: "work", subagents: 1 });
  });

  it("keeps the dots that need the user when the pill lacks room, in their order", () => {
    const dot = (key: string, state: ShownDot["state"]): ShownDot => ({ key, name: key, color: "", state, subagents: 0 });
    const dots = [dot("claude-code", "idle"), dot("gemini-cli", "work"), dot("codex", "ok"), dot("cursor", "perm"), dot("kiro", "err")];
    expect(fitDots(dots, 5)).toEqual({ shown: dots, hidden: 0 });
    const { shown, hidden } = fitDots(dots, 4);
    expect(shown.map((d) => d.key)).toEqual(["gemini-cli", "cursor", "kiro"]);
    expect(hidden).toBe(2);
  });

  it("names and colors a session after its badge", () => {
    const badged = { ...session("1", "claude-code", "active"), agent: { id: "qwen", name: "Qwen Code", color: "pink" as const } };
    expect([sessionName(badged), sessionTool(badged), sessionColor(badged), dotKey(badged)]).toEqual([
      "Qwen Code",
      "Qwen Code",
      "var(--agent-pink)",
      "agent:qwen",
    ]);
    expect([sessionName(session("2", "bob", "done")), sessionTool(session("2", "bob", "done"))]).toEqual(["Bob", "IBM Bob"]);
    expect(sessionColor(session("3", "copilot", "active"))).toBe("var(--ai-copilot)");
  });

  it("counts the lines of the file being edited", () => {
    const editing = { ...session("1", "claude-code", "active"), activity: { verb: "édite", target: "api.py", added: 12, removed: 3 } };
    expect(editCounter(editing)).toEqual({ file: "api.py", added: 12, removed: 3 });
    expect(editCounter({ ...editing, activity: { verb: "lance", target: "npm test", added: 12, removed: 3 } })).toBeNull();
    expect(editCounter({ ...editing, activity: { verb: "écrit", target: "notes.md", added: 0, removed: 0 } })).toBeNull();
    expect(editCounter(session("2", "codex", "active"))).toBeNull();
  });

  it("knows who waits in its own terminal", () => {
    expect(answersInTerminal({ ...session("1", "codex", "waiting"), inTerminal: true })).toBe(true);
    expect(answersInTerminal(session("2", "codex", "waiting"))).toBe(false);
    expect(answersInTerminal(session("3", "gemini-cli", "waiting"))).toBe(true);
    expect(answersInTerminal({ ...session("4", "copilot", "active"), inTerminal: true })).toBe(false);
  });

  it("finds the sessions that just failed, once", () => {
    const before = [session("1", "gemini-cli", "active"), session("2", "codex", "failed")];
    const after = [session("1", "gemini-cli", "failed"), session("2", "codex", "failed")];
    expect(newlyFailed(before, after).map((s) => s.id)).toEqual(["1"]);
    expect(newlyFailed(after, after)).toEqual([]);
  });
});
