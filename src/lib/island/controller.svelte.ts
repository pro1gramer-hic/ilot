// The island's behavior (docs/09-DEMO-FINALE.md, sections 3, 4 and 8 bis): which opening
// it shows, the queue behind a permission, the hub that a click brings out of the pill,
// the large panel (« Réglages ») below or above it, where the core placed it, and who
// holds the keyboard. Geometry lives in model.ts, the queue's rules in banners.ts, the
// hub's in hub.ts.

import { untrack } from "svelte";
import type { HubStage, HubView } from "../hub";
import { texts } from "../i18n/live.svelte";
import { type IslandPointer, buddyRecall, focusIsland, setIslandShape } from "../ipc";
import { banners } from "../stores/banners.svelte";
import { bubble } from "../stores/bubble.svelte";
import { conversations } from "../stores/conversations.svelte";
import { notifications } from "../stores/notifications.svelte";
import { permissions } from "../stores/permissions.svelte";
import { settings } from "../stores/settings.svelte";
import type { Surface } from "./guard";
import {
  DEFAULT_PLACEMENT,
  HUB_RADIUS,
  type IslandMode,
  type Measures,
  PANEL_MAX,
  type Placement,
  type Shape,
  type Tab,
  hubBox,
  hubHeight,
  islandBox,
  panelBox,
  resolveMode,
  shapeFor,
  sizeFor,
} from "./model";

/** Openings Mochi comes back for when he is out of the island. */
const ALERTS: readonly IslandMode[] = ["permission", "message", "failed", "finished"];
/** How long a dot stays lit after an agent finished or failed. */
const FLASH_MS = 2600;

/** Why the hub opens with its intro: îlot just started (it then folds back on its own
 * unless used), or the user asked to see it again. */
type HubIntro = "launch" | "replay";

class IslandController {
  /** The large panel (« Réglages »), a card of its own under the pill (or above it). */
  panelOpen = $state(false);
  /** The hub, the compact panel a click on the pill brings out (09, section 8 bis). */
  hubOpen = $state(false);
  hubView = $state<HubView>("home");
  /** The intro plays over the hub's home while this is set. */
  hubIntro = $state<HubIntro | null>(null);
  /** The browser preview's staged live session (fake data); never set in the app. */
  hubStage = $state.raw<HubStage | null>(null);
  /** A session to put on the hub's left card as it opens (its `HubSession` key); the
   * hub takes it, then clears it. */
  hubFollow = $state<string | null>(null);
  /** What the live session shows on its right: the agent's journal, or its latest change. */
  liveTab = $state<"journal" | "diff">("journal");
  /** Quick menu opened by a right click on the pill. */
  menuOpen = $state(false);
  /** Mochi's wardrobe, opened by a right click on him (prompt 12, lot 1). */
  wardrobeOpen = $state(false);
  /** Mochi is out « Sur le bureau »: he stays there, alerts do not call him back. */
  mochiDesk = $state(false);
  /** « Transparente au repos »: the island is faded, in place (stores/rest.svelte.ts). */
  resting = $state(false);
  /** The launch greeting still has to play in the pill (the intro did not show). */
  greeting = $state(false);
  tab = $state<Tab>("today");
  /** The cursor rests on the island. */
  hovered = $state(false);
  /** The island's window holds the keyboard (tracked from window focus and blur). */
  focused = false;
  /** Where the core placed the pill in its window (`island:placed`). */
  placement = $state<Placement>({ ...DEFAULT_PLACEMENT });
  /** The cursor, as the core sees it (`island:pointer`): eyes, tilt, light, closeness. */
  pointer = $state<IslandPointer>({ x: 0, y: 0, inside: false, near: false });
  /** The pill follows the pointer while the user moves it (`island:drag`). */
  dragging = $state(false);
  /** A file is dragged over the island (only its name will ever be used). */
  dropping = $state(false);
  /** Height measured by the openings that fit their content. */
  measures = $state<Measures>({ contentHeight: 0 });
  /** The user holds the island down: it gives a little. */
  pressed = $state(false);
  /** How far Mochi is pulled down out of the pill, in px (the island stretches). */
  pull = $state(0);
  /** Mochi left for his own window (« Se balade », « Te suit »); he comes back on an alert. */
  mochiOut = $state(false);
  /** Kicks the island's height (a poke, a landing, a swallow); Island.svelte plugs its
   * spring in. */
  kick: (velocity: number) => void = () => {};
  /** A dot of the pill (an AI's, or a badge's: `dotKey`) flashes green (finished) or red
   * (failed) for a moment. */
  flashes = $state<Record<string, "ok" | "err">>({});
  #flashTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  /** The permission on screen: the oldest waiting, or the one just answered while its
   * answer plays (the island then folds). */
  permission = $derived(permissions.current ?? permissions.answered?.request);

  /** Messages waiting while the « Pastille » mode keeps them out of the island. */
  unread = $derived(settings.value.modes.messages === "discret" ? notifications.unread : 0);

  mode: IslandMode = $derived(
    resolveMode({
      // With the panel open, a request shows in its Agents tab: the island stays folded.
      permission: this.permission !== undefined && !this.panelOpen,
      drop: this.dropping,
      menuOpen: this.menuOpen,
      wardrobeOpen: this.wardrobeOpen,
      event: banners.current?.kind ?? null,
    }),
  );

  size = $derived(
    sizeFor(this.mode, {
      modes: settings.value.modes,
      badge: this.unread > 0,
      risky: this.permission?.risk === "high",
      measures: this.measures,
    }),
  );

  /** The permission whose answer buttons count, and where: the panel's Agents tab when it
   * shows the request, else the island. */
  answering = $derived.by((): { surface: Surface; id: string } | undefined => {
    const request = permissions.current;
    if (!request) return undefined;
    if (this.panelOpen && this.tab === "agents") return { surface: "panel", id: request.id };
    return this.mode === "permission" ? { surface: "island", id: request.id } : undefined;
  });

  constructor() {
    $effect.root(() => {
      // Every request that comes on screen (or moves to the panel) starts its look again.
      $effect(() => permissions.guard.show(this.answering, performance.now()));
      // A request arriving while the panel is open brings the Agents tab, where it shows
      // first; one already waiting when the panel opened leaves the chosen tab alone.
      let seen = permissions.current?.id;
      $effect(() => {
        const id = permissions.current?.id;
        if (id !== undefined && id !== seen && untrack(() => this.panelOpen)) this.tab = "agents";
        seen = id;
      });
      // An opening covered by a permission, the menu or the wardrobe keeps the time it has left.
      $effect(() => banners.hold("cover", this.mode === "permission" || this.mode === "menu" || this.mode === "wardrobe"));
      // A permission goes before everything: it opens the island alone, the hub folds.
      $effect(() => {
        if (this.mode === "permission") untrack(() => this.closeHub());
      });
      // Mochi never misses an alert: out of the island, he flies back for it (on the
      // desktop he stays where he was put; a click on the island still calls him back).
      $effect(() => {
        if (this.mochiOut && !this.mochiDesk && ALERTS.includes(this.mode)) void buddyRecall();
      });
    });
  }

  /** A click on the island (not on one of its buttons): the hub (its messages when the
   * pill counts unread ones), or the panel's tab tied to the opening on screen; brings
   * Mochi back if he is out; never answers a permission. */
  activate(): void {
    if (this.unread > 0 && this.mode === "pill") {
      notifications.seeAll();
      this.openHub({ view: "messages" });
    } else if (this.mochiOut) {
      void buddyRecall();
    } else if (this.mode === "music" || this.mode === "track") {
      this.openPanel("music");
    } else if (this.mode === "message") {
      this.openPanel("messages");
    } else if (this.mode === "finished" || this.mode === "failed") {
      this.openPanel("agents");
    } else if (this.mode === "wardrobe") {
      // A click beside the wardrobe's pieces changes nothing.
    } else if (this.mode === "toast" && banners.current?.kind === "toast" && banners.current.tab) {
      this.openPanel(banners.current.tab);
    } else if (this.mode === "voice" || this.mode === "translation") {
      // « OK Mochi » and the clipboard wait for their own buttons: a click beside them
      // changes nothing.
    } else if (this.mode !== "permission") {
      if (this.panelOpen) this.closePanel();
      else if (this.hubOpen) this.closeHub();
      else this.openHub();
    }
  }

  /** Lights a dot in the pill for a moment (`dotKey` of the session). */
  flash(key: string, tone: "ok" | "err"): void {
    clearTimeout(this.#flashTimers[key]);
    this.flashes[key] = tone;
    this.#flashTimers[key] = setTimeout(() => delete this.flashes[key], FLASH_MS);
  }

  /** Cursor entered or left the island. Hovering never opens anything. */
  setHovered(inside: boolean): void {
    this.hovered = inside;
    banners.hold("hover", inside);
  }

  /** Opens the large panel (« Réglages ») on purpose (the hub's gear, shortcut, tray,
   * right click) and takes the keyboard. Not while a permission waits: it is answered
   * first. */
  openPanel(tab?: Tab): void {
    if (this.mode === "permission") {
      bubble.say(texts().mochi.say.answerFirst);
      return;
    }
    if (tab) this.tab = tab;
    if (banners.current && banners.current.kind !== "message") banners.queue.dismissCurrent();
    this.menuOpen = false;
    this.wardrobeOpen = false;
    this.closeHub();
    this.panelOpen = true;
    void focusIsland();
  }

  /** Closes the panel (Island.svelte then hands the keyboard focus back). */
  closePanel(): void {
    this.panelOpen = false;
  }

  /** Brings the hub out of the pill, on its home (or `view`), with its intro if asked.
   * Not while a permission waits. The launch intro leaves the keyboard where it is. */
  openHub({ view = "home", intro = null }: { view?: HubView; intro?: HubIntro | null } = {}): void {
    if (this.mode === "permission") {
      bubble.say(texts().mochi.say.answerFirst);
      return;
    }
    if (banners.current && banners.current.kind !== "message") banners.queue.dismissCurrent();
    this.menuOpen = false;
    this.wardrobeOpen = false;
    this.panelOpen = false;
    this.hubIntro = intro;
    this.showHub(intro ? "home" : view);
    this.hubOpen = true;
    if (intro !== "launch") void focusIsland();
  }

  /** The counter of the activity line (« api.py +12 −3 ») or of a finished banner: the hub
   * opens on that session followed live, on its latest change. */
  openChange(sessionId: string): void {
    // The browser preview's staged demo gives way to the change asked for.
    this.hubStage = null;
    this.hubFollow = `agent:${sessionId}`;
    this.liveTab = "diff";
    this.openHub({ view: "live" });
  }

  /** Shows another view of the open hub (its size follows). Its messages show the
   * latest line of each thread: they count as read. */
  showHub(view: HubView): void {
    this.hubView = view;
    if (view === "messages") {
      for (const conversation of conversations.list) conversations.markRead(conversation.key);
    }
  }

  /** Folds the hub (Island.svelte then hands the keyboard focus back). */
  closeHub(): void {
    this.hubOpen = false;
    this.hubIntro = null;
    this.hubView = "home";
    this.liveTab = "journal";
  }

  /** Right click on the pill: Ne pas déranger, Focus, Recentrer, Revoir l'intro, Réglages. */
  openMenu(): void {
    if (this.mode !== "pill") return;
    this.closeHub();
    this.menuOpen = true;
    void focusIsland();
  }

  closeMenu(): void {
    this.menuOpen = false;
  }

  /** A right click on Mochi (pill, hub, or his window on the desktop): his wardrobe opens
   * in the island and takes the keyboard. Not while a permission waits. */
  openWardrobe(): void {
    if (this.mode === "permission") {
      bubble.say(texts().mochi.say.answerFirst);
      return;
    }
    if (banners.current && banners.current.kind !== "message") banners.queue.dismissCurrent();
    this.menuOpen = false;
    this.panelOpen = false;
    this.closeHub();
    this.wardrobeOpen = true;
    void focusIsland();
  }

  closeWardrobe(): void {
    this.wardrobeOpen = false;
  }

  /** Global shortcut. On an opened permission, the first press only brings the keyboard
   * to it (Enter allows, Escape refuses); otherwise it closes what is open, or opens the
   * hub (the panel's Agents tab while a permission waits there). */
  toggle(): void {
    if (this.mode === "permission" && !this.focused) {
      void focusIsland();
    } else if (this.panelOpen) {
      this.closePanel();
    } else if (this.hubOpen) {
      this.closeHub();
    } else if (permissions.current) {
      this.openPanel("agents");
    } else {
      this.openHub();
    }
  }

  /** The user clicked elsewhere: the panel, the hub and the menu close without taking the
   * focus back. */
  blur(): void {
    this.focused = false;
    this.panelOpen = false;
    this.menuOpen = false;
    this.wardrobeOpen = false;
    this.closeHub();
  }

  /** Sends what can be clicked to the core: the island (and the music bubble beside it),
   * and the hub or the panel when open; clicks pass through everywhere else. */
  syncShape(extra: Shape[] = []): void {
    const island = shapeFor(this.size, this.placement);
    const panel = this.panelOpen ? [{ ...panelBox(this.placement), radius: PANEL_MAX.radius }] : [];
    const hub = this.hubOpen
      ? [{ ...hubBox(islandBox(this.size, this.placement), this.placement.opensDown, hubHeight(this.hubView)), radius: HUB_RADIUS }]
      : [];
    void setIslandShape([island, ...extra, ...hub, ...panel]);
  }
}

export const island = new IslandController();
