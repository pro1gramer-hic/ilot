// Pastilles of the island: which ones are on (settings.pastilles), what the core read for
// each (`pastilles:changed`), and which one the pill shows at rest. `nextPastille()` moves
// to the next one (the lot 5 shortcut « changer de pastille » calls it).

import {
  ollamaPull,
  ollamaPullCancel,
  onCoreEvent,
  pastilleRefresh,
  pastilleShowTool,
  pastillesCatalog,
  pastillesSnapshot,
  secretRemove,
  secretStore,
} from "../ipc";
import { texts } from "../i18n/live.svelte";
import { sessionOf } from "../media";
import {
  NO_PASTILLES,
  type PastilleExtra,
  type PastilleSettings,
  type PastilleView,
  type ServiceInfo,
  type ToolId,
  activeIds,
  checkBaseUrl,
  nextId,
  spotifyView,
  tokenName,
  validModel,
  withExtra,
  withoutExtra,
} from "../pastilles";
import { media } from "./media.svelte";
import { settings } from "./settings.svelte";

/** What the store asks of the core; the browser preview and the tests give their own. */
export type PastillesCore = {
  snapshot(): Promise<PastilleView[]>;
  catalog(): Promise<ServiceInfo[]>;
  refresh(id: string): Promise<void>;
  showTool(): Promise<void>;
  pull(model: string): Promise<void>;
  cancelPull(): Promise<void>;
  storeToken(id: string, token: string): Promise<void>;
  forgetToken(id: string): Promise<void>;
};

const TAURI_CORE: PastillesCore = {
  snapshot: pastillesSnapshot,
  catalog: pastillesCatalog,
  refresh: pastilleRefresh,
  showTool: pastilleShowTool,
  pull: ollamaPull,
  cancelPull: ollamaPullCancel,
  storeToken: (id, token) => secretStore(tokenName(id), token),
  forgetToken: (id) => secretRemove(tokenName(id)),
};

/** What the settings form gives for a service. */
export type ServiceForm = { token: string; baseUrl: string; account: string };

export class PastillesStore {
  /** What the core read, one view per active pastille (Spotify aside). */
  views = $state<PastilleView[]>([]);
  catalog = $state<ServiceInfo[]>([]);
  /** The pastille chosen with the hub's tiles or `nextPastille()`. */
  current = $state<string | null>(null);
  /** The last action the core refused, said for a few seconds in the settings or the hub. */
  notice = $state<string | null>(null);

  #core: PastillesCore = TAURI_CORE;
  #noticeTimer: ReturnType<typeof setTimeout> | undefined;

  /** Settings written before the pastilles existed have none. */
  config: PastilleSettings = $derived(settings.value.pastilles ?? NO_PASTILLES);
  /** The pastilles on, the main tool first. */
  ids: string[] = $derived(activeIds(this.config));
  /** The pastille the pill shows at rest and the hub opens on. */
  shown: string | null = $derived(this.current !== null && this.ids.includes(this.current) ? this.current : (this.ids[0] ?? null));

  /** Used by the browser preview and the tests. */
  useCore(core: PastillesCore): void {
    this.#core = core;
  }

  /** Starts following the core: what it reads, then the catalog of services. */
  async connect(): Promise<void> {
    onCoreEvent("pastilles:changed", (views) => (this.views = views));
    await Promise.all([this.load(), this.loadCatalog()]);
  }

  async load(): Promise<void> {
    this.views = await this.#core.snapshot();
  }

  async loadCatalog(): Promise<void> {
    this.catalog = await this.#core.catalog();
  }

  /** A pastille's view: what the core read, or Spotify from the media sessions. */
  view(id: string): PastilleView | undefined {
    if (id === "spotify") return spotifyView(sessionOf(media.sessions, "Spotify"));
    return this.views.find((v) => v.id === id);
  }

  /** « Changer de pastille »: the next one becomes the pill's. */
  nextPastille(): string | null {
    this.current = nextId(this.ids, this.shown);
    return this.current;
  }

  select(id: string): void {
    if (this.ids.includes(id)) this.current = id;
  }

  async setTool(tool: ToolId | ""): Promise<void> {
    await settings.update({ pastilles: { ...this.config, tool } });
  }

  /** Ollama, LM Studio or Spotify: no token, on at once. */
  async addLocal(id: string): Promise<void> {
    await settings.update({ pastilles: withExtra(this.config, { id, baseUrl: "", account: "" }) });
  }

  /** Checks the form, stores the token in the vault, then turns the pastille on. A
   * message says what is wrong; null when the pastille is on. */
  async addService(service: ServiceInfo, form: ServiceForm): Promise<string | null> {
    const token = form.token.trim();
    if (!token && !service.hasToken) return texts().common.pastilles.pasteToken;
    let baseUrl = "";
    if (service.customUrl !== undefined && (form.baseUrl.trim() || service.urlRequired)) {
      const checked = checkBaseUrl(form.baseUrl || "");
      if ("error" in checked) return checked.error;
      baseUrl = checked.url;
    }
    const account = form.account.trim();
    if (service.account && !account) return texts().common.pastilles.fill(service.account.label);
    const extra: PastilleExtra = { id: service.id, baseUrl, account };
    try {
      if (token) await this.#core.storeToken(service.id, token);
      await settings.update({ pastilles: withExtra(this.config, extra) });
      if (settings.error) return settings.error;
      if (!this.config.extras.some((e) => e.id === service.id)) return texts().common.pastilles.notKept;
      await this.loadCatalog();
      return null;
    } catch (error) {
      return String(error);
    }
  }

  /** Turns a pastille off; a service's token is forgotten with it. */
  async remove(id: string): Promise<void> {
    await settings.update({ pastilles: withoutExtra(this.config, id) });
    if (this.catalog.some((s) => s.id === id)) {
      await this.#attempt(() => this.#core.forgetToken(id));
      await this.loadCatalog();
    }
  }

  /** « Réessayer » on a pastille in trouble. */
  async retry(id: string): Promise<void> {
    await this.#attempt(() => this.#core.refresh(id));
  }

  /** Brings the main tool to the front. */
  async showTool(): Promise<void> {
    await this.#attempt(() => this.#core.showTool());
  }

  /** Downloads an Ollama model the user named. */
  async pull(model: string): Promise<boolean> {
    if (!validModel(model)) {
      this.#say(texts().common.pastilles.badModel);
      return false;
    }
    return this.#attempt(() => this.#core.pull(model.trim()));
  }

  async cancelPull(): Promise<void> {
    await this.#attempt(() => this.#core.cancelPull());
  }

  async #attempt(action: () => Promise<void>): Promise<boolean> {
    try {
      await action();
      this.#say(null);
      return true;
    } catch (error) {
      this.#say(String(error));
      return false;
    }
  }

  #say(message: string | null): void {
    clearTimeout(this.#noticeTimer);
    this.notice = message;
    if (message) this.#noticeTimer = setTimeout(() => (this.notice = null), 5000);
  }
}

export const pastilles = new PastillesStore();
