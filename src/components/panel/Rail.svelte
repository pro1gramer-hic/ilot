<!-- The panel's rail: Mochi with his name and mood (a dot and words), the tabs one
     above the other (arrow keys move between them; the chosen one is marked by a block
     that slides on a spring), and a small player that always stays at the bottom. -->
<script lang="ts">
  import { panelMood } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { TABS, type Tab } from "../../lib/island/model";
  import { subtitle } from "../../lib/media";
  import { FACE_IDS, type FaceId } from "../../lib/mochi/faces";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import type { MoveId } from "../../lib/mochi/moves";
  import { Spring } from "../../lib/spring.svelte";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { media } from "../../lib/stores/media.svelte";
  import { mood } from "../../lib/stores/mood.svelte";
  import { notifications } from "../../lib/stores/notifications.svelte";
  import { permissions } from "../../lib/stores/permissions.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Icon from "../Icon.svelte";
  import Vinyl from "../Vinyl.svelte";
  import { inbox, unreadCount } from "./inbox";
  import PanelIcon from "./PanelIcon.svelte";

  const buttons = $state<Partial<Record<Tab, HTMLButtonElement>>>({});
  const t = $derived(i18n.t.panel);

  const feeling = $derived(panelMood(sessions.list, permissions.current !== undefined, decisions.suspended));
  const unread = $derived(unreadCount(inbox(notifications.queue, notifications.postponed)));

  // Mochi trembles while something waits for the user, sleeps while everything is
  // suspended, dances to the music, breathes otherwise; a one-off move wins.
  const move: MoveId = $derived(
    mood.expressive ??
      (feeling.tone === "wait" ? "shake" : decisions.suspended ? "sleep" : media.playing ? "rock" : "breathe"),
  );
  const restFace = $derived(FACE_IDS.find((face) => face === settings.value.mochi.face));
  const face: FaceId | undefined = $derived(mood.face ?? (feeling.tone === "err" ? "worry" : restFace));

  // The block under the chosen tab: placed at once the first time, then on a spring.
  const top = new Spring(0);
  const height = new Spring(0);
  let placed = false;
  $effect(() => {
    const button = buttons[island.tab];
    if (!button) return;
    if (placed) {
      top.set(button.offsetTop);
      height.set(button.offsetHeight);
    } else {
      top.snap(button.offsetTop);
      height.snap(button.offsetHeight);
      placed = true;
    }
  });

  /** Arrow keys, Home and End move between tabs; the content follows (WAI-ARIA tabs). */
  function onKey(event: KeyboardEvent, index: number): void {
    const last = TABS.length - 1;
    const keys: Record<string, number> = {
      ArrowDown: (index + 1) % TABS.length,
      ArrowRight: (index + 1) % TABS.length,
      ArrowUp: (index + last) % TABS.length,
      ArrowLeft: (index + last) % TABS.length,
      Home: 0,
      End: last,
    };
    const next = keys[event.key];
    if (next === undefined) return;
    event.preventDefault();
    island.tab = TABS[next].id;
    buttons[TABS[next].id]?.focus();
  }

  const track = $derived(media.shown);
</script>

<aside class="rail" aria-label={t.rail.label}>
  <div class="me">
    <span class="avatar">
      <Mochi
        palette={settings.value.mochi.palette}
        {move}
        {face}
        scale={2}
        animated={settings.value.mochi.animated}
        label="{settings.value.mochi.name}, {feeling.label.toLowerCase()}"
        dressed
      />
    </span>
    <span class="who">
      <b class="name">{settings.value.mochi.name}</b>
      <span class="mood {feeling.tone}" aria-live="polite"><i aria-hidden="true"></i>{feeling.label}</span>
    </span>
  </div>

  <div class="nav" role="tablist" aria-label={t.rail.sections} aria-orientation="vertical">
    <i class="indicator" style:transform="translateY({top.value}px)" style:height="{height.value}px" aria-hidden="true"></i>
    {#each TABS as tab, index (tab.id)}
      <button
        type="button"
        role="tab"
        id="tab-{tab.id}"
        aria-selected={island.tab === tab.id}
        aria-controls={island.tab === tab.id ? `pane-${tab.id}` : undefined}
        tabindex={island.tab === tab.id ? 0 : -1}
        bind:this={buttons[tab.id]}
        onclick={() => (island.tab = tab.id)}
        onkeydown={(event) => onKey(event, index)}
      >
        <PanelIcon name={tab.id} />
        <span class="text">{t.tabs[tab.id]}</span>
        {#if tab.id === "agents" && sessions.liveCount > 0}
          <span class="count" aria-hidden="true">{sessions.liveCount}</span>
          <span class="visually-hidden">{t.rail.live(sessions.liveCount)}</span>
        {:else if tab.id === "messages" && unread > 0}
          <span class="badge" aria-hidden="true">{unread}</span>
          <span class="visually-hidden">{t.rail.unread(unread)}</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="player">
    <button
      type="button"
      class="now"
      onclick={() => (island.tab = "music")}
      aria-label={track ? t.rail.player(track.title, subtitle(track)) : t.rail.playerEmpty}
    >
      <Vinyl size={36} artwork={track?.artwork} playing={track?.playing ?? false} />
      <span class="lines">
        <b>{track?.title ?? t.rail.nothing}</b>
        <span>{track ? subtitle(track) : t.rail.start}</span>
      </span>
    </button>
    {#if track}
      <button
        type="button"
        class="toggle"
        disabled={!track.controls.playPause}
        onclick={() => void media.command("toggle", track)}
        aria-label={track.playing ? t.rail.pause : t.rail.play}
      >
        <Icon name={track.playing ? "pause" : "play"} size={12} />
      </button>
    {/if}
  </div>
</aside>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: var(--s4) var(--s3) var(--s3);
    border-right: 1px solid var(--line);
    background: linear-gradient(to bottom, var(--hover), transparent 40%);
  }

  .me {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 0 var(--s1) var(--s4);
  }

  /* Mochi drawn at 48 px, in a 44 px tile that crops the empty pixels around him. */
  .avatar {
    position: relative;
    flex: none;
    width: calc(var(--s8) + var(--s3));
    height: calc(var(--s8) + var(--s3));
    border-radius: calc(var(--r-ctl) + var(--s1));
    background: var(--surface);
    border: 1px solid var(--line);
    overflow: hidden;
  }

  .avatar :global(.mochi) {
    position: absolute;
    left: calc(-1 * var(--s1) / 2);
    top: calc(-1 * var(--s1));
  }

  .who {
    min-width: 0;
  }

  .name {
    display: block;
    font: 600 var(--fs-4) / 1.2 var(--font-pixel);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mood {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-top: calc(var(--s1) / 2);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .mood i {
    --tone: var(--ok);
    flex: none;
    width: var(--dot);
    height: var(--dot);
    border-radius: var(--r-pill);
    background: var(--tone);
    box-shadow: 0 0 0 var(--gap-mark) color-mix(in srgb, var(--tone) 22%, transparent);
  }

  .mood.wait i {
    --tone: var(--warn);
  }

  .mood.err i {
    --tone: var(--err);
  }

  .mood.idle i {
    --tone: var(--ink-3);
  }

  .nav {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: calc(var(--s1) / 2);
    /* A short panel scrolls its tabs; the player stays at the bottom. */
    min-height: 0;
    margin: calc(-1 * var(--s1)) calc(-1 * var(--s3));
    padding: var(--s1) var(--s3);
    overflow-y: auto;
    scrollbar-width: none;
  }

  /* The chosen tab's block, with the accent line on the rail's edge. */
  .indicator {
    position: absolute;
    left: var(--s3);
    right: var(--s3);
    top: 0;
    border-radius: var(--r-ctl);
    background: var(--surface-2);
    box-shadow: var(--hi);
    pointer-events: none;
    will-change: transform;
  }

  .indicator::before {
    content: "";
    position: absolute;
    left: calc(-1 * var(--s3));
    top: var(--s2);
    bottom: var(--s2);
    width: var(--stroke);
    border-radius: 0 var(--stroke) var(--stroke) 0;
    background: var(--accent);
  }

  [role="tab"] {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s3);
    height: calc(var(--s8) + var(--s1));
    padding: 0 var(--s3);
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    font-size: var(--fs-body);
    font-weight: 500;
    text-align: left;
    transition:
      background-color var(--t-quick) var(--ease-out),
      color var(--t-quick) var(--ease-out);
  }

  [role="tab"]:hover {
    background: var(--hover);
    color: var(--ink);
  }

  [role="tab"][aria-selected="true"] {
    background: none;
    color: var(--ink);
  }

  .text {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .count {
    color: var(--ink-3);
    font-size: var(--fs-1);
  }

  .badge {
    min-width: calc(var(--s4) + var(--s1) / 2);
    padding: 0 calc(var(--s1) + var(--gap-mark));
    border-radius: var(--r-pill);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--fs-1);
    font-weight: 600;
    line-height: calc(var(--s4) + var(--s1) / 2);
    text-align: center;
  }

  .player {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: var(--s2);
    border-radius: calc(var(--r-ctl) + var(--s1));
    background: var(--surface);
    border: 1px solid var(--line);
  }

  .now {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: var(--gap-mark);
    border-radius: var(--r-ctl);
    text-align: left;
  }

  .now:hover b {
    text-decoration: underline;
    text-decoration-color: var(--line-2);
    text-underline-offset: var(--gap-mark);
  }

  .lines {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .lines b,
  .lines span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lines b {
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .lines span {
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .toggle {
    flex: none;
    display: grid;
    place-items: center;
    width: calc(var(--s8) + var(--s1));
    height: calc(var(--s8) + var(--s1));
    border-radius: var(--r-pill);
    background: var(--ink);
    color: var(--panel);
    transition: transform var(--t-quick) var(--ease-out);
  }

  .toggle:hover:not(:disabled) {
    background: color-mix(in srgb, var(--ink) 86%, var(--panel));
  }

  .toggle:active:not(:disabled) {
    transform: scale(var(--press));
  }

  .toggle:disabled {
    opacity: var(--dim);
    cursor: default;
  }

  @media (prefers-reduced-motion: reduce) {
    .toggle:active:not(:disabled) {
      transform: none;
    }
  }
</style>
