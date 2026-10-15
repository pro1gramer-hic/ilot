// Bridge to the Rust core. Outside Tauri (browser preview, tests) calls do nothing, except
// the media and video ones, which the browser preview may answer (setPreviewCore).

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toStore } from "svelte/store";
import type { AgentSession } from "./agents";
import { DEMO_CONNECTORS, DEMO_THIS_WEEK, DEMO_WEEK } from "./fixtures";
import { texts } from "./i18n/live.svelte";
import type { Finished } from "./island/banners";
import type { Shape } from "./island/model";
import type { HeadId } from "./mochi/faces";
import type { KindId } from "./mochi/kinds";
import type { PaletteId } from "./mochi/palettes";
import { DEFAULT_MODES, type Modes } from "./modes";
import { DEFAULT_THEME, type ThemeId } from "./themes";
import type { PermissionAnswer, PermissionRequest } from "./stores/permissions.svelte";
import type { MediaAction, MediaLists, MediaSnapshot, RepeatMode, Track } from "./media";
import { NO_PASTILLES, type PastilleSettings, type PastilleView, type ServiceInfo } from "./pastilles";
import type { ReportWeek, WeekReport } from "./report";
import { type ActionId, assignShortcut } from "./shortcuts";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Stands in for the Rust core in the browser preview, so that buttons do something. */
export type PreviewCore = (command: string, args: Record<string, unknown>) => unknown;

let previewCore: PreviewCore | null = null;
/** The browser preview's stand-in for the chat, the vault, files, e-mail and windows
 * (prompt 12, lot 3), asked first; `undefined` hands the command to `previewCore`. */
let previewFeatures: PreviewCore | null = null;

export function setPreviewCore(core: PreviewCore | null): void {
  previewCore = core;
}

export function setPreviewFeatures(core: PreviewCore | null): void {
  previewFeatures = core;
}

async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T | undefined> {
  if (inTauri) return invoke<T>(command, args);
  const answer = previewFeatures?.(command, args);
  return (answer !== undefined ? answer : previewCore?.(command, args)) as T | undefined;
}


/** How îlot recognizes a messaging app (src-tauri/src/notifications.rs, `AppRule`). */
export type AppRule = {
  id: string;
  label: string;
  /** One or two letters shown when Windows gives no logo. */
  icon: string;
  appNames: string[];
  contains: string[];
  programs: string[];
};

export type NotificationSettings = {
  /** Off by default: no notification is read until the user turns this on. */
  enabled: boolean;
  /** Ids of the checked apps; empty by default. */
  allowed: string[];
  hideContent: boolean;
  /** Filled by the Rust core with its default rules. */
  rules: AppRule[];
  /** A message from Firefox, Chrome or Edge counts for WhatsApp when a WhatsApp Web tab is
   * open (on by default, used once WhatsApp is checked). */
  whatsappTabs: boolean;
};

export type MochiSettings = {
  /** 14 characters at most; the core trims it. */
  name: string;
  palette: PaletteId;
  animated: boolean;
  /** Silhouette: « Mèche » by default. */
  kind: KindId;
  /** Face at rest, one of the nine heads. */
  face: HeadId;
};

/** îlot's sounds (prompt 12): one volume and a switch that cuts them all. */
export type SoundSettings = { volume: number; muted: boolean };

/** Mochi's wardrobe: a piece per slot ("" free, "aucun" kept bare) and the seasons. */
export type WardrobeSettings = { head: string; eyes: string; neck: string; ears: string; seasonal: boolean };

/** Réglages › Général: the island follows the cursor to another screen, hides at rest. */
export type DisplaySettings = { followMouse: boolean; fadeWhenIdle: boolean };

/** Shortcuts of the named actions (src-tauri/src/shortcuts.rs); the island's own is
 * `Settings.shortcut`. Changed only through `setShortcut`. */
export type ShortcutSettings = { actions: Partial<Record<string, string>> };

/** « OK Mochi » (src-tauri/src/voice/mod.rs). */
export type VoiceSettings = {
  /** The microphone listens for « OK Mochi »: off by default. */
  listen: boolean;
  /** Mochi answers aloud. */
  speak: boolean;
};

export type Settings = {
  theme: ThemeId;
  modes: Modes;
  hideInFullscreen: boolean;
  doNotDisturb: boolean;
  shortcut: string;
  notifications: NotificationSettings;
  mochi: MochiSettings;
  /** Banner when an agent finishes. */
  announceFinished: boolean;
  /** Start îlot with Windows (off by default). */
  launchAtStartup: boolean;
  sound: SoundSettings;
  wardrobe: WardrobeSettings;
  display: DisplaySettings;
  shortcuts: ShortcutSettings;
  voice: VoiceSettings;
  /** The chat's provider and model by default; keys live in the vault, never here. */
  chat: ChatSettings;
  /** Réglages › Pastilles: the main tool and up to four more (none by default). */
  pastilles: PastilleSettings;
  /** Language of the interface: « auto » follows Windows (src/lib/i18n). */
  language: Language;
  /** DeepL's target language; the key lives in the vault, never here. */
  translate: TranslateSettings;
};

export type Language = "auto" | "fr" | "en";

export const DEFAULT_SETTINGS: Settings = {
  theme: DEFAULT_THEME,
  modes: { ...DEFAULT_MODES },
  hideInFullscreen: true,
  doNotDisturb: false,
  shortcut: "Ctrl+Alt+Space",
  notifications: { enabled: false, allowed: [], hideContent: false, rules: [], whatsappTabs: true },
  mochi: { name: "Mochi", palette: "lavande", animated: true, kind: "meche", face: "idle" },
  announceFinished: true,
  launchAtStartup: false,
  sound: { volume: 0.5, muted: false },
  wardrobe: { head: "", eyes: "", neck: "", ears: "", seasonal: true },
  display: { followMouse: false, fadeWhenIdle: false },
  shortcuts: { actions: {} },
  voice: { listen: false, speak: true },
  chat: { provider: "anthropic", model: "" },
  pastilles: { ...NO_PASTILLES, extras: [] },
  language: "auto",
  translate: { target: "" },
};

export type NotificationAccess = "unavailable" | "notAsked" | "allowed" | "denied";

export type NotificationStatus = {
  access: NotificationAccess;
  /** ilot.exe runs with its package identity (packaging/identity). */
  identity: boolean;
  /** Reading the apps the user checked. */
  listening: boolean;
  /** Windows gives its change event only with the identity: îlot then watches the store. */
  polling: boolean;
};

/** A notification that passed the rules, as the Rust core sends it (already masked when asked). */
export type IncomingNotification = {
  id: string;
  /** Rule id ("whatsapp"). */
  app: string;
  appLabel: string;
  monogram: string;
  /** App logo as a data: URL, when Windows has one. */
  icon: string | null;
  sender: string;
  text: string;
  masked: boolean;
  /** Epoch milliseconds. */
  receivedAt: number;
};

/** What can be clicked in the island's window: the island, the open panel; clicks pass
 * through everywhere else. */
export async function setIslandShape(shapes: Shape[]): Promise<void> {
  if (inTauri) await invoke("set_island_shape", { shapes });
}

/** When the island last asked for the keyboard (performance.now()). */
let focusAskedAt = -Infinity;
/** How long a blur after asking for the keyboard is part of taking it. */

/** Asks for the keyboard. `typing`: the user clicked in a text field, the only case
 * where the core insists until Windows gives it (window.rs, `take_focus`). */
export async function focusIsland(typing = false): Promise<void> {
  focusAskedAt = performance.now();
  if (inTauri) await invoke("focus_island", { typing });
}

/** Taking the keyboard moves Windows' focus through the window to the page: the page may
 * lose it for an instant on the way. A blur that soon after asking is not « elsewhere ». */
export function takingFocus(now = performance.now()): boolean {
  return now - focusAskedAt < FOCUS_SETTLE_MS;
}

const FOCUS_SETTLE_MS = 400;

export async function releaseFocus(): Promise<void> {
  if (inTauri) await invoke("release_focus");
}

export async function getSettings(): Promise<Settings> {
  return inTauri ? invoke<Settings>("get_settings") : { ...DEFAULT_SETTINGS };
}

export async function saveSettings(settings: Settings): Promise<Settings> {
  return inTauri ? invoke<Settings>("save_settings", { settings }) : settings;
}

/** Shortcuts the browser preview pretends Windows already holds (Task Manager's). */
export const PREVIEW_TAKEN_SHORTCUTS = ["Ctrl+Shift+Escape"];

/** Gives `action` a new shortcut (empty: none). The core registers it with Windows before
 * saving it and rejects with a French message when that fails: the previous one stays. */
export async function setShortcut(action: ActionId, shortcut: string, current: Settings): Promise<Settings> {
  if (inTauri) return invoke<Settings>("set_shortcut", { action, shortcut });
  return assignShortcut(current, action, shortcut, PREVIEW_TAKEN_SHORTCUTS);
}

/** The words « OK Mochi » listens for (src-tauri/src/voice/mod.rs, `Grammar`). */
export type VoiceGrammar = { wake: string[]; phrases: string[]; lang: "fr" | "en" };

/** Opens (or closes) the microphone; the core opens it only while « OK Mochi » is on in
 * the settings. True while it listens. The preview pretends. */
export async function voiceListen(on: boolean, grammar: VoiceGrammar | null): Promise<boolean> {
  return inTauri ? invoke<boolean>("voice_listen", { on, grammar }) : on;
}

/** Mochi says `text` aloud (local Windows voice). */
export async function voiceSay(text: string): Promise<void> {
  if (inTauri) await invoke("voice_say", { text });
}

export async function voiceHush(): Promise<void> {
  if (inTauri) await invoke("voice_hush");
}

/** What Mochi says when îlot has no Claude key (the preview never has one), in the language
 * of the moment: the binding follows it. */
export let NO_CLAUDE_KEY = texts().voice.noClaudeKey;
toStore(() => texts().voice.noClaudeKey).subscribe((text) => {
  NO_CLAUDE_KEY = text;
});

/** Sends the question the user confirmed to Claude with their key; rejects with the reason
 * in French (no key: nothing is sent). */
export async function voiceAsk(question: string, context: string): Promise<string> {
  if (inTauri) return invoke<string>("voice_ask", { question, context });
  throw new Error(NO_CLAUDE_KEY);
}

/** The Monday report of last week, or of this week so far; null when nothing was counted.
 * The preview shows a believable week. */
export async function weekReport(which: ReportWeek): Promise<WeekReport | null> {
  return inTauri ? invoke<WeekReport | null>("week_report", { which }) : structuredClone(which === "last" ? DEMO_WEEK : DEMO_THIS_WEEK);
}

/** True once per Monday, when last week's report is ready. */
export async function takeMondayReport(): Promise<boolean> {
  return inTauri ? invoke<boolean>("take_monday_report") : false;
}

/** « Effacer mes statistiques ». */
export async function clearStats(): Promise<void> {
  if (inTauri) await invoke("clear_stats");
}

/** Saves the report image (PNG, 1080 × 1920) where the user chooses; false when they
 * cancelled. The name only suggests a file name (ASCII). */
export async function saveReportImage(png: Uint8Array, name: string): Promise<boolean> {
  if (!inTauri) return false;
  return invoke<boolean>("save_report_image", png, { headers: { "x-ilot-name": name } });
}

export async function answerPermission(id: string, answer: PermissionAnswer): Promise<void> {
  if (inTauri) await invoke("answer_permission", { id, answer });
}

/** Stands in for the core's agent commands in the browser preview (diff, terminal). */
let previewAgents: PreviewCore | null = null;

export function setPreviewAgents(core: PreviewCore | null): void {
  previewAgents = core;
}

async function callAgents<T>(command: string, args: Record<string, unknown> = {}): Promise<T | undefined> {
  return inTauri ? invoke<T>(command, args) : (previewAgents?.(command, args) as T | undefined);
}

/** The user's answer to one question of an agent: labels picked, and a typed answer. */
export type QuestionAnswer = { picked: string[]; other?: string };

/** The user's answers to an agent's questions; the core checks them against the
 * questions and sends nothing that does not fit. */
export async function answerQuestions(id: string, answers: QuestionAnswer[]): Promise<void> {
  await callAgents("answer_questions", { id, answers });
}

/** One line of a change: removed, added, unchanged around it, or lines left out. */
export type DiffLine = { kind: "del" | "add" | "same" | "gap"; text: string };

/** The latest change of a session, from its hooks' payload (src-tauri/src/connectors/diff.rs). */
export type FileDiff = { file: string; lines: DiffLine[]; added: number; removed: number; truncated: boolean };

/** The latest change of a session, kept in memory by the core; none outside Tauri unless
 * the preview stages one. */
export async function sessionDiff(id: string): Promise<FileDiff | null> {
  return (await callAgents<FileDiff | null>("session_diff", { id })) ?? null;
}

/** What « Aller au terminal » did, said honestly (terminal.rs, `Outcome`). */
export type TerminalOutcome = { found: boolean; exact: boolean; message: string };

/** Brings the window of a session (or of the session that matters most) forward. */
export async function goToTerminal(sessionId?: string): Promise<TerminalOutcome> {
  const outcome = await callAgents<TerminalOutcome>("go_to_terminal", { sessionId: sessionId ?? null });
  return outcome ?? { found: false, exact: false, message: "Aucune session à rejoindre." };
}

/** System volume (0 to 1); the browser preview keeps a pretend level. */
let previewVolume = 0.6;

export async function getVolume(): Promise<number> {
  return inTauri ? invoke<number>("get_volume") : previewVolume;
}

/** Wheel on the pill: moves the volume by whole notches (2 % each) and returns the level. */
export async function changeVolume(notches: number): Promise<number> {
  if (inTauri) return invoke<number>("change_volume", { notches });
  previewVolume = Math.min(1, Math.max(0, previewVolume + notches * 0.02));
  return previewVolume;
}

/** `suspended`: « Tout suspendre » is on. */
export type AgentsSnapshot = { sessions: AgentSession[]; permissions: PermissionRequest[]; suspended: boolean };

export async function agentsSnapshot(): Promise<AgentsSnapshot> {
  return inTauri ? invoke<AgentsSnapshot>("agents_snapshot") : { sessions: [], permissions: [], suspended: false };
}

export type ConnectorStatus = "notInstalled" | "installed" | "connected" | "needsRepair";

export type ConnectorView = {
  id: string;
  name: string;
  caps: { activity: boolean; permissions: boolean };
  targets: { id: string; label: string; status: ConnectorStatus }[];
};

/** AIs and where they are installed. The preview shows believable examples (its scene of
 * every agent plugs some of the new ones). */
export async function listConnectors(): Promise<ConnectorView[]> {
  if (inTauri) return invoke<ConnectorView[]>("list_connectors");
  return (previewAgents?.("list_connectors", {}) as ConnectorView[] | undefined) ?? structuredClone(DEMO_CONNECTORS);
}

export async function setConnected(connectorId: string, targetId: string, connected: boolean): Promise<ConnectorView[]> {
  if (!inTauri) return (previewAgents?.("list_connectors", {}) as ConnectorView[] | undefined) ?? structuredClone(DEMO_CONNECTORS);
  return invoke<ConnectorView[]>("set_connected", { connectorId, targetId, connected });
}

export const EMPTY_MEDIA: MediaSnapshot = { current: null, sessions: [], recent: [], error: null };

/** What plays now, every session and the history of this run. */
export async function getMedia(): Promise<MediaSnapshot> {
  return (await call<MediaSnapshot>("get_media")) ?? { ...EMPTY_MEDIA };
}

/** Sends a command to the session of `appId`, or to the current session. */
export async function mediaCommand(action: MediaAction, appId?: string): Promise<void> {
  await call("media_command", { action, appId });
}

/** Brings the app or browser that plays to the front. */
export async function openMediaSource(appId: string): Promise<void> {
  await call("open_media_source", { appId });
}

export async function openMiniPlayer(): Promise<void> {
  await call("open_mini_player");
}

export async function closeMiniPlayer(): Promise<void> {
  await call("close_mini_player");
}

/** Plays a video in îlot's isolated YouTube window. */
export async function playYoutube(videoId: string, start: number): Promise<void> {
  await call("play_youtube", { videoId, start });
}

/** Outside Tauri (browser preview), Windows is out of reach. */
export const NO_NOTIFICATIONS: NotificationStatus = {
  access: "unavailable",
  identity: false,
  listening: false,
  polling: false,
};

export async function notificationStatus(): Promise<NotificationStatus> {
  return inTauri ? invoke<NotificationStatus>("notifications_status") : { ...NO_NOTIFICATIONS };
}

/** Shows Windows' consent prompt. */
export async function requestNotificationAccess(): Promise<NotificationStatus> {
  return inTauri ? invoke<NotificationStatus>("request_notification_access") : { ...NO_NOTIFICATIONS };
}

/** Brings the app (or browser) that sent this notification to the front; rejects with a French message. */
export async function openNotificationApp(id: string): Promise<void> {
  if (inTauri) await invoke("open_notification_app", { id });
}

/** Opens Windows Settings › Privacy › Notifications. */
export async function openNotificationSettings(): Promise<void> {
  if (inTauri) await invoke("open_notification_settings");
}

// The final demo's port (docs/10-PORTAGE-FINALE.md): the island window that moves, Mochi
// out of the island, the decisions journal, « Tout suspendre », the audio level.

/** Where the pill sits in its window, and which way the island opens. */
export type Placement = { anchorX: number; anchorY: number; opensDown: boolean };

/** Cursor seen by the core, in the island window's coordinates. */
export type IslandPointer = { x: number; y: number; inside: boolean; near: boolean };

export type BuddySpotKind = "bas" | "fenetre" | "curseur" | "bord" | "milieu";
export type BuddySpot = { kind: BuddySpotKind; x: number; y: number };
export type BuddyMode = "balade" | "suit" | "bureau";

export type DecisionKind = "allow" | "deny" | "always" | "suspended" | "terminal" | "expired";
export type Decision = { at: number; source: string; project: string; command: string; decision: DecisionKind };

export async function islandDragStart(): Promise<void> {
  if (inTauri) await invoke("island_drag_start");
}

export async function islandMoveBy(dx: number, dy: number): Promise<void> {
  if (inTauri) await invoke("island_move_by", { dx, dy });
}

/** Where the core placed the pill (its first `island:placed` may go before the page
 * listens). The preview keeps its own placement. */
export async function islandPlacement(): Promise<Placement | undefined> {
  return inTauri ? invoke<Placement>("island_placement") : undefined;
}

/** The island at rest (`low`: its webview uses as little memory as it can) or in use. */
export async function islandMemory(low: boolean): Promise<void> {
  if (inTauri) await invoke("island_memory", { low });
}

export async function islandRecenter(): Promise<void> {
  if (inTauri) await invoke("island_recenter");
}

export async function buddyShow(x: number, y: number, mode: BuddyMode): Promise<void> {
  if (inTauri) await invoke("buddy_show", { x, y, mode });
}

export async function buddyMove(x: number, y: number): Promise<void> {
  if (inTauri) await invoke("buddy_move", { x, y });
}

/** Places Mochi can go to on the screen; the preview makes some up. */
export async function buddySpots(): Promise<BuddySpot[]> {
  if (inTauri) return invoke<BuddySpot[]>("buddy_spots");
  return [
    { kind: "bas", x: 640, y: 690 },
    { kind: "fenetre", x: 300, y: 180 },
    { kind: "curseur", x: 520, y: 400 },
    { kind: "bord", x: 1340, y: 380 },
    { kind: "milieu", x: 680, y: 420 },
  ];
}

export async function buddyRecall(): Promise<void> {
  if (inTauri) await invoke("buddy_recall");
}

/** Screen point where Mochi lands back in the island. */
export async function buddyHome(): Promise<{ x: number; y: number }> {
  return inTauri ? invoke<{ x: number; y: number }>("buddy_home") : { x: 640, y: 28 };
}

export async function buddyHide(): Promise<void> {
  if (inTauri) await invoke("buddy_hide");
}

/** « Sur le bureau »: the user picked Mochi up (his page follows the cursor). */
export async function buddyGrab(): Promise<void> {
  if (inTauri) await invoke("buddy_grab");
}

/** « Sur le bureau »: Mochi was put down there; the core saves the place. */
export async function buddySettle(x: number, y: number): Promise<void> {
  if (inTauri) await invoke("buddy_settle", { x, y });
}

/** At launch: Mochi comes back where he was left on the desktop. True when he is out. */
export async function buddyRestore(): Promise<boolean> {
  return inTauri ? invoke<boolean>("buddy_restore") : false;
}

/** From Mochi's window: his wardrobe opens in the island. */
export async function openWardrobe(): Promise<void> {
  if (inTauri) await invoke("open_wardrobe");
}

// Several screens and the island at rest (prompt 12, lot 1).

/** A screen as Réglages › Général lists it. */
export type ScreenView = { id: string; label: string; detail: string; primary: boolean; current: boolean };

/** The browser preview pretends to have two screens. */
let previewScreens: ScreenView[] = [
  { id: "DISPLAY1", label: "Écran 1", detail: "1920 × 1080", primary: true, current: true },
  { id: "DISPLAY2", label: "Écran 2", detail: "2560 × 1440", primary: false, current: false },
];

export async function listScreens(): Promise<ScreenView[]> {
  return inTauri ? invoke<ScreenView[]>("list_screens") : structuredClone(previewScreens);
}

export async function islandToScreen(id: string): Promise<void> {
  if (inTauri) await invoke("island_to_screen", { id });
  else previewScreens = previewScreens.map((s) => ({ ...s, current: s.id === id }));
}


// Sounds (prompt 12, lot 1): the interface makes each WAV, the core plays it.

/** A replacement file found in %APPDATA%\ilot\sons. */
export type SoundFile = { id: string; file: string; usable: boolean };

export async function playSound(wav: Uint8Array): Promise<void> {
  if (inTauri) await invoke("play_sound", wav);
}

export async function soundOverrides(): Promise<SoundFile[]> {
  return inTauri ? invoke<SoundFile[]>("sound_overrides") : [];
}

/** The bytes of a replacement file (an ArrayBuffer from the core). */
export async function soundFile(id: string): Promise<ArrayBuffer> {
  const bytes = await invoke<ArrayBuffer | number[]>("sound_file", { id });
  return bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes).buffer;
}

export async function openSoundsFolder(): Promise<void> {
  if (inTauri) await invoke("open_sounds_folder");
}

export async function suspendAgents(on: boolean): Promise<void> {
  if (inTauri) await invoke("suspend_agents", { on });
}

export async function decisions(): Promise<Decision[]> {
  return inTauri ? invoke<Decision[]>("decisions") : [];
}

/** Asks the core for the output's peak level (`audio:level`), never the sound itself. */
export async function audioMeter(on: boolean): Promise<void> {
  if (inTauri) await invoke("audio_meter", { on });
}

export async function mediaSeek(appId: string, seconds: number): Promise<void> {
  await call("media_seek", { appId, seconds });
}

/** Shuffle and repeat of a session (SMTC); the app may refuse. */
export async function mediaMode(appId: string, mode: { shuffle?: boolean; repeat?: RepeatMode }): Promise<void> {
  await call("media_mode", { appId, shuffle: mode.shuffle ?? null, repeat: mode.repeat ?? null });
}

/** Volume of one app (Spotify), 0 to 1; null while it plays no sound. */
export async function appVolume(app: "spotify"): Promise<number | null> {
  return (await call<number | null>("app_volume", { app })) ?? null;
}

export async function setAppVolume(app: "spotify", level: number): Promise<number> {
  return (await call<number>("set_app_volume", { app, level })) ?? level;
}

// Pastilles (src-tauri/src/pastilles/): the core reads, the island shows.

export async function pastillesSnapshot(): Promise<PastilleView[]> {
  return inTauri ? invoke<PastilleView[]>("pastilles_snapshot") : [];
}

/** The services îlot knows, and which have a token stored. */
export async function pastillesCatalog(): Promise<ServiceInfo[]> {
  return inTauri ? invoke<ServiceInfo[]>("pastilles_catalog") : [];
}

/** Reads a pastille again soon (the core keeps 30 s between two readings). */
export async function pastilleRefresh(id: string): Promise<void> {
  if (inTauri) await invoke("pastille_refresh", { id });
}

/** Brings the main tool chosen in the settings to the front. */
export async function pastilleShowTool(): Promise<void> {
  if (inTauri) await invoke("pastille_show_tool");
}

/** Starts an Ollama download the user asked for. */
export async function ollamaPull(model: string): Promise<void> {
  if (inTauri) await invoke("ollama_pull", { model });
}

export async function ollamaPullCancel(): Promise<void> {
  if (inTauri) await invoke("ollama_pull_cancel");
}

export async function copyText(text: string): Promise<void> {
  if (inTauri) await invoke("copy_text", { text });
}

export type VideoSite = "youtube" | "crunchyroll";

/** How the video is out of the island (`DetachedKind` of video.rs). */
export type DetachedKind = "window" | "pictureInPicture" | "onTop";

/** Payload of `video:state` and of `video_state` (video.rs, `VideoState`). */
export type VideoState = { detached: DetachedKind | null; site: VideoSite | null; extension: boolean };

/** What « Détacher la vidéo » did (video.rs, `Outcome`). */
export type DetachOutcome = "window" | "pictureInPicture" | "needsGesture" | "onTop" | "miniPlayer";

export const NO_VIDEO: VideoState = { detached: null, site: null, extension: false };

/** Detaches the video of a session into a player that has the right to play it. */
export async function detachVideo(appId: string, animate: boolean): Promise<DetachOutcome> {
  return (await call<DetachOutcome>("detach_video", { appId, animate })) ?? "miniPlayer";
}

/** Sends the detached video back into the island. */
export async function dockVideo(animate: boolean): Promise<void> {
  await call("dock_video", { animate });
}

export async function videoState(): Promise<VideoState> {
  return (await call<VideoState>("video_state")) ?? { ...NO_VIDEO };
}

/** Opens îlot's Crunchyroll window, where the user signs in. */
export async function openCrunchyroll(): Promise<void> {
  await call("open_crunchyroll");
}

/** The bar of the small player moves to (x, y), logical screen pixels; its player follows. */
export async function videoBarMove(x: number, y: number): Promise<void> {
  await call("video_bar_move", { x, y });
}

/** The bar closes its player. */
export async function closeVideo(): Promise<void> {
  await call("close_video");
}

// ---------- Prompt 12, lot 3: the vault, the chat, dropped files, e-mail, a window as
// context, dictation.

/** The chat's providers: three on the Internet with the user's key, two on this PC. */
export type ChatProvider = "anthropic" | "gemini" | "openai" | "ollama" | "lmstudio";

/** An empty model means « the provider's first one ». */
export type ChatSettings = { provider: ChatProvider; model: string };

/** Saves (or replaces) a secret in the Windows Credential Manager. Names: `chat:<provider>`
 * or `service:<id>`. The secret is never read back by the interface. */
export async function secretStore(name: string, secret: string): Promise<void> {
  await call("secret_store", { name, secret });
}

/** « Clé enregistrée » or not: all the interface ever learns about a secret. */
export async function secretHas(name: string): Promise<boolean> {
  return (await call<boolean>("secret_has", { name })) ?? false;
}

export async function secretRemove(name: string): Promise<void> {
  await call("secret_remove", { name });
}

export type ChatTurn = { role: "user" | "assistant"; text: string };

export type ChatRequest = {
  requestId: string;
  provider: ChatProvider;
  model: string;
  turns: ChatTurn[];
  /** An attachment made from a file dropped on the island (`fileAttach`). */
  attachment?: string;
  /** A window's text the user previewed and joined (`contextConfirm`). */
  context?: string;
};

/** Sends a question; the answer streams back as `chat:delta`, then `chat:done` or `chat:error`. */
export async function chatSend(request: ChatRequest): Promise<void> {
  await call("chat_send", { request });
}

export async function chatCancel(requestId: string): Promise<void> {
  await call("chat_cancel", { requestId });
}

/** The models a provider offers (read from the provider itself). */
export async function chatModels(provider: ChatProvider): Promise<string[]> {
  return (await call<string[]>("chat_models", { provider })) ?? [];
}

/** A dropped file the chat or a mail may use; its content is read only when sent. */
export type Attachment = { id: string; name: string; kind: "text" | "image" | "other" };

/** Turns the path of a file just dropped on the island into an attachment. */
export async function fileAttach(path: string): Promise<Attachment | undefined> {
  return call<Attachment>("file_attach", { path });
}

export type MailDraft = { to: string; subject: string; body: string; attachment?: string };

/** How the draft left the island (mail.rs, `Handed`). */
export type MailHanded = { way: "mapi" | "outlook" | "mailto"; client: string; note: string | null };

/** The mail app the island names before « Envoyer ». */
export type MailApp = { name: string; mapi: boolean };

export async function mailApp(): Promise<MailApp> {
  return (await call<MailApp>("mail_app")) ?? { name: "ton application de messagerie", mapi: false };
}

/** « Envoyer »: hands the draft to the mail app, which opens it ready to send. */
export async function mailSend(draft: MailDraft): Promise<MailHanded | undefined> {
  return call<MailHanded>("mail_send", { draft });
}

/** A window Mochi was dropped on (window_context.rs). A blocked one has no title. */
export type ContextProposal = { id: string; title: string; app: string; blocked: string | null };

/** What « Lire la fenêtre » shows before anything is joined. */
export type ContextPreview = { id: string; title: string; app: string; text: string; truncated: boolean };

/** Mochi was let go there (buddy window): the window under him is offered to the island. */
export async function buddyDropWindow(x: number, y: number): Promise<boolean> {
  return (await call<boolean>("buddy_drop_window", { x, y })) ?? false;
}

export async function contextRead(id: string): Promise<ContextPreview | undefined> {
  return call<ContextPreview>("context_read", { id });
}

export async function contextConfirm(id: string): Promise<void> {
  await call("context_confirm", { id });
}

export async function contextForget(id: string): Promise<void> {
  await call("context_forget", { id });
}

/** True when Windows has its French speech recognizer (local dictation). */
export async function dictationAvailable(): Promise<boolean> {
  return (await call<boolean>("dictation_available")) ?? false;
}

export async function dictationStart(): Promise<void> {
  await call("dictation_start");
}

export async function dictationStop(): Promise<void> {
  await call("dictation_stop");
}

// ---------- Prompt 12: DeepL (src-tauri/src/translate.rs) and the clipboard read for it.

/** DeepL's code of the language texts go into; empty follows the interface's language. */
export type TranslateSettings = { target: string };

/** Why the core gave no translation (nothing was sent for `noKey`, `empty`, `tooLong`,
 * `badLanguage`). */
export type TranslateFailure =
  | "noKey"
  | "keyRefused"
  | "quota"
  | "busy"
  | "unavailable"
  | "network"
  | "tooLong"
  | "empty"
  | "badLanguage"
  | "unexpected";

/** A translation and the language DeepL recognized (`EN`…, empty when it did not say). */
export type Translation = { text: string; detected: string };

/** « Traduire »: the core sends this text to DeepL with the user's key. Throws a
 * `TranslateFailure`. */
export async function translateText(text: string, target: string): Promise<Translation> {
  const translation = await call<Translation>("translate_text", { text, target });
  if (!translation) throw "unexpected" satisfies TranslateFailure;
  return translation;
}

/** Why the clipboard gave no text. */
export type ClipboardFailure = "empty" | "tooLong" | "busy";

/** « Traduire le presse-papiers »: its text, read by the core now and only now. Throws a
 * `ClipboardFailure`. */
export async function readClipboardText(): Promise<string> {
  const text = await call<string>("read_clipboard_text");
  if (text === undefined) throw "empty" satisfies ClipboardFailure;
  return text;
}

export type CoreEvents = {
  "island:drag": { dragging: boolean };
  "island:placed": Placement;
  "island:pointer": IslandPointer;
  /** `down`: the main button is held (Mochi stays in hand). */
  "buddy:cursor": { x: number; y: number; down: boolean };
  "buddy:recall": null;
  "buddy:returned": null;
  /** A right click on Mochi out of the island: his wardrobe opens. */
  "island:wardrobe": null;
  "agents:suspended": { on: boolean };
  "decision:added": Decision;
  "audio:level": { peak: number };
  "island:toggle": null;
  "island:open-settings": null;
  "settings:changed": Settings;
  "sessions:changed": AgentSession[];
  "permission:added": PermissionRequest;
  "permission:removed": { id: string };
  "agent:finished": Finished;
  /** A session has a new latest change (fetched with `sessionDiff`). */
  "diff:changed": { id: string };
  "media:changed": Track | null;
  "media:sessions": MediaLists;
  "video:state": VideoState;
  "notifications:incoming": IncomingNotification;
  "notifications:status": NotificationStatus;
  /** A shortcut of a named action was pressed (the island's own sends `island:toggle`). */
  "shortcut:action": string;
  /** « OK Mochi »: the keyword (alone when `phrase` is empty), or a phrase after it. */
  "voice:heard": { wake: boolean; phrase: string };
  "voice:state": { listening: boolean };

  "chat:delta": { requestId: string; text: string };
  "chat:done": { requestId: string };
  "chat:error": { requestId: string; message: string };
  "context:proposed": ContextProposal;
  "dictation:text": { text: string; final: boolean };
  "dictation:end": { error: string | null };

  "pastilles:changed": PastilleView[];
};

/** Listeners of the browser preview, where `emitPreview` stands in for the core. */
const previewListeners = new Map<string, Set<(payload: unknown) => void>>();

/** The browser preview plays an event of the core (a streamed answer, a window offered). */
export function emitPreview<K extends keyof CoreEvents>(name: K, payload: CoreEvents[K]): void {
  for (const handler of previewListeners.get(name) ?? []) handler(payload);
}

/** Listens to an event of the Rust core; returns a function that stops listening. */
export function onCoreEvent<K extends keyof CoreEvents>(
  name: K,
  handler: (payload: CoreEvents[K]) => void,
): () => void {
  if (!inTauri) {
    const listeners = previewListeners.get(name) ?? new Set();
    const listener = handler as (payload: unknown) => void;
    listeners.add(listener);
    previewListeners.set(name, listeners);
    return () => {
      listeners.delete(listener);
    };
  }
  const stop = listen<CoreEvents[K]>(name, (event) => handler(event.payload));
  return () => {
    void stop.then((unlisten) => unlisten());
  };
}
