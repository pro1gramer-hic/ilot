// Browser preview: the island with fake data on a desktop-like backdrop, driven by URL
// parameters (?state=banner-finished&tab=agents&theme=light&agents=trio). Playwright uses
// it to capture every state; window.__ilot lets a test move between states. `?date=` sets
// the day the wardrobe's seasons read (2026-12-24 shows Noël).

import { flushSync } from "svelte";
import { AI_SOURCES, type AgentSession, type AiSource } from "./agents";
import {
  DEMO_BANNERS,
  DEMO_CODEX_PERMISSION,
  DEMO_CONNECTORS,
  DEMO_CONVERSATION,
  DEMO_DECISIONS,
  DEMO_DIFF,
  DEMO_FAILED,
  DEMO_RISKY_PERMISSION,
  DEMO_FINISHED,
  DEMO_HUB,
  DEMO_HUB_STAGE,
  DEMO_MANY,
  DEMO_NOTIFICATION_RULES,
  DEMO_PERMISSION,
  DEMO_POSTPONED,
  DEMO_AGENT_QUESTION,
  DEMO_QUESTIONS,
  DEMO_SESSIONS,
  DEMO_SHORT_PERMISSION,
  DEMO_TIMELINE,
  DEMO_TRIO,
  DEMO_WEEK,
} from "./fixtures";
import { NO_CLAUDE_KEY } from "./ipc";
import { paintReport } from "./report-image";
import { actions } from "./stores/actions.svelte";
import { report } from "./stores/report.svelte";
import { voice } from "./stores/voice.svelte";
import { type ConnectorView, type TerminalOutcome, setPreviewAgents } from "./ipc";
import { DEFAULT_THEME, THEMES, type ThemeId } from "./themes";
import { island } from "./island/controller.svelte";
import { TABS, type Tab } from "./island/model";
import { type MediaScene, readScene, sceneSnapshot, startFakeCore, withPlayModes } from "./media-preview";
import { readPastillePreview, sceneSettings, startFakePastilles } from "./pastilles-preview";
import { type Conversation, addReply, conversationKey, markConversationRead, receiveMessage } from "./notifications";
import { answerToast } from "./island/banners";
import { DEMO_ANSWER, DEMO_MODELS, DEMO_QUESTION, DEMO_WINDOW, type FakeFeatures, showFakeWindow, startFakeFeatures } from "./chat-preview";
import type { Attachment } from "./ipc";
import { banners } from "./stores/banners.svelte";
import { chat } from "./stores/chat.svelte";
import { conversations } from "./stores/conversations.svelte";
import { mail } from "./stores/mail.svelte";
import { decisions } from "./stores/decisions.svelte";
import { history } from "./stores/history.svelte";
import { media } from "./stores/media.svelte";
import { notifications } from "./stores/notifications.svelte";
import { pastilles } from "./stores/pastilles.svelte";
import { permissions } from "./stores/permissions.svelte";
import { rest } from "./stores/rest.svelte";
import { sessions } from "./stores/sessions.svelte";
import { settings } from "./stores/settings.svelte";
import { sound } from "./stores/sound.svelte";
import { timeline } from "./stores/timeline.svelte";
import { translations } from "./stores/translate.svelte";
import { wardrobe } from "./stores/wardrobe.svelte";
import type { Lane, Timeline } from "./timeline";

const MINUTE_MS = 60_000;

/** The AIs of the scenes made before the lot 2 agents: their fake hour always shows. */
const ORIGINAL_SOURCES: readonly AiSource[] = ["claude-code", "gemini-cli", "codex", "generic"];

/** The fake hour of every AI (fixtures, in minutes ago): an AI that still works keeps
 * its last stretch open. */
function demoTimeline(now: number, list: readonly AgentSession[]): Timeline {
  const ago = (minutes: number): number => now - minutes * MINUTE_MS;
  const working = (source: string): boolean => list.some((s) => s.source === source && s.status === "active");
  // The first four AIs keep their fake hour; any other one only in a scene where it works.
  const shown = (source: AiSource): boolean => ORIGINAL_SOURCES.includes(source) || list.some((s) => s.source === source);
  const lanes = Object.fromEntries(
    AI_SOURCES.map((source): [string, Lane] => {
      const { segments, marks, steps } = (shown(source) ? DEMO_TIMELINE[source] : undefined) ?? { segments: [], marks: [], steps: [] };
      return [
        source,
        {
          segments: segments.map(([from, to, label], index) => ({
            start: ago(from),
            end: working(source) && index === segments.length - 1 ? null : ago(to),
            label,
          })),
          marks: marks.map(([minutes, kind, label, verdict]) => ({ at: ago(minutes), kind, label, ...(verdict ? { verdict } : {}) })),
          steps: steps.map(([minutes, text]) => ({ at: ago(minutes), text })),
        },
      ];
    }),
  ) as Timeline["lanes"];
  const since = Object.fromEntries(
    list.filter((s) => s.status === "active").map((s) => [s.id, lanes[s.source].segments.at(-1)?.start ?? now]),
  );
  const open = list.some((s) => s.status === "active");
  return { lanes, since, day: { date: new Date(now).toDateString(), closedMs: 52 * MINUTE_MS, openSince: open ? ago(38) : null } };
}

export const PREVIEW_STATES = [
  "pill",
  "pill-busy",
  "pill-music",
  "pill-waiting",
  "banner-finished",
  "banner-finished-2",
  "banner-message",
  "banner-message-more",
  "banner-message-masked",
  // Prompt 12, DeepL: a message in English to translate.
  "banner-message-foreign",
  "permission",
  "permission-long",
  "permission-risky",
  "failed",
  "toast",
  "music",
  "track",
  "drop",
  "menu",
  "panel",
  "hub",
  "hub-messages",
  "hub-pastilles",
  "hub-live",
  "hub-intro",
  "hub-salut",
  "salut",
  "wardrobe",
  "voice-listening",
  "voice-ask",
  "voice-nokey",
  // Prompt 12, lot 3: the chat, a swallowed file, an e-mail draft, a window as context.
  "chat",
  "chat-nokey",
  "chat-thinking",
  "chat-answer",
  "file",
  "mail",
  "context",
  "context-blocked",
  // Prompt 12, lot 2: an agent's questions, a Codex permission, a change in the live view.
  "question",
  "question-multi",
  "permission-codex",
  "hub-diff",
] as const;

export type PreviewState = (typeof PREVIEW_STATES)[number];

/** Which agents work: the usual pair, three AIs and a generic tool (phase 4), or agents of
 * every kind with a badge (prompt 12, lot 2). */
export type AgentScene = "demo" | "trio" | "many";

export type PreviewOptions = { state: PreviewState; tab: Tab; theme: ThemeId; media: MediaScene; agents: AgentScene };

export function readPreview(search: string): PreviewOptions {
  const params = new URLSearchParams(search);
  const state = params.get("state");
  const tab = params.get("tab");
  return {
    state: PREVIEW_STATES.find((s) => s === state) ?? "pill-busy",
    tab: TABS.find((t) => t.id === tab)?.id ?? "today",
    theme: THEMES.find((t) => t.id === params.get("theme"))?.id ?? DEFAULT_THEME,
    media: readScene(params.get("media")),
    agents: params.get("agents") === "trio" ? "trio" : params.get("agents") === "many" ? "many" : "demo",
  };
}

/** What plays in the preview (?media=spotify|youtube|crunchyroll|none|error). */
let scene: MediaScene = "spotify";
let agentScene: AgentScene = "demo";
let playModes = true;

/** The staged live session is over: Claude's fake session ends, its hour and its dot say
 * so, as the core's events would. */
function finishStage(): void {
  const previous = sessions.list;
  const next = previous.map((s): AgentSession => (s.id === DEMO_HUB_STAGE.sessionId ? { ...s, status: "done", activity: null, subagents: [] } : s));
  timeline.record(previous, next);
  sessions.list = next;
  timeline.mark("claude-code", "done", "epilot, +12 −3");
  island.flash("claude-code", "ok");
}

/** Puts fake data in the stores so that the island shows the requested state. */
export function showState(state: PreviewState): void {
  // Nothing at all: Mochi sleeps alone. Music only: no agent works.
  const quiet = state === "pill" || state === "salut";
  const idle = quiet || state === "pill-music";
  const trio = agentScene === "trio";
  const many = agentScene === "many";
  const hub = state.startsWith("hub");
  const working = many ? DEMO_MANY : hub ? DEMO_HUB : trio ? DEMO_TRIO : DEMO_SESSIONS;
  sessions.list = structuredClone(working).map((s, index) => ({
    ...s,
    // Gemini asks in its own terminal: the pill turns amber, no request in îlot.
    status: idle ? "done" : state === "pill-waiting" && index === 1 ? "waiting" : s.status,
    subagents: idle ? [] : s.subagents,
  }));
  const now = Date.now();
  timeline.replace(demoTimeline(now, sessions.list), now);
  decisions.suspended = false;
  decisions.list = DEMO_DECISIONS.map(([minutes, source, project, command, decision]) => ({
    at: now - minutes * MINUTE_MS,
    source,
    project,
    command,
    decision,
  }));
  history.log = {
    finished: [...DEMO_FINISHED, ...DEMO_FINISHED].map((finished, index) => ({ ...finished, at: now - (12 + 10 * index) * MINUTE_MS })),
    failures: 1,
  };
  notifications.postponed =
    state === "panel" ? DEMO_POSTPONED.map((item, index) => ({ ...item, receivedAt: now - (4 + 15 * index) * MINUTE_MS })) : [];
  startFakeCore(media, withPlayModes(sceneSnapshot(quiet ? "none" : scene, now), playModes));
  permissions.clear();
  banners.queue.clear();
  island.panelOpen = false;
  island.closeHub();
  // The staged demo plays over Claude's session; the real live view shows its change.
  island.hubStage = state === "hub-diff" ? null : { ...DEMO_HUB_STAGE, finish: finishStage };
  island.menuOpen = state === "menu";
  island.wardrobeOpen = state === "wardrobe";
  island.greeting = state === "salut";
  island.dropping = state === "drop";
  if (state === "permission") permissions.add({ ...DEMO_SHORT_PERMISSION, receivedAt: Date.now() });
  else if (state === "permission-risky") permissions.add({ ...DEMO_RISKY_PERMISSION, receivedAt: Date.now() });
  else if (state === "permission-codex") permissions.add({ ...DEMO_CODEX_PERMISSION, receivedAt: Date.now() });
  else if (state === "question") permissions.add({ ...structuredClone(DEMO_AGENT_QUESTION), receivedAt: Date.now() });
  else if (state === "question-multi") permissions.add({ ...structuredClone(DEMO_QUESTIONS), receivedAt: Date.now() });
  else if (state === "permission-long" || (state === "panel" && !trio)) {
    permissions.add({ ...DEMO_PERMISSION, receivedAt: Date.now() });
  }
  // The panel opens on a request already waiting: it keeps the tab it was asked for.
  flushSync();
  island.panelOpen = state === "panel";
  if (state === "hub" || state === "hub-messages" || state === "hub-live" || state === "hub-pastilles") {
    const views = { hub: "home", "hub-messages": "messages", "hub-live": "live", "hub-pastilles": "pastilles" } as const;
    island.openHub({ view: views[state] });
  } else if (state === "hub-intro") {
    island.openHub({ intro: "replay" });
  } else if (state === "hub-salut") {
    island.openHub({ intro: "launch" });
  } else if (state === "hub-diff") {
    island.openChange(sessions.list[0].id);
  }
  if (state === "banner-finished") banners.queue.push({ kind: "finished", items: [DEMO_FINISHED[0]] });
  if (state === "banner-finished-2") banners.queue.push({ kind: "finished", items: structuredClone(DEMO_FINISHED) });
  if (state === "failed") banners.queue.push({ kind: "failed", failed: DEMO_FAILED });
  if (state === "toast") banners.queue.push(answerToast("allow", "Claude Code", DEMO_SHORT_PERMISSION.detail));
  if (state === "music" || state === "track") banners.queue.push({ kind: state });
  const message =
    state === "banner-message" || state === "banner-message-more"
      ? DEMO_BANNERS.short
      : state === "banner-message-masked"
        ? DEMO_BANNERS.masked
        : state === "banner-message-foreign"
          ? DEMO_BANNERS.foreign
          : null;
  const waiting = state === "banner-message-more" ? [DEMO_BANNERS.long, DEMO_BANNERS.group] : [];
  notifications.queue = { current: message ? { ...message, receivedAt: Date.now() } : null, waiting };
  // « OK Mochi »: the microphone listens (the dot by Mochi), then a question waits for a
  // click, or the answer without a key (the preview, like îlot before the vault, has none).
  voice.listening = state.startsWith("voice");
  voice.ask = null;
  if (state === "voice-ask" || state === "voice-nokey") voice.prepare("Résume mon activité.");
  if (state === "voice-nokey") voice.ask = { question: "Résume mon activité.", context: "", status: "error", answer: NO_CLAUDE_KEY };
  // Captures need openings that stay put.
  banners.hold("hover", state.startsWith("banner") || state.startsWith("voice-") || ["failed", "toast", "music", "track", "file"].includes(state));
  showFeatures(state);
}

/** The demo file Mochi swallows in the preview. */
const DEMO_FILE: Attachment = { id: "f-demo", name: "compte-rendu-reunion.txt", kind: "text" };

/** Lot 3 of prompt 12 in the preview: fresh chat and mail, fake keys, the fake window. */
function showFeatures(state: PreviewState): void {
  fake?.stop();
  if (fake) {
    fake.hold = state === "chat-thinking";
    fake.keys.clear();
    if (state !== "chat-nokey") fake.keys.add("chat:anthropic");
  }
  chat.clear();
  chat.dismissContext();
  chat.draft = "";
  chat.keys = {};
  // DeepL: no key until a test adds one; no translation kept from another state.
  fake?.keys.delete("service:deepl");
  translations.hasKey = undefined;
  translations.items = {};
  translations.forgetClipboard();
  mail.discard();
  showFakeWindow(state === "context");
  if (state === "file") banners.queue.push({ kind: "file", attachment: DEMO_FILE });
  if (state === "mail") {
    mail.open(DEMO_FILE);
    mail.to = "lea.ahouansou@exemple.fr";
    mail.body = "Bonjour Léa,\n\nVoici le compte rendu de la réunion de ce matin.\n\nBonne journée !";
  }
  if (state === "chat" || state === "chat-nokey") chat.open();
  if (state === "chat-answer") {
    chat.open();
    chat.messages = [
      { id: "u-demo", role: "user", text: DEMO_QUESTION, file: DEMO_FILE.name },
      { id: "r-demo", role: "assistant", text: DEMO_ANSWER, by: "Claude" },
    ];
  }
  if (state === "chat-thinking") {
    chat.open();
    chat.attachment = DEMO_FILE;
    chat.draft = DEMO_QUESTION;
    void chat.send();
  }
  if (state === "context" || state === "context-blocked") {
    chat.open();
    chat.propose(
      state === "context"
        ? { id: "w-demo", title: DEMO_WINDOW.title, app: DEMO_WINDOW.app, blocked: null }
        : { id: "w-bank", title: "", app: "firefox", blocked: "Banque ou paiement : îlot ne lit pas cette fenêtre." },
    );
  }
}

let fake: FakeFeatures | null = null;

/** The threads of the Messages tab, as if these messages had come in during the run. */
export function demoConversations(now: number): Conversation[] {
  return DEMO_CONVERSATION.reduce<Conversation[]>((list, m, index) => {
    const id = `demo-${index}`;
    const at = now - m.minutesAgo * 60_000;
    const key = conversationKey(m.app, m.sender);
    if (m.mine) return addReply(list, key, m.text, at, id);
    const rule = DEMO_NOTIFICATION_RULES.find((r) => r.id === m.app)!;
    const incoming = { id, app: m.app, appLabel: rule.label, monogram: rule.icon, icon: null, sender: m.sender, text: m.text };
    const next = receiveMessage(list, { ...incoming, masked: false, receivedAt: at });
    return m.read ? markConversationRead(next, key) : next;
  }, []);
}

/** The scene of every agent: Cursor plugged under Windows, OpenCode in WSL. */
function manyConnectors(): ConnectorView[] {
  const plugged: Record<string, string> = { cursor: "windows", opencode: "wsl:Ubuntu" };
  return structuredClone(DEMO_CONNECTORS).map((view) => ({
    ...view,
    targets: view.targets.map((target) => (plugged[view.id] === target.id ? { ...target, status: "connected" as const } : target)),
  }));
}

/** What « Aller au terminal » says in the preview, as the core would for that session. */
function previewTerminal(sessionId: unknown): TerminalOutcome {
  const session = sessions.list.find((s) => s.id === sessionId) ?? sessions.followed;
  if (!session) return { found: false, exact: false, message: "Aucune session à rejoindre." };
  if (session.terminal === "cursor") return { found: true, exact: true, message: `Cursor ouvert sur ${session.project}.` };
  return { found: true, exact: false, message: "Terminal mis au premier plan, onglet non identifié." };
}

/** The preview answers the core's agent commands with fake data: Claude's change of
 * api.py, and what « Aller au terminal » would say. */
function startFakeAgents(): void {
  setPreviewAgents((command, args) => {
    if (command === "session_diff") return sessions.list.find((s) => s.id === args.id)?.activity?.target === "api.py" ? DEMO_DIFF : null;
    if (command === "go_to_terminal") return previewTerminal(args.sessionId);
    if (command === "list_connectors" && agentScene === "many") return manyConnectors();
    return undefined;
  });
}

export function startPreview(search: string): void {
  const options = readPreview(search);
  startFakeAgents();
  document.documentElement.dataset.preview = "";
  const notificationRules = { ...settings.value.notifications, rules: DEMO_NOTIFICATION_RULES };
  const demoChat = { provider: "anthropic" as const, model: DEMO_MODELS.anthropic[1] };
  const extras = readPastillePreview(search);
  settings.apply({
    ...settings.value,
    theme: options.theme,
    notifications: notificationRules,
    chat: demoChat,
    pastilles: sceneSettings(extras.pastilles),
  });
  fake = startFakeFeatures({ keys: ["chat:anthropic"], hold: false });
  startFakePastilles(pastilles, extras.pastilles);
  playModes = extras.playModes;
  island.tab = options.tab;
  const day = /^(\d{4})-(\d{2})-(\d{2})$/.exec(new URLSearchParams(search).get("date") ?? "");
  if (day) wardrobe.today = new Date(Number(day[1]), Number(day[2]) - 1, Number(day[3]));
  scene = options.media;
  agentScene = options.agents;
  conversations.list = demoConversations(Date.now());
  showState(options.state);
  Object.assign(window, {
    __ilot: {
      showState,
      island,
      banners,
      conversations,
      decisions,
      history,
      media,
      notifications,
      pastilles,
      permissions,
      sessions,
      settings,
      timeline,
      sound,
      rest,
      wardrobe,
      demoBanners: DEMO_BANNERS,
      actions,
      voice,
      report,
      /** The report image of the demo week, as a base64 PNG (captures). */
      reportImage: async (hideProjects: boolean): Promise<string> => {
        const png = await paintReport(DEMO_WEEK, hideProjects, settings.value.mochi);
        return btoa(Array.from(png, (byte) => String.fromCharCode(byte)).join(""));
      },
      chat,
      mail,
      translations,
      features: fake,
    },
  });
}
