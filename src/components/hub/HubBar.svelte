<!-- The hub's top bar (09, section 8 bis): Accueil and Messages (a dot while a thread has
     something unread), Pastilles while at least one is on (prompt 12, lot 4), the chat
     (prompt 12, lot 3), a new session, how many agents work,
     Réglages (the large panel), the sound on or off in one click (prompt 12, lot 1) and Ne
     pas déranger. Left and right arrows move between the views. -->
<script lang="ts">
  import { workingLabel } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { chat } from "../../lib/stores/chat.svelte";
  import { conversations } from "../../lib/stores/conversations.svelte";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import HubIcon from "./HubIcon.svelte";

  const BASE_VIEWS = [
    { id: "home", icon: "home" },
    { id: "messages", icon: "chat" },
    { id: "chat", icon: "spark" },
  ] as const;
  const PASTILLES_VIEW = { id: "pastilles", icon: "pastilles" } as const;
  const t = $derived(i18n.t.hub.bar);

  const VIEWS = $derived(pastilles.ids.length > 0 ? [...BASE_VIEWS, PASTILLES_VIEW] : [...BASE_VIEWS]);

  let tabs = $state<HTMLButtonElement[]>([]);
  const current = $derived(Math.max(0, VIEWS.findIndex((view) => view.id === island.hubView)));
  const quiet = $derived(settings.value.doNotDisturb);
  const muted = $derived(settings.value.sound.muted);

  /** The tab the hub's keyboard starts on. */
  export function focus(): void {
    tabs[current]?.focus({ preventScroll: true });
  }

  function onkeydown(event: KeyboardEvent): void {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = (current + step + VIEWS.length) % VIEWS.length;
    if (VIEWS[next].id === "chat") chat.open();
    else island.showHub(VIEWS[next].id);
    tabs[next]?.focus();
  }
</script>

<div class="bar">
  <div class="views" role="tablist" aria-label={t.label} tabindex="-1" {onkeydown}>
    {#each VIEWS as view, index (view.id)}
      <button
        type="button"
        role="tab"
        class="icon"
        id="hub-tab-{view.id}"
        title={t.views[view.id]}
        aria-selected={index === current}
        aria-controls="hub-view"
        tabindex={index === current ? 0 : -1}
        bind:this={tabs[index]}
        onclick={() => (view.id === "chat" ? chat.open() : island.showHub(view.id))}
      >
        <HubIcon name={view.icon} />
        <span class="visually-hidden">{t.views[view.id]}</span>
        {#if view.id === "messages" && conversations.unread > 0}
          <i class="unread" aria-hidden="true"></i>
          <span class="visually-hidden">, {t.unread(conversations.unread)}</span>
        {/if}
      </button>
    {/each}
  </div>
  <button
    type="button"
    class="icon"
    aria-label={i18n.t.hub.newSessionLabel}
    title={i18n.t.hub.newSession}
    onclick={() => island.openPanel("agents")}
  >
    <HubIcon name="plus" />
  </button>
  <span class="count">{workingLabel(sessions.liveCount, decisions.suspended)}</span>
  <button type="button" class="icon" aria-label={t.settings} title={t.settings} onclick={() => island.openPanel()}>
    <HubIcon name="gear" />
  </button>
  <button
    type="button"
    class="icon"
    class:on={muted}
    aria-label={t.mute}
    title={muted ? t.muted : t.mute}
    aria-pressed={muted}
    onclick={() => void settings.update({ sound: { ...settings.value.sound, muted: !muted } })}
  >
    <HubIcon name={muted ? "mute" : "sound"} />
  </button>
  <button
    type="button"
    class="icon"
    class:on={quiet}
    aria-label={t.quiet}
    title={quiet ? t.quietOn : t.quiet}
    aria-pressed={quiet}
    onclick={() => void settings.update({ doNotDisturb: !quiet })}
  >
    <HubIcon name="moon" />
  </button>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--s1);
    height: 34px;
    padding: 0 var(--gap-mark);
  }

  .views {
    display: flex;
    gap: var(--s1);
    outline: none;
  }

  .icon {
    position: relative;
    width: 32px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    transition:
      background-color var(--t-quick),
      color var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .icon:hover {
    background: var(--surface);
    color: var(--ink);
  }

  .icon:active {
    transform: scale(var(--press));
  }

  .icon[aria-selected="true"],
  .icon.on {
    background: var(--surface-2);
    color: var(--ink);
  }

  .icon.on {
    color: var(--accent-text);
  }

  .unread {
    position: absolute;
    right: 5px;
    top: 5px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--island);
  }

  .count {
    flex: 1;
    margin: 0 var(--s2);
    text-align: right;
    color: var(--ink-3);
    font-size: var(--fs-2);
    white-space: nowrap;
  }

  @media (prefers-reduced-motion: reduce) {
    .icon:active {
      transform: none;
    }
  }
</style>
