import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_SETTINGS } from "../ipc";
import { DEMO_CATALOG } from "../pastilles-preview";
import type { PastilleView, ServiceInfo } from "../pastilles";
import { type PastillesCore, PastillesStore } from "./pastilles.svelte";
import { settings } from "./settings.svelte";

/** A core that remembers what it was asked, and the tokens it « stored ». */
function fakeCore(): PastillesCore & { calls: string[]; vault: Map<string, string>; views: PastilleView[] } {
  const core = {
    calls: [] as string[],
    vault: new Map<string, string>(),
    views: [] as PastilleView[],
    snapshot: async () => core.views,
    catalog: async () => DEMO_CATALOG.map((s) => ({ ...s, hasToken: core.vault.has(s.id) })),
    refresh: async (id: string) => void core.calls.push(`refresh ${id}`),
    showTool: async () => void core.calls.push("show tool"),
    pull: async (model: string) => void core.calls.push(`pull ${model}`),
    cancelPull: async () => void core.calls.push("cancel pull"),
    storeToken: async (id: string, token: string) => {
      core.calls.push(`store ${id}`);
      core.vault.set(id, token);
    },
    forgetToken: async (id: string) => {
      core.calls.push(`forget ${id}`);
      core.vault.delete(id);
    },
  };
  return core;
}

const service = (id: string): ServiceInfo => DEMO_CATALOG.find((s) => s.id === id)!;
const form = (token: string, baseUrl = "", account = "") => ({ token, baseUrl, account });

describe("pastilles store", () => {
  let store: PastillesStore;
  let core: ReturnType<typeof fakeCore>;

  beforeEach(async () => {
    // The settings store marks the theme on the page: a page is all it needs here.
    vi.stubGlobal("document", { documentElement: { dataset: {} } });
    settings.apply(structuredClone(DEFAULT_SETTINGS));
    store = new PastillesStore();
    core = fakeCore();
    store.useCore(core);
    await store.loadCatalog();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("starts with no pastille and nothing to show", () => {
    expect(store.ids).toEqual([]);
    expect(store.shown).toBeNull();
    expect(store.nextPastille()).toBeNull();
  });

  it("the pill follows the chosen pastille, and nextPastille goes round", async () => {
    await store.setTool("vscode");
    await store.addLocal("ollama");
    await store.addLocal("spotify");
    expect(store.ids).toEqual(["tool", "ollama", "spotify"]);
    expect(store.shown).toBe("tool");
    expect(store.nextPastille()).toBe("ollama");
    expect(store.nextPastille()).toBe("spotify");
    expect(store.nextPastille()).toBe("tool");
    store.select("spotify");
    expect(store.shown).toBe("spotify");
    store.select("absent");
    expect(store.shown).toBe("spotify");
    // A pastille removed: the pill falls back on the first one.
    await store.remove("spotify");
    expect(store.shown).toBe("tool");
  });

  it("a token goes to the vault, never into the settings", async () => {
    const error = await store.addService(service("github"), form("  ghp_secret123  "));
    expect(error).toBeNull();
    expect(core.vault.get("github")).toBe("ghp_secret123");
    expect(store.ids).toEqual(["github"]);
    expect(JSON.stringify(settings.value)).not.toContain("ghp_secret123");
    expect(store.catalog.find((s) => s.id === "github")?.hasToken).toBe(true);
    // Removing the pastille forgets its token.
    await store.remove("github");
    expect(core.vault.has("github")).toBe(false);
    expect(core.calls).toContain("forget github");
  });

  it("the form says what is missing, and nothing is stored then", async () => {
    expect(await store.addService(service("github"), form(""))).toBe("Colle le jeton du service.");
    expect(await store.addService(service("dokploy"), form("k", ""))).toMatch(/Adresse invalide/);
    expect(await store.addService(service("n8n"), form("k", "http://n8n.example.com"))).toMatch(/non chiffrée/);
    expect(await store.addService(service("cloudflare"), form("k", "", " "))).toMatch(/Identifiant du compte/);
    expect(core.vault.size).toBe(0);
    expect(store.ids).toEqual([]);
    expect(await store.addService(service("dokploy"), form("k", "https://deploy.example.com/"))).toBeNull();
    expect(settings.value.pastilles.extras).toEqual([{ id: "dokploy", baseUrl: "https://deploy.example.com", account: "" }]);
  });

  it("downloads only a model with a valid name", async () => {
    expect(await store.pull("../../x")).toBe(false);
    expect(store.notice).toMatch(/Nom de modèle invalide/);
    expect(await store.pull("gemma4")).toBe(true);
    expect(core.calls).toEqual(["pull gemma4"]);
  });

  it("a refusal of the core is said, not thrown", async () => {
    store.useCore({ ...core, showTool: async () => Promise.reject("VS Code n'est pas ouvert.") });
    await store.showTool();
    expect(store.notice).toBe("VS Code n'est pas ouvert.");
  });

  it("Spotify's view comes from the media sessions, the others from the core", async () => {
    core.views = [{ id: "ollama", tone: "off", line: "Ollama n'est pas lancé", short: "Ollama", metrics: [], items: [] }];
    await store.load();
    expect(store.view("ollama")?.line).toBe("Ollama n'est pas lancé");
    expect(store.view("spotify")?.line).toBe("Spotify ne joue rien");
    expect(store.view("github")).toBeUndefined();
  });
});
