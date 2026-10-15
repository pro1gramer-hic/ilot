// Browser preview of the pastilles (?pastilles=demo|off|pull|error|services|local): fake
// readings and a fake core that answers like the Rust one (src-tauri/src/pastilles/), so
// the preview and Playwright run the real store and components. No network, no vault.

import type { PastilleSettings, PastilleView, PullView, ServiceInfo } from "./pastilles";
import type { PastillesStore } from "./stores/pastilles.svelte";

export const PASTILLE_SCENES = ["none", "demo", "off", "pull", "error", "services", "local"] as const;

export type PastilleScene = (typeof PASTILLE_SCENES)[number];

export function readPastilleScene(value: string | null): PastilleScene {
  return PASTILLE_SCENES.find((s) => s === value) ?? "none";
}

/** ?pastilles=…: which pastilles are on and what they read; ?modes=off: Spotify does not
 * let others change its shuffle and repeat (nothing documents which one it does). */
export function readPastillePreview(search: string): { pastilles: PastilleScene; playModes: boolean } {
  const params = new URLSearchParams(search);
  return { pastilles: readPastilleScene(params.get("pastilles")), playModes: params.get("modes") !== "off" };
}

/** The services of src-tauri/src/pastilles/services (same ids, names and Mochi). */
export const DEMO_CATALOG: ServiceInfo[] = [
  {
    id: "github",
    name: "GitHub",
    palette: "nuit",
    kind: "meche",
    customUrl: "https://api.github.com",
    urlRequired: false,
    refreshMinutes: 3,
    tokenLabel: "Jeton d'accès fin (fine-grained)",
    rights: "Lecture seule : Pull requests et Commit statuses en lecture, sur les dépôts choisis.",
    help: "https://github.com/settings/personal-access-tokens/new",
    hasToken: false,
  },
  {
    id: "gitlab",
    name: "GitLab",
    palette: "peche",
    kind: "oreilles",
    customUrl: "https://gitlab.com",
    urlRequired: false,
    refreshMinutes: 3,
    tokenLabel: "Jeton d'accès personnel",
    rights: "Portée read_api seulement (lecture de l'API).",
    help: "https://docs.gitlab.com/user/profile/personal_access_tokens/",
    hasToken: false,
  },
  {
    id: "vercel",
    name: "Vercel",
    palette: "nuit",
    kind: "ecran",
    urlRequired: false,
    refreshMinutes: 2,
    tokenLabel: "Jeton d'accès",
    rights: "Portée la plus étroite possible : un seul projet, ou une équipe.",
    warning: "Vercel ne propose pas de jeton en lecture seule : îlot ne fait que lire, mais limite sa portée à un projet.",
    help: "https://vercel.com/account/settings/tokens",
    hasToken: false,
  },
  {
    id: "cloudflare",
    name: "Cloudflare",
    palette: "citron",
    kind: "oreilles",
    urlRequired: false,
    account: {
      label: "Identifiant du compte",
      hint: "32 caractères, visibles dans le tableau de bord, à droite de la page d'accueil du compte.",
    },
    refreshMinutes: 5,
    tokenLabel: "Jeton d'API personnalisé",
    rights: "Lecture seule, limitée à ce compte : Cloudflare Pages (Read), Workers Scripts (Read), Notifications (Read).",
    help: "https://dash.cloudflare.com/profile/api-tokens",
    hasToken: false,
  },
  {
    id: "dokploy",
    name: "Dokploy",
    palette: "ciel",
    kind: "ecran",
    customUrl: "http://localhost:3000",
    urlRequired: true,
    refreshMinutes: 2,
    tokenLabel: "Clé d'API (Profil, section API/CLI)",
    rights: "Dokploy ne propose pas de clé en lecture seule : îlot ne fait que lire.",
    warning:
      "La clé Dokploy donne tous les droits du compte. îlot ne fait que des lectures, mais garde-la pour îlot seul et donne-lui une date d'expiration.",
    help: "https://docs.dokploy.com/docs/api",
    hasToken: false,
  },
  {
    id: "dockerhub",
    name: "Docker Hub",
    palette: "ciel",
    kind: "meche",
    urlRequired: false,
    account: { label: "Nom d'utilisateur Docker Hub", hint: "Celui du jeton : îlot liste les images de ce compte." },
    refreshMinutes: 30,
    tokenLabel: "Jeton d'accès personnel (dckr_pat_…)",
    rights: "Repo Read-only (ou Public Repo Read-only si tout est public).",
    help: "https://app.docker.com/settings/personal-access-tokens",
    hasToken: false,
  },
  {
    id: "stripe",
    name: "Stripe",
    palette: "lavande",
    kind: "oreilles",
    urlRequired: false,
    refreshMinutes: 10,
    tokenLabel: "Clé restreinte (rk_live_…)",
    rights: "Clé restreinte : PaymentIntents en lecture (Read), tout le reste sur None.",
    help: "https://dashboard.stripe.com/apikeys",
    hasToken: false,
  },
  {
    id: "n8n",
    name: "n8n",
    palette: "peche",
    kind: "ecran",
    customUrl: "http://localhost:5678",
    urlRequired: true,
    refreshMinutes: 2,
    tokenLabel: "Clé d'API (Settings, n8n API)",
    rights: "Sur Enterprise : portées execution:read seulement. Sinon la clé a tous les droits : donne-lui une date d'expiration.",
    warning: "Hors Enterprise, une clé n8n donne tous les droits du compte. îlot ne fait que lire les exécutions.",
    help: "https://docs.n8n.io/api/authentication/",
    hasToken: false,
  },
  {
    id: "resend",
    name: "Resend",
    palette: "lavande",
    kind: "ecran",
    urlRequired: false,
    refreshMinutes: 5,
    tokenLabel: "Clé d'API (re_…)",
    rights: "Resend n'a pas de clé en lecture seule : lister les e-mails demande sans doute une clé « Full access ».",
    warning:
      "Attention : Resend ne propose aucune clé en lecture seule. Une clé « Full access » peut aussi envoyer et supprimer. îlot ne fait que lire, mais crée une clé pour îlot seul.",
    help: "https://resend.com/api-keys",
    hasToken: false,
  },
  {
    id: "notion",
    name: "Notion",
    palette: "citron",
    kind: "meche",
    urlRequired: false,
    refreshMinutes: 5,
    tokenLabel: "Secret de connexion interne",
    rights: "Capacité « Read content » seule, sans information sur les utilisateurs. Partage ensuite les pages voulues avec la connexion.",
    help: "https://www.notion.so/profile/integrations",
    hasToken: false,
  },
  {
    id: "calcom",
    name: "Cal.com",
    palette: "menthe",
    kind: "meche",
    customUrl: "https://api.cal.com",
    urlRequired: false,
    refreshMinutes: 10,
    tokenLabel: "Clé d'API (cal_live_…)",
    rights: "Cal.com n'a pas de clé en lecture seule : îlot ne fait que lire les rendez-vous à venir.",
    warning: "Une clé Cal.com a de nombreux droits. îlot ne fait que lire tes prochains rendez-vous : donne-lui une date d'expiration.",
    help: "https://app.cal.com/settings/developer/api-keys",
    hasToken: false,
  },
];

const MIN = 60_000;

/** What a fake reading of each pastille says, `now` being the preview's clock. */
export function demoView(id: string, now: number, scene: PastilleScene = "demo"): PastilleView {
  const base = { id, metrics: [], items: [], checkedAt: now - 2 * MIN };
  switch (id) {
    case "tool":
      return scene === "off"
        ? { ...base, tone: "off", line: "VS Code n'est pas ouvert", short: "VS Code", tool: { open: false, focused: false, candidates: [] } }
        : {
            ...base,
            tone: "ok",
            line: "VS Code ouvert sur epilot",
            short: "epilot",
            tool: { open: true, focused: false, project: "epilot", candidates: ["lib.rs", "epilot"] },
          };
    case "ollama":
      if (scene === "off") return { ...base, tone: "off", line: "Ollama n'est pas lancé", short: "Ollama", models: [] };
      return {
        ...base,
        tone: "busy",
        line: "gemma4 en mémoire",
        short: "gemma4",
        models: [
          { name: "gemma4", size: 6_591_830_464, loaded: true, vram: 5_333_539_264, until: now + 4 * MIN, details: "8.0B, Q4_K_M" },
          { name: "llama3.2:3b", size: 2_019_393_189, loaded: false, details: "3.2B" },
          { name: "qwen3:8b", size: 5_200_000_000, loaded: false, details: "8.2B" },
        ],
      };
    case "lmstudio":
      return {
        ...base,
        tone: "idle",
        line: "Aucun modèle en mémoire, 2 installés",
        short: "LM Studio",
        models: [
          { name: "google/gemma-4-26b-a4b", size: 17_990_911_801, loaded: false, details: "Gemma 4 26B A4B" },
          { name: "qwen/qwen3-8b", size: 5_100_000_000, loaded: false, details: "Qwen3 8B" },
        ],
      };
    case "github":
      if (scene === "error") {
        return { ...base, tone: "err", line: "Jeton refusé : vérifie ses droits", short: "GitHub", problem: "Jeton refusé : vérifie ses droits" };
      }
      return {
        ...base,
        tone: "warn",
        line: "3 PR ouvertes, 1 relecture demandée",
        short: "1 à relire",
        metrics: [
          { label: "PR ouvertes", value: "3" },
          { label: "À relire", value: "1", tone: "warn" },
          { label: "CI en échec", value: "0", tone: "ok" },
        ],
        items: [
          { title: "Pastilles de services", detail: "hic-69/ilot", at: now - 12 * MIN },
          { title: "Corrige le parseur de hooks", detail: "hic-69/epilot", at: now - 3 * 60 * MIN },
        ],
      };
    case "stripe":
      return {
        ...base,
        tone: "ok",
        line: "4 paiements aujourd'hui, 162,50 €",
        short: "162,50 €",
        metrics: [
          { label: "Paiements", value: "4" },
          { label: "Montant du jour", value: "162,50 €" },
          { label: "Refusés", value: "0", tone: "idle" },
        ],
        items: [
          { title: "42,00 €", detail: "Abonnement Pro", at: now - 38 * MIN, tone: "ok" },
          { title: "120,50 €", detail: "Paiement de 10:12", at: now - 3 * 60 * MIN, tone: "ok" },
        ],
      };
    case "vercel":
      return {
        ...base,
        tone: "busy",
        line: "Dernier déploiement en construction : site",
        short: "Déploiement…",
        metrics: [
          { label: "Derniers", value: "10" },
          { label: "En cours", value: "1", tone: "busy" },
          { label: "En échec", value: "0", tone: "ok" },
        ],
        items: [
          { title: "site", detail: "Production, en construction", at: now - 1 * MIN, tone: "busy" },
          { title: "docs", detail: "Aperçu, en ligne", at: now - 50 * MIN, tone: "ok" },
        ],
      };
    case "dokploy":
      return {
        ...base,
        tone: "err",
        line: "5 applications, 1 en échec",
        short: "1 en échec",
        metrics: [
          { label: "En ligne", value: "4", tone: "ok" },
          { label: "En cours", value: "0", tone: "idle" },
          { label: "En échec", value: "1", tone: "err" },
        ],
        items: [
          { title: "front", detail: "en échec", tone: "err" },
          { title: "api", detail: "en ligne", tone: "ok" },
        ],
      };
    case "n8n":
      return {
        ...base,
        tone: "off",
        line: "Injoignable pour l'instant",
        short: "n8n",
        problem: "Injoignable pour l'instant",
      };
    default:
      return { ...base, tone: "idle", line: "Lecture en cours…", short: "" };
  }
}

/** The pastilles on in each scene. */
export function sceneSettings(scene: PastilleScene): PastilleSettings {
  const extra = (id: string, baseUrl = "") => ({ id, baseUrl, account: "" });
  switch (scene) {
    case "none":
      return { tool: "", extras: [] };
    case "services":
      return { tool: "", extras: [extra("github"), extra("vercel"), extra("dokploy", "https://deploy.example.com"), extra("n8n", "https://n8n.example.com")] };
    case "error":
      return { tool: "vscode", extras: [extra("github"), extra("n8n", "https://n8n.example.com"), extra("ollama")] };
    case "local":
      return { tool: "cursor", extras: [extra("lmstudio"), extra("ollama"), extra("spotify")] };
    default:
      return { tool: "vscode", extras: [extra("ollama"), extra("github"), extra("stripe"), extra("spotify")] };
  }
}

/** A download half done, as the core tells it. */
export const DEMO_PULL: PullView = { model: "llama3.2:3b", step: "Téléchargement", completed: 842_000_000, total: 2_019_393_189, done: false };

/** Plays the core in the preview: readings for the scene, the catalog, tokens « stored »
 * in memory, a download that starts when asked. */
export function startFakePastilles(store: PastillesStore, scene: PastilleScene): void {
  const now = Date.now();
  let views: PastilleView[] = sceneSettings(scene)
    .extras.map((e) => e.id)
    .concat(scene === "services" || scene === "none" ? [] : ["tool"])
    .filter((id) => id !== "spotify")
    .map((id) => demoView(id, now, scene));
  if (scene === "pull") views = views.map((v) => (v.id === "ollama" ? { ...v, pull: DEMO_PULL } : v));
  let catalog = DEMO_CATALOG.map((s) => ({ ...s, hasToken: sceneSettings(scene).extras.some((e) => e.id === s.id) }));
  const push = (): void => {
    store.views = views;
  };
  store.useCore({
    snapshot: async () => views,
    catalog: async () => catalog,
    refresh: async () => {},
    showTool: async () => {},
    pull: async (model) => {
      views = views.map((v) => (v.id === "ollama" ? { ...v, pull: { model, step: "Préparation", completed: 0, total: 0, done: false } } : v));
      push();
    },
    cancelPull: async () => {
      views = views.map((v) => (v.id === "ollama" ? { ...v, pull: undefined } : v));
      push();
    },
    storeToken: async (id) => {
      catalog = catalog.map((s) => (s.id === id ? { ...s, hasToken: true } : s));
      // The first reading comes a moment later, as with the real core.
      setTimeout(() => {
        views = [...views.filter((v) => v.id !== id), demoView(id, Date.now())];
        push();
      }, 600);
    },
    forgetToken: async (id) => {
      catalog = catalog.map((s) => (s.id === id ? { ...s, hasToken: false } : s));
      views = views.filter((v) => v.id !== id);
      push();
    },
  });
  store.catalog = catalog;
  push();
}
