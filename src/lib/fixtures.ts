// Believable fake data. Phase 1 shows it in the island until the real sources are
// plugged in; the browser preview keeps using it for screenshots.

import type { AgentSession, AiSource } from "./agents";
import type { HubStage } from "./hub";
import type { AppRule, ConnectorView, DecisionKind, FileDiff } from "./ipc";
import type { Failed, Finished } from "./island/banners";
import type { Track } from "./media";
import type { NotificationItem } from "./notifications";
import type { DayCounters, WeekReport } from "./report";
import type { PermissionRequest } from "./stores/permissions.svelte";

export const DEMO_SESSIONS: AgentSession[] = [
  {
    id: "claude-1",
    source: "claude-code",
    project: "epilot",
    task: "corrige le fuseau horaire de l'API",
    status: "active",
    activity: { verb: "édite", target: "api.py", added: 12, removed: 3 },
    subagents: [
      { id: "agent-1", kind: "Explore" },
      { id: "agent-2", kind: "Explore" },
      { id: "agent-3", kind: "Plan" },
    ],
  },
  {
    id: "gemini-1",
    source: "gemini-cli",
    project: "my_rpg",
    task: "écrit les tests du parseur de cartes",
    status: "active",
    activity: { verb: "lit", target: "map_parser.c" },
  },
  {
    id: "codex-1",
    source: "codex",
    project: "ilot",
    task: "documente le relais ilot-hook",
    status: "done",
  },
];

/** Cover drawn in SVG: the preview never loads anything from the network. */
function cover(width: number, height: number, body: string): string {
  const size = `width="${width}" height="${height}" viewBox="0 0 ${width} ${height}"`;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" ${size}>${body}</svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

const LOFI_COVER = cover(
  300,
  300,
  `<defs><linearGradient id="sky" x2="0" y2="1"><stop offset="0" stop-color="#2a1b5e"/><stop offset=".6" stop-color="#b14f97"/><stop offset="1" stop-color="#ff9b62"/></linearGradient></defs>
  <rect width="300" height="300" fill="url(#sky)"/><circle cx="214" cy="88" r="32" fill="#ffe6c4"/>
  <path d="M0 214h300v86H0z" fill="#1b1239"/><rect x="38" y="164" width="124" height="74" rx="7" fill="#120c29"/>
  <rect x="50" y="175" width="100" height="47" rx="3" fill="#8b93ff" opacity=".6"/><rect x="190" y="184" width="10" height="50" fill="#120c29"/>
  <circle cx="195" cy="176" r="16" fill="#2f7a5b"/>`,
);

const RAIN_COVER = cover(
  480,
  270,
  `<defs><linearGradient id="night" x2="0" y2="1"><stop offset="0" stop-color="#090d28"/><stop offset="1" stop-color="#29336a"/></linearGradient></defs>
  <rect width="480" height="270" fill="url(#night)"/><rect x="300" y="70" width="110" height="150" rx="4" fill="#f6c87a" opacity=".85"/>
  <path d="M355 70v150M300 145h110" stroke="#29336a" stroke-width="6"/>
  <g stroke="#9fb2ff" stroke-width="2" opacity=".5">${Array.from({ length: 26 }, (_, i) => `<path d="M${(i * 37) % 480} ${(i * 53) % 270}l-8 26"/>`).join("")}</g>`,
);

const WAVES_COVER = cover(
  480,
  270,
  `<defs><linearGradient id="sea" x2="0" y2="1"><stop offset="0" stop-color="#0b3b5c"/><stop offset="1" stop-color="#58c4c8"/></linearGradient></defs>
  <rect width="480" height="270" fill="url(#sea)"/><circle cx="360" cy="80" r="40" fill="#fff4dc" opacity=".9"/>
  <path d="M0 170q60-30 120 0t120 0 120 0 120 0v100H0z" fill="#0e5c7c"/><path d="M0 205q60-24 120 0t120 0 120 0 120 0v65H0z" fill="#08354d"/>`,
);

const SAKURA_COVER = cover(
  480,
  270,
  `<defs><linearGradient id="dusk" x2="0" y2="1"><stop offset="0" stop-color="#2e2468"/><stop offset=".55" stop-color="#b04f98"/><stop offset="1" stop-color="#ff8c69"/></linearGradient></defs>
  <rect width="480" height="270" fill="url(#dusk)"/><path d="M0 210q120-60 240-10t240-20v90H0z" fill="#2a1840"/>
  <g fill="#ffd1e3">${Array.from({ length: 18 }, (_, i) => `<circle cx="${(i * 71) % 480}" cy="${(i * 41) % 190}" r="${3 + (i % 3)}"/>`).join("")}</g>`,
);

const DUEL_COVER = cover(
  480,
  270,
  `<defs><radialGradient id="sun"><stop offset="0" stop-color="#fff3e2"/><stop offset=".45" stop-color="#ff6a3d"/><stop offset="1" stop-color="#3b0f1f"/></radialGradient></defs>
  <rect width="480" height="270" fill="url(#sun)"/><path d="M0 220h480v50H0z" fill="#1c0710"/>
  <path d="M150 220l14-70 14 70zM302 220l14-70 14 70z" fill="#1c0710"/>`,
);

const ALL_CONTROLS = { playPause: true, next: true, previous: true };

const DEMO_SPOTIFY: Track = {
  id: "spotify-lofi",
  title: "Lo-fi beats pour coder",
  artist: "Lofi Girl",
  source: "Spotify",
  app: "Spotify",
  appId: "Spotify.exe",
  playing: true,
  position: 1312,
  duration: 7200,
  updatedAt: 0,
  controls: ALL_CONTROLS,
  artwork: LOFI_COVER,
};

const DEMO_YOUTUBE: Track = {
  id: "youtube-rain",
  title: "Pluie de nuit · lo-fi pour dormir et réviser",
  artist: "Chillhop Music",
  source: "YouTube",
  app: "Microsoft Edge",
  appId: "MSEdge",
  playing: false,
  position: 846,
  duration: 0,
  updatedAt: 0,
  controls: ALL_CONTROLS,
  artwork: RAIN_COVER,
};

const DEMO_CRUNCHYROLL: Track = {
  id: "crunchyroll-ep7",
  title: "Épisode 7 · Le dernier printemps",
  artist: "",
  source: "Crunchyroll",
  app: "Crunchyroll",
  appId: "15EF7777.Crunchyroll_mgdgtskya6f22!App",
  playing: false,
  position: 760,
  duration: 1430,
  updatedAt: 0,
  controls: { playPause: true, next: false, previous: false },
  artwork: SAKURA_COVER,
};


/** Sessions open on the fake desktop. */
export const DEMO_MEDIA_SESSIONS: Track[] = [DEMO_SPOTIFY, DEMO_YOUTUBE, DEMO_CRUNCHYROLL];

/** History of the fake run, newest first. */
export const DEMO_QUEUE: Track[] = [
  DEMO_SPOTIFY,
  DEMO_CRUNCHYROLL,
  {
    ...DEMO_YOUTUBE,
    id: "youtube-waves",
    title: "Vagues · musique pour coder",
    artist: "Ambient Lab",
    app: "îlot",
    appId: "ilot.exe",
    duration: 7200,
    artwork: WAVES_COVER,
    videoId: "aBcDeFgHiJk",
  },
  DEMO_YOUTUBE,
  {
    ...DEMO_CRUNCHYROLL,
    id: "crunchyroll-ep12",
    title: "Épisode 12 · Le duel",
    position: 312,
    duration: 1440,
    artwork: DUEL_COVER,
  },
];

export const DEMO_PERMISSION: PermissionRequest = {
  id: "perm-1",
  source: "claude-code",
  project: "epilot",
  title: "Claude veut exécuter une commande",
  detail: "npm run build -- --mode production && npm run test:api",
  reason: "Pour vérifier que le front compile après la modification de api.py.",
  truncated: false,
  extras: [],
  // Compound and runs project code: Claude rules cannot match it as read.
  canAlways: false,
  receivedAt: 0,
  timeoutMs: 120_000,
  // What the core's `risk::assess` says of this command: nothing risky.
  risk: "low",
  warnings: [],
  folder: "~/dev/epilot",
};

/** A command short enough to be read whole in the banner, where it can be allowed. */
export const DEMO_SHORT_PERMISSION: PermissionRequest = {
  ...DEMO_PERMISSION,
  id: "perm-short",
  detail: "git status --short",
  reason: null,
  canAlways: true,
};

export const DEMO_FINISHED: Finished[] = [
  { source: "claude-code", project: "epilot", durationMs: 252_000, added: 12, removed: 3 },
  { source: "gemini-cli", project: "my_rpg", durationMs: 610_000, added: 48, removed: 0 },
];

export const DEMO_CONNECTORS: ConnectorView[] = [
  {
    id: "claude-code",
    name: "Claude",
    caps: { activity: true, permissions: true },
    targets: [
      { id: "wsl:Ubuntu", label: "WSL · Ubuntu", status: "connected" },
      { id: "windows", label: "Windows", status: "notInstalled" },
    ],
  },
  {
    id: "gemini-cli",
    name: "Gemini",
    caps: { activity: true, permissions: false },
    targets: [
      { id: "wsl:Ubuntu", label: "WSL · Ubuntu", status: "connected" },
      { id: "windows", label: "Windows", status: "notInstalled" },
    ],
  },
  {
    id: "codex",
    name: "Codex",
    caps: { activity: true, permissions: true },
    targets: [
      { id: "wsl:Ubuntu", label: "WSL · Ubuntu", status: "connected" },
      { id: "windows", label: "Windows", status: "notInstalled" },
    ],
  },
  {
    id: "generic",
    name: "Autre",
    caps: { activity: true, permissions: false },
    targets: [
      { id: "wsl:Ubuntu", label: "WSL · Ubuntu", status: "installed" },
      { id: "windows", label: "Windows", status: "notInstalled" },
    ],
  },
  ...(
    [
      ["cursor", "Cursor", "installed", "installed"],
      ["copilot", "Copilot", "notInstalled", "installed"],
      ["opencode", "OpenCode", "notInstalled", "installed"],
      ["amp", "Amp", "notInstalled", "notInstalled"],
      ["bob", "IBM Bob", "notInstalled", "notInstalled"],
      ["kiro", "Kiro", "notInstalled", "installed"],
    ] as const
  ).map(
    ([id, name, windows, wsl]): ConnectorView => ({
      id,
      name,
      caps: { activity: true, permissions: false },
      targets: [
        { id: "wsl:Ubuntu", label: "WSL · Ubuntu", status: wsl },
        { id: "windows", label: "Windows", status: windows },
      ],
    }),
  ),
];

/** Three AIs and a generic tool at work at once (phase 4 captures). */
export const DEMO_TRIO: AgentSession[] = [
  {
    id: "claude-code:c1",
    source: "claude-code",
    project: "epilot",
    task: "corrige le fuseau horaire de l'API",
    status: "active",
    activity: { verb: "édite", target: "api.py", added: 12, removed: 3 },
    subagents: [
      { id: "agent-1", kind: "Explore" },
      { id: "agent-2", kind: "Explore" },
      { id: "agent-3", kind: "Plan" },
    ],
  },
  {
    id: "gemini-cli:g1",
    source: "gemini-cli",
    project: "my_rpg",
    task: "écrit les tests du parseur de cartes",
    status: "waiting",
    activity: { verb: "lance", target: "make tests_run" },
  },
  {
    id: "codex:x1",
    source: "codex",
    project: "ilot",
    task: "documente le relais ilot-hook",
    status: "active",
    activity: { verb: "édite", target: "README.md", added: 40, removed: 2 },
    subagents: [{ id: "x-agent-1", kind: "worker" }],
  },
  {
    id: "generic:ci@site",
    source: "generic",
    name: "ci",
    project: "site",
    task: "npm run build",
    status: "done",
    activity: { verb: "travaille" },
  },
];

/** Agents of every kind at once (lot 2 captures): Claude in Windows Terminal, Cursor and
 * Copilot at work, OpenCode done, Kiro waiting in its terminal, and a tool that gave
 * itself a badge with `ilot_agent`. */
export const DEMO_MANY: AgentSession[] = [
  { ...DEMO_TRIO[0], terminal: "windows-terminal" },
  {
    id: "cursor:668320d2",
    source: "cursor",
    project: "site",
    task: "refait la page d'accueil",
    status: "active",
    activity: { verb: "édite", target: "Hero.tsx", added: 24, removed: 9 },
    terminal: "cursor",
  },
  {
    id: "copilot:2f6c8d0e",
    source: "copilot",
    project: "api",
    task: "ajoute les tests du parseur",
    status: "active",
    activity: { verb: "lance", target: "npm test" },
    subagents: [{ id: "copilot-a1", kind: "Test writer" }],
  },
  { id: "opencode:ses_01JZ8Q4V3K2M", source: "opencode", project: "notes", task: "range les notes de réunion", status: "done" },
  {
    id: "kiro:abc123",
    source: "kiro",
    project: "infra",
    task: "met à jour le pipeline",
    status: "waiting",
    inTerminal: true,
    activity: { verb: "demande" },
  },
  {
    id: "claude-code:q1",
    source: "claude-code",
    agent: { id: "qwen", name: "Qwen Code", color: "teal" },
    project: "data",
    task: "nettoie le jeu de données",
    status: "active",
    activity: { verb: "lit", target: "clean.py" },
  },
];

/** Claude's latest change of api.py, as its Edit hook describes it (preview of the live
 * view's diff). */
export const DEMO_DIFF: FileDiff = {
  file: "api.py",
  added: 3,
  removed: 2,
  truncated: false,
  lines: [
    { kind: "same", text: "" },
    { kind: "same", text: "def horodatage():" },
    { kind: "del", text: "    return datetime.now()" },
    { kind: "add", text: "    return datetime.now(timezone.utc)  # fuseau explicite" },
    { kind: "same", text: "" },
    { kind: "gap", text: "" },
    { kind: "del", text: "from datetime import datetime" },
    { kind: "add", text: "from datetime import datetime, timezone" },
    { kind: "add", text: "from zoneinfo import ZoneInfo" },
    { kind: "same", text: "" },
  ],
};

/** Claude asks one question (AskUserQuestion), with an answer of one's own allowed. */
export const DEMO_AGENT_QUESTION: PermissionRequest = {
  id: "question-1",
  source: "claude-code",
  project: "epilot",
  title: "Claude te pose une question",
  detail: "Quelle bibliothèque de dates garder pour l'API ?",
  reason: null,
  truncated: false,
  extras: [],
  canAlways: false,
  receivedAt: 0,
  timeoutMs: 55_000,
  risk: "low",
  warnings: [],
  folder: "~/dev/epilot",
  questions: [
    {
      question: "Quelle bibliothèque de dates garder pour l'API ?",
      header: "Dates",
      options: [
        { label: "date-fns", description: "Des fonctions séparées, légère." },
        { label: "Luxon", description: "Les fuseaux horaires intégrés." },
        { label: "Temporal", description: "L'API native, encore récente." },
      ],
      multiSelect: false,
      freeText: true,
    },
  ],
};

/** Claude asks two questions, the second with several choices. */
export const DEMO_QUESTIONS: PermissionRequest = {
  ...DEMO_AGENT_QUESTION,
  id: "question-2",
  title: "Claude te pose 2 questions",
  questions: [
    DEMO_AGENT_QUESTION.questions![0],
    {
      question: "Quelles vérifications lancer avant chaque commit ?",
      header: "Vérifs",
      options: [
        { label: "Tests", description: "Vitest sur les fichiers changés." },
        { label: "Lint", description: "ESLint, sans corriger tout seul." },
        { label: "Types", description: "svelte-check et tsc." },
      ],
      multiSelect: true,
      freeText: true,
    },
  ],
};

/** A Codex request (tests/fixtures/codex/PermissionRequest.json): same card as Claude's,
 * without « Toujours » (Codex refuses `updatedPermissions`). */
export const DEMO_CODEX_PERMISSION: PermissionRequest = {
  ...DEMO_SHORT_PERMISSION,
  id: "codex-perm",
  source: "codex",
  project: "demo",
  title: "Codex veut exécuter une commande",
  detail: "touch /home/user/projects/allowed-by-ilot.txt",
  reason: "Créer un fichier de test hors du dossier de travail ?",
  canAlways: false,
  folder: "~/projects/demo",
};

/** The hub's scene (09, section 8 bis): Claude edits, Gemini failed, Codex runs a build, a
 * generic tool rests; the same stories as the fake hour below. */
export const DEMO_HUB: AgentSession[] = [
  DEMO_TRIO[0],
  { ...DEMO_TRIO[1], status: "failed", activity: { verb: "lance", target: "make tests_run" } },
  { ...DEMO_TRIO[2], task: "vérifie le build du relais", activity: { verb: "lance", target: "npm run build" } },
  { ...DEMO_TRIO[3], status: "paused" },
];

/** The demo's live session, staged by the browser preview over Claude's session: îlot never
 * reads an agent's files, the app shows its journal instead. */
export const DEMO_HUB_STAGE: HubStage = {
  sessionId: DEMO_TRIO[0].id,
  file: "api.py",
  path: "epilot/api.py",
  lang: "PY",
  lines: [
    { n: 10, tokens: [["k", "from"], ["", " datetime "], ["k", "import"], ["", " datetime"]] },
    { n: 11, tokens: [] },
    { n: 12, tokens: [["k", "def"], ["", " horodatage():"]] },
    { n: 13, tokens: [["", "    "], ["k", "return"], ["", " datetime.now()"]] },
    { n: 14, tokens: [] },
    { n: 15, tokens: [["k", "def"], ["", " serialise(evenement):"]] },
    {
      n: 16,
      tokens: [["", "    "], ["k", "return"], ["", " {"], ["s", '"id"'], ["", ": evenement.id, "], ["s", '"at"'], ["", ": horodatage().isoformat()}"]],
    },
    { n: 17, tokens: [] },
    { n: 18, tokens: [["c", "# la page Historique lit ces dates"]] },
  ],
  edits: [
    { line: 10, text: "from datetime import datetime, timezone" },
    { line: 13, text: "    return datetime.now(timezone.utc)  # fuseau explicite" },
  ],
  snippet: [12, 13],
  terminal: [
    { text: "pytest tests/test_api.py -q", tone: "prompt" },
    { text: "  12 tests collectés", tone: "dim" },
    { text: "  ✓ horodatage en UTC  4 ms", tone: "ok" },
    { text: "  ✓ sérialise les dates ISO  3 ms", tone: "ok" },
    { text: "  ✓ garde le fuseau au retour  5 ms", tone: "ok" },
    { text: " tests/test_api.py", tone: "pass" },
    { text: "  Tests : 12 passés, 12 au total" },
  ],
};

/** The last hour of the fake AIs, in minutes ago: stretches of work (the last one stays
 * open when its AI still works), marks, and the latest actions of each card. An AI
 * left out had a quiet hour. */
export const DEMO_TIMELINE: Partial<Record<
  AiSource,
  {
    segments: [number, number, string][];
    marks: [number, "done" | "failed" | "permission", string, DecisionKind?][];
    steps: [number, string][];
  }
>> = {
  "claude-code": {
    segments: [
      [58, 41, "Refonte des tokens"],
      [38, 2, "Corrige le fuseau horaire de l'API"],
    ],
    marks: [
      [41, "done", "epilot, +42 −7"],
      [24, "permission", "npm run test", "allow"],
    ],
    steps: [
      [2, "Édite api.py"],
      [24, "Lance npm run test, autorisé par toi"],
      [41, "A terminé : refonte des tokens"],
      [58, "Lit docs/09-DEMO-FINALE.md"],
    ],
  },
  "gemini-cli": {
    segments: [
      [55, 47, "Lecture de la doc"],
      [44, 9, "Tests du parseur de cartes"],
      [6, 1, "Écrit les tests du parseur de cartes"],
    ],
    marks: [
      [47, "done", "my_rpg, lecture terminée"],
      [9, "failed", "make tests_run, 2 tests en échec"],
    ],
    steps: [
      [1, "Lit map_parser.c"],
      [9, "A échoué : make tests_run, 2 tests en échec"],
      [44, "Relance les tests du parseur"],
      [47, "A terminé : lecture de la doc"],
    ],
  },
  codex: {
    segments: [
      [50, 36, "Build du site"],
      [33, 12, "Documente le relais ilot-hook"],
    ],
    marks: [
      [36, "done", "site, build réussi"],
      [31, "permission", "npm install", "allow"],
      [12, "done", "ilot, +40 −2"],
    ],
    steps: [
      [12, "A terminé : documente le relais ilot-hook"],
      [31, "npm install, autorisé par toi"],
      [36, "Build du site réussi"],
      [50, "Démarre la session"],
    ],
  },
  generic: {
    segments: [[29, 18, "npm run build"]],
    marks: [[18, "done", "site, npm run build"]],
    steps: [[18, "A terminé : npm run build"]],
  },
  cursor: {
    segments: [[21, 0, "Refait la page d'accueil"]],
    marks: [],
    steps: [
      [1, "Édite Hero.tsx"],
      [12, "Lance npm run dev"],
      [21, "Lit src/pages/index.tsx"],
    ],
  },
  copilot: {
    segments: [[14, 0, "Ajoute les tests du parseur"]],
    marks: [],
    steps: [
      [2, "Lance npm test"],
      [9, "Délègue Test writer"],
    ],
  },
  opencode: {
    segments: [[48, 33, "Range les notes de réunion"]],
    marks: [[33, "done", "notes, +18 −4"]],
    steps: [[33, "A terminé : range les notes de réunion"]],
  },
  kiro: {
    segments: [[26, 0, "Met à jour le pipeline"]],
    marks: [],
    steps: [
      [3, "Attend ta réponse dans son terminal"],
      [17, "Édite ci.yml"],
    ],
  },
};

/** The decisions journal of the fake run, in minutes ago. */
export const DEMO_DECISIONS: [number, AiSource, string, string, DecisionKind][] = [
  [24, "claude-code", "epilot", "npm run test", "allow"],
  [31, "codex", "ilot", "npm install", "allow"],
  [52, "claude-code", "epilot", "git push --force origin main", "deny"],
];

/** Same rules as `default_rules()` in src-tauri/src/notifications.rs (the core sends the real ones). */
export const DEMO_NOTIFICATION_RULES: AppRule[] = [
  { id: "whatsapp", label: "WhatsApp", icon: "Wa", appNames: ["WhatsApp"], contains: [], programs: [] },
  { id: "discord", label: "Discord", icon: "Di", appNames: ["Discord"], contains: [], programs: [] },
  { id: "snapchat", label: "Snapchat", icon: "Sn", appNames: ["Snapchat"], contains: [], programs: [] },
  { id: "telegram", label: "Telegram", icon: "Tg", appNames: ["Telegram"], contains: [], programs: [] },
  { id: "messenger", label: "Messenger", icon: "Me", appNames: ["Messenger"], contains: [], programs: [] },
  { id: "instagram", label: "Instagram", icon: "Ig", appNames: ["Instagram"], contains: [], programs: [] },
  { id: "teams", label: "Teams", icon: "Tm", appNames: ["Microsoft Teams", "Teams"], contains: [], programs: [] },
  { id: "slack", label: "Slack", icon: "Sl", appNames: ["Slack"], contains: [], programs: [] },
  { id: "gmail", label: "Gmail", icon: "Gm", appNames: ["Gmail"], contains: [], programs: [] },
  { id: "web", label: "Sites web", icon: "Wb", appNames: ["Google Chrome", "Microsoft Edge", "Firefox"], contains: [], programs: [] },
];

const message = (id: string, app: "whatsapp" | "discord", sender: string, text: string): NotificationItem => ({
  id,
  app,
  appLabel: app === "whatsapp" ? "WhatsApp" : "Discord",
  monogram: app === "whatsapp" ? "Wa" : "Di",
  sender,
  text,
  masked: false,
  count: 1,
  receivedAt: 0,
});

/** The four banners of the phase 6 screenshots. */
export const DEMO_BANNERS = {
  short: message("n-1", "whatsapp", "Maman", "Tu rentres à quelle heure ?"),
  long: message(
    "n-2",
    "discord",
    "kenji (#général, Projet RPG)",
    "J'ai poussé la nouvelle carte, tu peux tester le parseur ce soir ? Il plante encore sur les tuiles vides.",
  ),
  group: { ...message("n-3", "discord", "léa", "on lance la partie ?"), count: 3 },
  masked: { ...message("n-4", "whatsapp", "Maman", ""), masked: true },
  /** A message in English, for « Traduire » (DeepL, prompt 12). */
  foreign: message("n-5", "discord", "kenji (#général, Projet RPG)", "Pushed the new map, can you test the parser tonight? It still crashes on empty tiles."),
} satisfies Record<string, NotificationItem>;

export const DEMO_POSTPONED: NotificationItem[] = [DEMO_BANNERS.short, DEMO_BANNERS.group];

/** Messages tab of the preview, oldest first: who wrote, how long ago, and the reply the
 * user copied (`mine`). `read` marks the thread as read once that message is in. */
export const DEMO_CONVERSATION: {
  app: "whatsapp" | "discord" | "snapchat";
  sender: string;
  text: string;
  minutesAgo: number;
  mine?: boolean;
  read?: boolean;
}[] = [
  { app: "snapchat", sender: "Awa", text: "Nouveau snap. Ouvre Snapchat pour le voir.", minutesAgo: 95, read: true },
  { app: "discord", sender: "Kenji dans #dev-ilot", text: "Tu peux relire la PR des ressorts avant ce soir ?", minutesAgo: 48, read: true },
  { app: "discord", sender: "Kenji dans #dev-ilot", text: "La PR des ressorts est prête.", minutesAgo: 42 },
  { app: "whatsapp", sender: "Léa Ahouansou", text: "Tu viens ce soir ?", minutesAgo: 14 },
  { app: "whatsapp", sender: "Léa Ahouansou", text: "Oui, je finis vers 18h", minutesAgo: 13, mine: true },
  { app: "whatsapp", sender: "Léa Ahouansou", text: "On se voit à 18h ? J'ai pris les places pour le concert.", minutesAgo: 2 },
];

/** A risky command (« Risque élevé », 620 px): a forced push. */
export const DEMO_RISKY_PERMISSION: PermissionRequest = {
  ...DEMO_SHORT_PERMISSION,
  id: "demo-risky",
  detail: "git push --force origin main",
  canAlways: false,
  risk: "high",
  warnings: ["Pousse sur GitHub en forçant : l'historique distant sera écrasé."],
};

/** A failed agent, for the « … a échoué » opening. */
export const DEMO_FAILED: Failed = { source: "gemini-cli", project: "notes", detail: "npm test, 2 tests en échec" };

const MIN = 60_000;

/** One day of the report: minutes worked, then the counters in the order of `DayCounters`. */
function demoDay(...[minutes, sessions, files, added, removed, commands, allowed, denied]: [number, number, number, number, number, number, number, number]): DayCounters {
  return { activeMs: minutes * MIN, sessions, files, added, removed, commands, allowed, denied };
}

/** Last week as the Monday report shows it (counters and project names only). */
export const DEMO_WEEK: WeekReport = {
  monday: "2026-10-05",
  days: [
    demoDay(130, 5, 14, 412, 120, 38, 6, 1),
    demoDay(205, 7, 22, 861, 302, 71, 11, 2),
    demoDay(100, 3, 9, 230, 88, 25, 4, 0),
    demoDay(175, 6, 17, 640, 211, 52, 9, 1),
    demoDay(75, 4, 8, 190, 64, 19, 3, 1),
    demoDay(20, 1, 2, 35, 4, 6, 1, 0),
    demoDay(0, 0, 0, 0, 0, 0, 0, 0),
  ],
  total: demoDay(705, 26, 72, 2368, 789, 211, 34, 5),
  agents: [
    { name: "claude-code", ms: 470 * MIN },
    { name: "codex", ms: 160 * MIN },
    { name: "gemini-cli", ms: 75 * MIN },
  ],
  projects: [
    { name: "epilot", ms: 380 * MIN },
    { name: "ilot", ms: 215 * MIN },
    { name: "site-vitrine", ms: 110 * MIN },
  ],
  busiestDay: 1,
  longest: { ms: 112 * MIN, source: "claude-code", project: "epilot" },
};

/** This week so far: Monday only. */
export const DEMO_THIS_WEEK: WeekReport = {
  monday: "2026-10-12",
  days: [demoDay(48, 2, 5, 96, 31, 12, 2, 0), ...Array.from({ length: 6 }, () => demoDay(0, 0, 0, 0, 0, 0, 0, 0))],
  total: demoDay(48, 2, 5, 96, 31, 12, 2, 0),
  agents: [{ name: "claude-code", ms: 48 * MIN }],
  projects: [{ name: "ilot", ms: 48 * MIN }],
  busiestDay: 0,
  longest: { ms: 31 * MIN, source: "claude-code", project: "ilot" },
};
