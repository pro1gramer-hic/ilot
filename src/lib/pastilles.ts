// Pastilles (prompt 12, lot 4): small blocks of state in the island, each with its own
// little Mochi. The Rust core reads them (src-tauri/src/pastilles/) and sends what it saw
// (`pastilles:changed`); this file holds the types shared with it and the pure rules of
// the interface: which pastilles are on, which one the pill shows, how a time or a size
// reads. Spotify's pastille is built here from the media sessions.

import type { AgentSession } from "./agents";
import { texts, textsLocale } from "./i18n/live.svelte";
import { type Track, subtitle } from "./media";
import type { FaceId } from "./mochi/faces";
import type { KindId } from "./mochi/kinds";
import type { MoveId } from "./mochi/moves";
import type { PaletteId } from "./mochi/palettes";

/** How a pastille feels (src-tauri/src/pastilles/services/mod.rs, `Tone`). */
export type PastilleTone = "ok" | "busy" | "warn" | "err" | "idle" | "off";

export type PastilleMetric = { label: string; value: string; tone?: PastilleTone };

/** One line of a list; `at` is told as « il y a 5 min » or « dans 2 h ». */
export type PastilleItem = { title: string; detail: string; at?: number; tone?: PastilleTone };

/** The main tool's window (tools.rs, `ToolView`). */
export type ToolView = { open: boolean; focused: boolean; project?: string; candidates: string[] };

/** A model of a local server (local.rs, `LocalModel`). */
export type LocalModel = { name: string; size: number; loaded: boolean; vram?: number; until?: number; details: string };

/** A download started from îlot (local.rs, `Pull`). */
export type PullView = { model: string; step: string; completed: number; total: number; done: boolean; error?: string };

/** One pastille as the core sends it (mod.rs, `PastilleView`). */
export type PastilleView = {
  id: string;
  tone: PastilleTone;
  line: string;
  short: string;
  metrics: PastilleMetric[];
  items: PastilleItem[];
  problem?: string;
  checkedAt?: number;
  tool?: ToolView;
  models?: LocalModel[];
  pull?: PullView;
};

/** A service of the settings' catalog (mod.rs, `ServiceInfo`). */
export type ServiceInfo = {
  id: string;
  name: string;
  palette: PaletteId;
  kind: KindId;
  /** The user's instance: its default (absent when the address is fixed). */
  customUrl?: string;
  urlRequired: boolean;
  account?: { label: string; hint: string };
  refreshMinutes: number;
  tokenLabel: string;
  rights: string;
  warning?: string;
  help: string;
  hasToken: boolean;
};

export type ToolId = "vscode" | "cursor" | "codex" | "antigravity";

/** One more pastille: a local one, or a service with its address and identifier. */
export type PastilleExtra = { id: string; baseUrl: string; account: string };

/** Réglages › Pastilles (config.rs, `PastilleSettings`). Never a token here. */
export type PastilleSettings = { tool: ToolId | ""; extras: PastilleExtra[] };

export const NO_PASTILLES: PastilleSettings = { tool: "", extras: [] };

/** Pastilles besides the main tool. */
export const MAX_EXTRAS = 4;

/** A pastille's name and its little Mochi. */
export type Look = { name: string; palette: PaletteId; kind: KindId };

export const TOOLS: readonly ({ id: ToolId } & Look)[] = [
  { id: "vscode", name: "VS Code", palette: "ciel", kind: "ecran" },
  { id: "cursor", name: "Cursor", palette: "nuit", kind: "ecran" },
  { id: "codex", name: "Codex", palette: "menthe", kind: "ecran" },
  { id: "antigravity", name: "Antigravity", palette: "lavande", kind: "meche" },
];

export type LocalId = "ollama" | "lmstudio" | "spotify";

/** Pastilles that need no token, with what they show. */
export const LOCALS: readonly ({ id: LocalId; hint: string } & Look)[] = [
  {
    id: "ollama",
    name: "Ollama",
    palette: "nuit",
    kind: "oreilles",
    get hint() {
      return texts().common.pastilles.hints.ollama;
    },
  },
  {
    id: "lmstudio",
    name: "LM Studio",
    palette: "lavande",
    kind: "ecran",
    get hint() {
      return texts().common.pastilles.hints.lmstudio;
    },
  },
  {
    id: "spotify",
    name: "Spotify",
    palette: "menthe",
    kind: "meche",
    get hint() {
      return texts().common.pastilles.hints.spotify;
    },
  },
];

/** The pastilles on, in the order they show: the main tool first. */
export function activeIds(settings: PastilleSettings): string[] {
  return [...(settings.tool ? ["tool"] : []), ...settings.extras.map((e) => e.id)];
}

/** The pastille after `current` (the first one when `current` is gone): « changer de pastille ». */
export function nextId(ids: readonly string[], current: string | null): string | null {
  if (ids.length === 0) return null;
  const index = current === null ? -1 : ids.indexOf(current);
  return ids[(index + 1) % ids.length];
}

/** Name and Mochi of a pastille. */
export function lookOf(id: string, tool: ToolId | "", catalog: readonly ServiceInfo[]): Look {
  if (id === "tool") return TOOLS.find((t) => t.id === tool) ?? { name: texts().common.pastilles.tool, palette: "ciel", kind: "ecran" };
  const local = LOCALS.find((l) => l.id === id);
  if (local) return local;
  const service = catalog.find((s) => s.id === id);
  return service ? { name: service.name, palette: service.palette, kind: service.kind } : { name: id, palette: "lavande", kind: "meche" };
}

/** Agent sessions that work (or wait) right now. */
const working = (s: AgentSession): boolean => s.status === "active" || s.status === "waiting";

/** The project the tool's window shows: a part of its title where an agent of îlot works,
 * or else the core's guess. */
export function toolProject(view: ToolView | undefined, sessions: readonly AgentSession[]): string | null {
  if (!view?.open) return null;
  const projects = new Set(sessions.filter(working).map((s) => s.project.toLowerCase()));
  return view.candidates.find((c) => projects.has(c.toLowerCase())) ?? view.project ?? null;
}

/** How many agents of îlot work in this project. */
export function agentsIn(project: string | null, sessions: readonly AgentSession[]): number {
  if (!project) return 0;
  const name = project.toLowerCase();
  return sessions.filter((s) => working(s) && s.project.toLowerCase() === name).length;
}

/** The tool's state line, with its agents. */
export function toolLine(name: string, view: ToolView | undefined, project: string | null, agents: number): string {
  const said = texts().common.pastilles;
  if (!view?.open) return said.toolClosed(name);
  const where = project ? said.toolOn(name, project) : said.toolOpen(name);
  return agents > 0 ? said.withAgents(where, agents) : where;
}

/** Spotify's pastille, from the media sessions (no request: media.rs already follows them). */
export function spotifyView(spotify: Track | null): PastilleView {
  const base = { id: "spotify", metrics: [], items: [] };
  const said = texts().media;
  if (!spotify) return { ...base, tone: "off", line: said.empty.spotify.title, short: "Spotify" };
  return {
    ...base,
    tone: spotify.playing ? "busy" : "idle",
    line: said.spotifyLine(spotify.playing, spotify.title, subtitle(spotify)),
    short: spotify.title,
  };
}

/** Mochi's stance for a tone: calm even when something fails (a worried face, no shaking). */
export function stanceOf(tone: PastilleTone): { move: MoveId; face?: FaceId } {
  switch (tone) {
    case "ok":
      return { move: "breathe", face: "happy" };
    case "busy":
      return { move: "watch" };
    case "warn":
      return { move: "breathe", face: "wow" };
    case "err":
      return { move: "breathe", face: "worry" };
    case "off":
      return { move: "sleep" };
    default:
      return { move: "breathe" };
  }
}

/** « à l'instant », « il y a 5 min », « il y a 3 h », « hier », « dans 20 min », « demain ». */
export function relativeTime(at: number, now: number): string {
  const minutes = Math.round((at - now) / 60_000);
  const abs = Math.abs(minutes);
  const time = texts().common.time;
  if (abs < 1) return time.justNow;
  const past = minutes < 0;
  if (abs < 60) return past ? time.minutesAgo(abs) : time.inMinutes(abs);
  const hours = Math.round(abs / 60);
  if (hours < 24) return past ? time.hoursAgo(hours) : time.inHours(hours);
  const days = Math.round(hours / 24);
  if (days === 1) return past ? time.yesterday : time.tomorrow;
  return past ? time.daysAgo(days) : time.inDays(days);
}

/** « 6,6 Go », « 512 Mo » ("6.6 GB", "512 MB"). */
export function formatBytes(bytes: number): string {
  const size = texts().common.size;
  if (!Number.isFinite(bytes) || bytes <= 0) return size.megabytes("0");
  const go = bytes / 1024 ** 3;
  if (go >= 1) return size.gigabytes(new Intl.NumberFormat(textsLocale(), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(go));
  return size.megabytes(String(Math.max(1, Math.round(bytes / 1024 ** 2))));
}

/** Share of a download done, from 0 to 1. */
export function pullShare(pull: PullView): number {
  if (pull.done && !pull.error) return 1;
  return pull.total > 0 ? Math.min(1, pull.completed / pull.total) : 0;
}

/** A model name typed by the user (the core checks it again: local.rs, `valid_model`). */
export function validModel(name: string): boolean {
  const text = name.trim();
  return /^[A-Za-z0-9][A-Za-z0-9._:/-]{0,99}$/.test(text) && !text.includes("..") && !text.includes("//");
}

const LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

/** A base URL typed by the user: https, or http on this machine only; no user, password,
 * query or fragment (the core checks it again: http.rs, `base_url`). The clean URL or an
 * error message. */
export function checkBaseUrl(text: string): { url: string } | { error: string } {
  const bad = { error: texts().common.pastilles.badUrl };
  const trimmed = text.trim();
  if (!trimmed || trimmed.length > 200 || /\s/.test(trimmed)) return bad;
  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return bad;
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") return bad;
  if (url.protocol === "http:" && !LOCAL_HOSTS.has(url.hostname)) {
    return { error: texts().common.pastilles.plainHttp };
  }
  if (url.username || url.password || url.search || url.hash || !url.hostname) return bad;
  return { url: url.href.replace(/\/+$/, "") };
}

/** The extras after adding one (four at most, each once). */
export function withExtra(settings: PastilleSettings, extra: PastilleExtra): PastilleSettings {
  if (settings.extras.some((e) => e.id === extra.id) || settings.extras.length >= MAX_EXTRAS) return settings;
  return { ...settings, extras: [...settings.extras, extra] };
}

export function withoutExtra(settings: PastilleSettings, id: string): PastilleSettings {
  return { ...settings, extras: settings.extras.filter((e) => e.id !== id) };
}

/** The vault's name of a service's token (vault.rs: `service:` names only). */
export function tokenName(id: string): string {
  return `service:${id}`;
}
