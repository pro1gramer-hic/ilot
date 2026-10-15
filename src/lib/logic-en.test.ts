import { describe, expect, it, vi } from "vitest";

// The logic modules in English (prompt 12, lot 5), as the browser preview shows them with
// `?lang=en`: the same rules as their French tests, with the words of the English catalog.
// French stays the language of every other test.

vi.stubGlobal("location", { search: "?lang=en" });

const { i18n } = await import("./i18n/i18n.svelte");
const agents = await import("./agents");
const hub = await import("./hub");
const timeline = await import("./timeline");
const report = await import("./report");
const notifications = await import("./notifications");
const banners = await import("./island/banners");
const { TABS } = await import("./island/model");
const media = await import("./media");
const pastilles = await import("./pastilles");
const shortcuts = await import("./shortcuts");
const { MODES, DEFAULT_MODES } = await import("./modes");
const { THEMES } = await import("./themes");
const voice = await import("./voice");
const { HEAD_LABELS } = await import("./mochi/faces");
const { KIND_LABELS } = await import("./mochi/kinds");
const { PALETTES } = await import("./mochi/palettes");
const { ANIMATIONS } = await import("./mochi/moves");
const { PIECES, SEASONS, SLOTS } = await import("./mochi/outfits");
const { POKES } = await import("./mood");
const { SOUNDS } = await import("./sound/catalog");
const chat = await import("./chat");
const { stepLabel } = await import("./questions");
const assistant = await import("./assistant");
const { DEFAULT_SETTINGS } = await import("./ipc");

type Session = import("./agents").AgentSession;

const session = (over: Partial<Session> = {}): Session => ({
  id: "s1",
  source: "claude-code",
  project: "epilot",
  task: "refactor the tokens",
  status: "active",
  ...over,
});

const MIN = 60_000;

describe("the logic modules in English", () => {
  it("speak English when the address asks for it", () => {
    expect(i18n.locale).toBe("en");
  });

  it("name the agents' states, decisions and activity", () => {
    expect(agents.STATUS_LABEL.waiting).toBe("waiting for your OK");
    expect(agents.AI.generic.name).toBe("Other");
    expect(agents.AI.generic.tool).toBe("Custom tool");
    expect(agents.DECISION.deny.label).toBe("Denied");
    expect(agents.summary([session(), session({ id: "s2", source: "gemini-cli" })])).toBe("2 agents at work · Claude 1, Gemini 1");
    expect(agents.summary([])).toBe("All quiet");
    expect(agents.activityLine([session({ status: "waiting", source: "gemini-cli" })])).toBe(
      "Gemini is waiting for your OK on epilot, in its terminal",
    );
    expect(agents.activityLine([session(), session({ id: "s2", source: "codex", project: "site" })])).toBe(
      "Claude is working on epilot, Codex on site",
    );
    expect(agents.tickerText(session({ activity: { verb: "édite", target: "api.py" } }))).toBe("edits api.py");
    expect(agents.tickerText(session({ activity: { verb: "inconnu" } }))).toBe("inconnu");
    expect(agents.subagentsLine([{ id: "a", kind: "Explore" }, { id: "b", kind: "Explore" }])).toBe("2 subagents: Explore ×2");
    expect(agents.agentState([], "codex", "absent", false).label).toBe("Not installed");
    expect(agents.panelMood([], false, false).label).toBe("All good");
  });

  it("follow a session in the hub, with its steps", () => {
    expect(hub.workingLabel(1, false)).toBe("1 agent at work");
    expect(hub.workingLabel(0, false)).toBe("No agent at work");
    expect([...hub.LIVE_STEPS]).toEqual(["Read", "Edit", "Command", "Done"]);
    expect(hub.stepOfLine("Edits api.py")).toBe(1);
    expect(hub.stepOfActivity({ action: "run", verb: "runs" })).toBe(2);
    const [tile] = hub.hubSessions([session()], null);
    expect(hub.tileLabel(tile)).toBe("epilot, Claude Code, working");
  });

  it("write the journal, durations and clock times", () => {
    expect(timeline.stepText(session({ status: "done" }))).toBe("Finished: refactor the tokens");
    expect(timeline.doing(session({ activity: { verb: "lance", target: "npm test" } }))).toBe("Runs npm test");
    expect(timeline.durationLabel(65 * MIN)).toBe("1h 05");
    expect(timeline.clockLabel(new Date(2026, 9, 12, 14, 7).getTime())).toMatch(/^2:07\sPM$/);
    expect(banners.formatDuration(30_000)).toBe("less than a minute");
  });

  it("say the Monday report and draw its words", () => {
    expect(report.weekRange("2026-10-05")).toMatch(/^October 5\s–\s11, 2026$/);
    expect(report.weekRange("2026-09-28")).toMatch(/^September 28\s–\sOctober 4, 2026$/);
    expect(report.count(1204)).toBe("1,204");
    expect([...report.DAY_NAMES]).toEqual(["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]);
    expect(report.DAY_LETTERS.join("")).toBe("MTWTFSS");
    const week = {
      monday: "2026-10-05",
      days: [],
      total: { activeMs: 0, sessions: 3, files: 0, added: 0, removed: 0, commands: 0, allowed: 1, denied: 2 },
      agents: [],
      projects: [{ name: "epilot", ms: MIN }],
      busiestDay: null,
      longest: null,
    };
    const figures = new Map(report.figures(week, true).map((f) => [f.id, f]));
    expect(figures.get("time")?.label).toBe("Coding time");
    expect(figures.get("permissions")?.value).toBe("1 allowed");
    expect(figures.get("agent")?.value).toBe("None");
    expect(figures.get("project")?.value).toBe("Project A");
    expect(report.imageName(week)).toBe("ilot week 2026-10-05");
  });

  it("tell the messages, with English punctuation", () => {
    const item = { id: "n", app: "whatsapp", appLabel: "WhatsApp", monogram: "W", sender: "Mom", text: "Coming home?", masked: false, count: 1, receivedAt: 0 };
    expect(notifications.bannerParts(item).map((p) => p.text).join("")).toBe("Mom: Coming home?");
    expect(notifications.bannerParts({ ...item, masked: true }).map((p) => p.text).join("")).toBe("New message from Mom");
    expect(notifications.bannerParts({ ...item, count: 3 }).map((p) => p.text).join("")).toBe("WhatsApp · 3 new messages");
    expect(notifications.quickReplies("discord")).toEqual(["I'll look tonight", "Send the link", "Not before tomorrow"]);
    expect(notifications.quickReplies("signal")[0]).toBe("Okay");
    expect(notifications.replyDone("WhatsApp", true)).toBe("Reply copied: paste it in WhatsApp");
  });

  it("word the island's openings", () => {
    const finished = { source: "claude-code" as const, project: "epilot", durationMs: 4 * MIN, added: 1, removed: 0 };
    expect(banners.finishedText([finished])).toBe("Claude finished · epilot · 4 min");
    expect(banners.finishedText([finished, finished])).toBe("2 agents finished");
    expect(banners.answerToast("deny", "Claude Code", "rm -rf")).toMatchObject({ title: "Command denied", sub: "Claude Code has been told." });
    expect(banners.dropToast("C:\\notes\\plan.md").title).toBe("plan.md: path copied");
    expect(TABS.map((tab) => tab.label).slice(0, 4)).toEqual(["Today", "Agents", "Messages", "Music"]);
  });

  it("say what plays", () => {
    const track = { id: "t", title: "Lofi", artist: "", source: "YouTube" as const, app: "Edge", appId: "edge", playing: true, position: 12, duration: 0, updatedAt: 0, controls: { playPause: true, next: false, previous: false } };
    expect(media.timeLabel(track, 0)).toBe("0:12 · live");
    expect(media.tabStatus([track], "YouTube")).toBe("YouTube: playing");
    expect(media.tabStatus([], "Spotify")).toBe("Spotify: nothing playing");
    expect(media.emptyState("Spotify").title).toBe("Spotify isn't playing anything");
    expect(media.repeatLabel("list")).toBe("Repeat all");
    expect(media.detachLabel(track)).toBe("Detach the video");
  });

  it("tell the pastilles' times, sizes and rules", () => {
    const now = Date.now();
    expect(pastilles.relativeTime(now - 5 * MIN, now)).toBe("5 min ago");
    expect(pastilles.relativeTime(now + 30 * 60 * MIN, now)).toBe("tomorrow");
    expect(pastilles.formatBytes(6_591_830_464)).toBe("6.1 GB");
    expect(pastilles.formatBytes(0)).toBe("0 MB");
    expect(pastilles.toolLine("VS Code", { open: true, focused: true, candidates: [] }, "epilot", 2)).toBe("VS Code on epilot, 2 agents at work");
    expect(pastilles.spotifyView(null).line).toBe("Spotify isn't playing anything");
    expect(pastilles.checkBaseUrl("http://example.com")).toEqual({ error: "Unencrypted address: only http://localhost is accepted, otherwise https://" });
    expect(pastilles.LOCALS[0].hint).toBe("Models loaded and installed, on this PC.");
  });

  it("name the shortcuts' actions and keys", () => {
    expect(shortcuts.ACTIONS[0].label).toBe("Open the island");
    expect(shortcuts.keyCaps("Ctrl+Shift+Space")).toEqual(["Ctrl", "Shift", "Space"]);
    expect(shortcuts.assignProblem(DEFAULT_SETTINGS, "open-chat", DEFAULT_SETTINGS.shortcut)).toBe("Already used by “Open the island”.");
    expect(shortcuts.shortcutProblem("A")).toBe("Add Ctrl or Alt: on its own, this key would stop working elsewhere.");
  });

  it("name the modes and themes, with the same ids", () => {
    expect(MODES.music.label).toBe("Music in the folded island");
    expect(MODES.relief.options.map(([value, label]) => `${value}:${label}`)).toEqual(["3d:3D", "leger:Light", "plat:Flat"]);
    expect(DEFAULT_MODES.music).toBe("titre");
    expect(THEMES.find((theme) => theme.id === "obsidienne")?.label).toBe("Obsidian");
  });

  it("listen for English phrases and tell the core so", () => {
    const ready = ["go-to-alert", "mute"] as const;
    const grammar = voice.grammarFor("Mochi", ready);
    expect(grammar.lang).toBe("en");
    expect(grammar.phrases).toContain("show the alert");
    expect(grammar.phrases).not.toContain("montre l'alerte");
    expect(voice.commandFor("What's waiting for me", ready)).toMatchObject({ kind: "action", reply: "Here's what's waiting for you." });
    expect(voice.commandFor("what’s new", ready)).toMatchObject({ kind: "ask", question: "Sum up my activity." });
    expect(voice.activityContext([session()], { finished: 1, failures: 0, decisions: 2 })).toBe(
      "Agents: Claude Code at work on epilot.\nToday: 1 task done, 0 failures, 2 decisions.",
    );
  });

  it("name Mochi's looks, outfits, words and sounds", () => {
    expect(HEAD_LABELS.wink).toBe("Mischievous");
    expect(KIND_LABELS.meche).toBe("Tuft");
    expect(PALETTES.lavande.name).toBe("Lavender");
    expect(ANIMATIONS[1].label).toBe("Breathing");
    expect(PIECES.noel.label).toBe("Santa hat");
    expect(SEASONS.ete.label).toBe("Summer");
    expect(SLOTS[0].label).toBe("Head");
    expect(POKES[0].text).toBe("Hey! That tickles!");
    expect(SOUNDS.finish.label).toBe("Task done");
  });

  it("word the chat, the questions and the request bar", () => {
    expect(chat.destination(chat.providerInfo("ollama"))).toBe("Ollama answers on this PC, over localhost. Nothing leaves.");
    expect(chat.providerInfo("lmstudio").maker).toBe("this PC");
    expect(chat.modelText("")).toBe("Pick a model");
    expect(stepLabel(1, 3)).toBe("Question 2 of 3");
    expect(assistant.remainingLabel(45_000)).toBe("45 s");
    expect(assistant.summarize({ finished: [], failures: 0 }, [])).toBe("No task finished today so far.");
  });
});
