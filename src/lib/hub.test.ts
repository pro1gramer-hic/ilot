import { describe, expect, it } from "vitest";
import type { AgentSession } from "./agents";
import {
  DIZZY_SECONDS,
  type HubStage,
  INTRO_MS,
  LIVE_MS,
  POKE_WINDOW_MS,
  canFollow,
  focusKind,
  frameSteps,
  hubSessions,
  introAt,
  liveFrame,
  liveSteps,
  mochiFor,
  pickFollowed,
  poke,
  sessionPalette,
  stepCount,
  stepOfLine,
  stepOfActivity,
  tileLabel,
  tileSubagents,
  tileTip,
  visibleTiles,
  workingLabel,
} from "./hub";
import { EDGE, HUB_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH, hubBox, hubHeight } from "./island/model";
import type { Track } from "./media";

const agent = (id: string, source: AgentSession["source"], status: AgentSession["status"], verb?: string): AgentSession => ({
  id,
  source,
  project: id,
  task: "corrige le fuseau horaire",
  status,
  ...(verb ? { activity: { verb, target: "api.py" } } : {}),
});

const track: Track = {
  id: "t",
  title: "Lo-fi beats",
  artist: "Lofi Girl",
  source: "Spotify",
  app: "Spotify",
  appId: "Spotify.exe",
  playing: true,
  position: 0,
  duration: 100,
  updatedAt: 0,
  controls: { playPause: true, next: true, previous: true },
};

describe("hub sessions", () => {
  it("lists the agents in the store's order, then the music", () => {
    const list = hubSessions([agent("epilot", "claude-code", "active"), agent("rpg", "gemini-cli", "failed")], track);
    expect(list.map((s) => [s.key, s.status, s.palette])).toEqual([
      ["agent:epilot", "work", "peche"],
      ["agent:rpg", "err", "ciel"],
      ["music", "play", "citron"],
    ]);
    expect(list[0].tool).toBe("Claude Code");
    expect(list[2].tool).toBe("Spotify");
    expect(hubSessions([], { ...track, playing: false })[0].status).toBe("idle");
    expect(hubSessions([], null)).toEqual([]);
  });

  it("maps every agent state", () => {
    const states = (["active", "waiting", "failed", "done", "paused"] as const).map(
      (status) => hubSessions([agent("a", "codex", status)], null)[0].status,
    );
    expect(states).toEqual(["work", "perm", "err", "ok", "idle"]);
  });

  it("follows the chosen session while it is there, else what needs the user first", () => {
    const list = hubSessions(
      [agent("done", "codex", "done"), agent("fail", "gemini-cli", "failed"), agent("work", "claude-code", "active")],
      track,
    );
    expect(pickFollowed(list, null)?.key).toBe("agent:work");
    expect(pickFollowed(list, "music")?.key).toBe("music");
    expect(pickFollowed(list, "agent:gone")?.key).toBe("agent:work");
    const waiting = hubSessions([agent("work", "claude-code", "active"), agent("ask", "codex", "waiting")], null);
    expect(pickFollowed(waiting, null)?.key).toBe("agent:ask");
    expect(pickFollowed([], null)).toBeUndefined();
  });

  it("keeps at most five icons, the followed one first, in their own order", () => {
    const many = hubSessions(
      [
        agent("a", "codex", "done"),
        agent("b", "codex", "paused"),
        agent("c", "claude-code", "active"),
        agent("d", "gemini-cli", "failed"),
        agent("e", "codex", "done"),
        agent("f", "codex", "waiting"),
      ],
      track,
    );
    const shown = visibleTiles(many, "agent:b").map((s) => s.key);
    expect(shown).toEqual(["agent:b", "agent:c", "agent:d", "agent:f", "music"]);
    expect(visibleTiles(many.slice(0, 3), undefined)).toHaveLength(3);
  });

  it("names each icon for the tooltip and for screen readers", () => {
    const [claude] = hubSessions([agent("epilot", "claude-code", "active")], null);
    expect(tileTip(claude)).toBe("epilot, travail en cours");
    expect(tileLabel(claude)).toBe("epilot, Claude Code, travail en cours");
    const [busy] = hubSessions([{ ...agent("epilot", "claude-code", "active"), subagents: [{ id: "a1" }, { id: "a2" }] }], null);
    expect(tileSubagents(busy)).toBe(2);
    expect(tileTip(busy)).toBe("epilot, travail en cours, 2 sous-agents");
    expect(tileSubagents(hubSessions([], track)[0])).toBe(0);
  });

  it("says how many agents work", () => {
    expect(workingLabel(0, false)).toBe("Aucun agent au travail");
    expect(workingLabel(1, false)).toBe("1 agent au travail");
    expect(workingLabel(3, false)).toBe("3 agents au travail");
    expect(workingLabel(3, true)).toBe("Agents suspendus");
  });

  it("gives Mochi a move and a face by state", () => {
    expect(mochiFor("err")).toEqual({ move: "shake", face: "worry" });
    expect(mochiFor("play")).toEqual({ move: "rock", face: "happy" });
    expect(mochiFor("work")).toEqual({ move: "breathe" });
  });

  it("chooses what the left card says", () => {
    const kinds = [
      agent("a", "claude-code", "active", "édite"),
      agent("b", "codex", "active", "lance"),
      agent("c", "codex", "waiting"),
      agent("d", "codex", "failed"),
      agent("e", "codex", "done"),
      agent("f", "codex", "paused"),
    ].map((s) => focusKind(hubSessions([s], null)[0]));
    expect(kinds).toEqual(["action", "command", "waiting", "failed", "done", "idle"]);
    const music = hubSessions([], track)[0];
    expect(focusKind(music)).toBe("music");
    expect(canFollow(music)).toBe(false);
    expect(canFollow(hubSessions([agent("a", "codex", "active")], null)[0])).toBe(true);
    expect(canFollow(hubSessions([agent("a", "codex", "done")], null)[0])).toBe(false);
  });
});

describe("live steps", () => {
  it("reads the step of the core's actions, of its verbs and of journal lines", () => {
    const step = (activity: { action?: "read" | "edit" | "write" | "run" | "use"; verb: string }) => stepOfActivity(activity);
    expect([step({ action: "read", verb: "reads" }), step({ action: "edit", verb: "edits" }), step({ action: "write", verb: "writes" })]).toEqual([0, 1, 1]);
    expect([step({ action: "run", verb: "runs" }), step({ action: "use", verb: "uses" })]).toEqual([2, null]);
    // Without its action (an older core), the French verb still tells the step.
    expect([step({ verb: "lit" }), step({ verb: "édite" }), step({ verb: "lance" })]).toEqual([0, 1, 2]);
    expect(stepOfActivity(null)).toBeNull();
    expect(stepOfLine("Édite api.py")).toBe(1);
    expect(stepOfLine("Lance npm run test, autorisé par toi")).toBe(2);
    expect(stepOfLine("A terminé : refonte des tokens")).toBeNull();
  });

  it("ticks only the steps the agent went through, and all of them once done", () => {
    const editing = agent("a", "claude-code", "active", "édite");
    expect(liveSteps(editing, ["Édite api.py", "Lit api.py"])).toEqual(["done", "now", "todo", "todo"]);
    expect(liveSteps(editing, [])).toEqual(["todo", "now", "todo", "todo"]);
    expect(liveSteps(agent("a", "codex", "done"), [])).toEqual(["done", "done", "done", "done"]);
    expect(liveSteps(agent("a", "codex", "waiting", "lance"), ["Lance npm test"])).toEqual(["todo", "todo", "done", "todo"]);
    expect(stepCount(["done", "now", "todo", "todo"])).toBe("2/4");
    expect(stepCount(["done", "done", "done", "done"])).toBe("4/4");
    expect(stepCount(["todo", "todo", "done", "todo"])).toBe("1/4");
  });
});

describe("too many clicks", () => {
  it("squashes, then annoys, then makes Mochi dizzy at the fifth click within 1.6 s", () => {
    let times: number[] = [];
    const reactions = [0, 200, 400, 600, 800].map((at) => {
      const result = poke(times, at);
      times = result.times;
      return result.reaction;
    });
    expect(reactions).toEqual(["squish", "squish", "annoyed", "annoyed", "dizzy"]);
    expect(times).toEqual([]);
    expect(DIZZY_SECONDS).toBe(3);
  });

  it("forgets clicks older than 1.6 s", () => {
    const four = [0, 500, 1000, 1500].reduce<number[]>((acc, at) => poke(acc, at).times, []);
    expect(poke(four, 1590).reaction).toBe("dizzy");
    expect(poke(four, 1500 + POKE_WINDOW_MS)).toEqual({ times: [1500 + POKE_WINDOW_MS], reaction: "squish" });
  });
});

describe("intro", () => {
  it("wakes Mochi, throws the ball, then sways", () => {
    expect(introAt(0)).toEqual({ phase: "sky", move: "still", face: "sleep" });
    expect(introAt(950).face).toBe("blink");
    expect(introAt(1200)).toEqual({ phase: "breathe", move: "breathe" });
    expect(introAt(2000).phase).toBe("ball");
    expect(introAt(3000)).toEqual({ phase: "catch", move: "bounce", face: "happy" });
    expect(introAt(4000).face).toBe("wink");
    expect(introAt(INTRO_MS).phase).toBe("done");
  });
});

describe("staged live session", () => {
  const stage: HubStage = {
    sessionId: "c1",
    file: "api.py",
    path: "epilot/api.py",
    lang: "PY",
    lines: [1, 2, 3].map((n) => ({ n, tokens: [["", `ligne ${n}`]] as const })),
    edits: [{ line: 2, text: "abc" }],
    snippet: [1, 2],
    terminal: [{ text: "pytest", tone: "prompt" }, { text: "ok", tone: "ok" }],
  };
  const readEnd = 3 * LIVE_MS.readLine;
  const typeAt = readEnd + LIVE_MS.strike;
  const terminalAt = typeAt + 3 * LIVE_MS.typeChar + LIVE_MS.beforeTerminal;
  const doneAt = terminalAt + LIVE_MS.prompt + LIVE_MS.line;

  it("reads line by line, then strikes and types the new line", () => {
    expect(liveFrame(stage, 0).reading).toBe(1);
    expect(liveFrame(stage, LIVE_MS.readLine).reading).toBe(2);
    const struck = liveFrame(stage, readEnd);
    expect([struck.step, struck.reading, struck.edits[0]]).toEqual([1, null, { line: 2, struck: true, typed: null }]);
    expect(liveFrame(stage, typeAt).edits[0].typed).toBe("a");
    expect(liveFrame(stage, typeAt + LIVE_MS.typeChar).edits[0].typed).toBe("ab");
    expect(liveFrame(stage, typeAt, true).edits[0].typed).toBe("abc");
  });

  it("opens the terminal, prints its lines, ends and lets the party run", () => {
    const opening = liveFrame(stage, terminalAt);
    expect([opening.step, opening.terminalOpen, opening.terminalLines]).toEqual([2, true, 1]);
    expect(liveFrame(stage, terminalAt + LIVE_MS.prompt).terminalLines).toBe(2);
    const done = liveFrame(stage, doneAt);
    expect([done.step, done.done, done.over]).toEqual([3, true, false]);
    expect(frameSteps(done)).toEqual(["done", "done", "done", "done"]);
    expect(frameSteps(opening)).toEqual(["done", "done", "now", "todo"]);
    expect(liveFrame(stage, doneAt + LIVE_MS.party).over).toBe(true);
  });
});

describe("hub box", () => {
  const pill = { x: 380, y: 8, width: 220, height: 40 };

  it("sits under the island, centered on it, 16 px away", () => {
    expect(hubBox(pill, true, hubHeight("home"))).toEqual({ x: 130, y: 64, width: HUB_WIDTH, height: 250 });
    expect(hubBox(pill, true, hubHeight("live")).height).toBe(420);
    expect(hubHeight("dizzy")).toBe(140);
  });

  it("follows a taller opening, goes above when the island opens up, and stays in the window", () => {
    expect(hubBox({ ...pill, height: 64 }, true, 250).y).toBe(88);
    const low = { x: 700, y: 660, width: 220, height: 40 };
    expect(hubBox(low, false, 250)).toEqual({ x: WINDOW_WIDTH - HUB_WIDTH - EDGE, y: 394, width: HUB_WIDTH, height: 250 });
    expect(hubBox({ ...pill, y: 400 }, true, 420).height).toBe(WINDOW_HEIGHT - 456 - EDGE);
  });
});

describe("the hub's tiles of the agents of lot 2", () => {
  it("give each followed agent and each badge its own Mochi and color", () => {
    const list = hubSessions(
      [
        agent("site", "cursor", "active", "édite"),
        agent("infra", "kiro", "waiting"),
        { ...agent("data", "claude-code", "active", "lit"), agent: { id: "qwen", name: "Qwen Code", color: "teal" } },
      ],
      null,
    );
    expect(list.map((s) => [s.tool, s.palette, s.color])).toEqual([
      ["Cursor", "ivoire", "var(--ai-cursor)"],
      ["Kiro", "glycine", "var(--ai-kiro)"],
      ["Qwen Code", "lagon", "var(--agent-teal)"],
    ]);
    expect(sessionPalette({ source: "amp" })).toBe("fuchsia");
  });
});
