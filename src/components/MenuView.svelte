<!-- Quick menu, opened by a right click on the pill: Ne pas déranger, a focus session,
     Recentrer l'île, the hub's intro again, Réglages (the large panel). Arrow keys move,
     Escape or a click elsewhere closes it. -->
<script lang="ts">
  import { remainingLabel } from "../lib/assistant";
  import { i18n } from "../lib/i18n/i18n.svelte";
  import { inTauri, islandRecenter } from "../lib/ipc";
  import { island } from "../lib/island/controller.svelte";
  import { DEFAULT_PLACEMENT } from "../lib/island/model";
  import { rise } from "../lib/motion.svelte";
  import { focus } from "../lib/stores/focus.svelte";
  import { settings } from "../lib/stores/settings.svelte";
  import Icon from "./Icon.svelte";

  const FOCUS_MINUTES = 25;
  const t = $derived(i18n.t.island.menu);
  let items = $state<HTMLButtonElement[]>([]);

  $effect(() => {
    items[0]?.focus();
  });

  function onkeydown(event: KeyboardEvent): void {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    items[(index + step + items.length) % items.length]?.focus();
  }

  async function toggleQuiet(): Promise<void> {
    await settings.update({ doNotDisturb: !settings.value.doNotDisturb });
    island.closeMenu();
  }

  /** Back to the top center of the screen (the browser preview moves its anchor). */
  function recenter(): void {
    if (inTauri) void islandRecenter();
    else island.placement = { ...DEFAULT_PLACEMENT };
    island.closeMenu();
  }

  function toggleFocus(): void {
    if (focus.active) focus.stop();
    else focus.start(FOCUS_MINUTES);
    island.closeMenu();
  }
</script>

<div class="menu" role="menu" aria-label={t.label} tabindex="-1" {onkeydown}>
  <button type="button" role="menuitemcheckbox" aria-checked={settings.value.doNotDisturb} bind:this={items[0]} onclick={toggleQuiet} in:rise={{ index: 0, delay: 80 }}>
    <Icon name="moon" size={13} />
    <span>{t.quiet}</span>
    <i class="switch" class:on={settings.value.doNotDisturb} aria-hidden="true"></i>
  </button>
  <button type="button" role="menuitem" bind:this={items[1]} onclick={toggleFocus} in:rise={{ index: 1, delay: 80 }}>
    <Icon name="timer" size={13} />
    <span>{focus.active ? t.stopFocus(remainingLabel(focus.remainingMs)) : t.focus(FOCUS_MINUTES)}</span>
  </button>
  <button type="button" role="menuitem" bind:this={items[2]} onclick={recenter} in:rise={{ index: 2, delay: 80 }}>
    <Icon name="target" size={13} />
    <span>{t.recenter}</span>
  </button>
  <button type="button" role="menuitem" bind:this={items[3]} onclick={() => island.openHub({ intro: "replay" })} in:rise={{ index: 3, delay: 80 }}>
    <Icon name="play" size={13} />
    <span>{t.replayIntro}</span>
  </button>
  <button type="button" role="menuitem" bind:this={items[4]} onclick={() => island.openPanel("modes")} in:rise={{ index: 4, delay: 80 }}>
    <Icon name="sliders" size={13} />
    <span>{t.settings}</span>
  </button>
</div>

<style>
  /* Mochi floats at the top left: the items start right of it. */
  .menu {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 6px 6px 48px;
    outline: none;
  }

  button {
    position: relative;
    height: 30px;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: 0 var(--s2);
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    font-size: var(--fs-2);
    text-align: left;
    transition: transform var(--t-quick) var(--ease-out);
  }

  button::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--hover);
    opacity: 0;
    transition: opacity var(--t-quick);
  }

  button:hover::after,
  button:focus-visible::after {
    opacity: 1;
  }

  button:hover,
  button:focus-visible {
    color: var(--ink);
  }

  button:active {
    transform: scale(0.97);
  }

  span {
    flex: 1;
  }

  /* A small switch: its knob slides, its track lights up. */
  .switch {
    position: relative;
    width: 22px;
    height: 12px;
    flex: none;
    border-radius: var(--r-pill);
    background: var(--card-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .switch::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--accent);
    opacity: 0;
    transition: opacity var(--t-quick);
  }

  .switch::after {
    content: "";
    position: absolute;
    left: 2px;
    top: 2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ink);
    transition: transform var(--t-quick) var(--ease-out);
  }

  .switch.on::before {
    opacity: 1;
  }

  .switch.on::after {
    transform: translateX(10px);
    background: var(--on-accent);
  }

  @media (prefers-reduced-motion: reduce) {
    button:active {
      transform: none;
    }
  }
</style>
