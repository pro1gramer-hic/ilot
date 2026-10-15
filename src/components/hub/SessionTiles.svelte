<!-- The right card of the hub's home (09, section 8 bis): one small Mochi per session, in
     its colors, with a state dot (it pulses while the agent works), and a dashed « + »
     tile. Icons only: the name and the state show in a tooltip on hover or keyboard
     focus, and screen readers hear them. One icon is chosen at a time (radio group,
     arrows move); choosing it puts its session on the left card. -->
<script lang="ts">
  import { type HubSession, mochiFor, tileLabel, tileSubagents, tileTip } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import HubIcon from "./HubIcon.svelte";

  let {
    tiles,
    followed,
    onchoose,
  }: { tiles: HubSession[]; followed: string | undefined; onchoose: (key: string) => void } = $props();

  let buttons = $state<HTMLButtonElement[]>([]);

  function onkeydown(event: KeyboardEvent, index: number): void {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = (index + step + tiles.length) % tiles.length;
    onchoose(tiles[next].key);
    buttons[next]?.focus();
  }
</script>

<div class="box">
  <div class="tiles">
    <div class="group" role="radiogroup" aria-label={i18n.t.hub.tiles}>
      {#each tiles as session, index (session.key)}
        {@const stance = mochiFor(session.status)}
        {@const helpers = tileSubagents(session)}
        <button
          type="button"
          role="radio"
          class="tile"
          aria-checked={session.key === followed}
          aria-label={tileLabel(session)}
          data-tip={tileTip(session)}
          tabindex={session.key === followed || (followed === undefined && index === 0) ? 0 : -1}
          style:--sc={session.color}
          bind:this={buttons[index]}
          onclick={() => onchoose(session.key)}
          onkeydown={(event) => onkeydown(event, index)}
        >
          <Mochi palette={session.palette} move={stance.move} face={stance.face} scale={2} shadow={false} label="" />
          <i class="state {session.status}" aria-hidden="true"></i>
          {#if helpers > 0}<b class="subs" aria-hidden="true">{helpers}</b>{/if}
        </button>
      {/each}
    </div>
    <button
      type="button"
      class="tile add"
      aria-label={i18n.t.hub.newSessionLabel}
      data-tip={i18n.t.hub.newSession}
      onclick={() => island.openPanel("agents")}
    >
      <HubIcon name="plus" size={18} />
    </button>
  </div>
</div>

<style>
  .box {
    height: 100%;
    display: grid;
    place-items: center;
    padding: var(--s3);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, 56px);
    gap: var(--s3);
  }

  /* The radio group lays its icons in the same grid as the « + » tile. */
  .group {
    display: contents;
  }

  .tile {
    position: relative;
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    border-radius: var(--r-card);
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      transform 0.2s var(--ease-pop),
      box-shadow var(--t-quick);
  }

  .tile[aria-checked="true"] {
    box-shadow:
      inset 0 0 0 1.5px var(--sc),
      0 8px 20px color-mix(in srgb, var(--sc) 25%, transparent);
  }

  .tile.add {
    color: var(--ink-3);
    background: none;
    box-shadow: none;
    border: 1px dashed var(--line-2);
  }

  .tile.add:hover {
    color: var(--ink);
  }

  /* The state dot, cut out of the tile's corner; it rings while the agent works. */
  .state {
    position: absolute;
    right: -3px;
    top: -3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--ink-3);
    box-shadow: 0 0 0 2.5px var(--surface);
  }

  .state.work {
    background: var(--accent);
  }

  .state.work::after {
    content: "";
    position: absolute;
    inset: -3px;
    border-radius: 50%;
    border: 1.5px solid var(--accent);
    animation: ping 1.6s var(--ease-out) infinite;
  }

  .state.ok {
    background: var(--ok);
  }

  .state.err {
    background: var(--err);
  }

  .state.perm {
    background: var(--warn);
  }

  .state.play {
    background: var(--sc);
  }

  /* Its subagents at work, counted in the opposite corner. */
  .subs {
    position: absolute;
    right: -4px;
    bottom: -4px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--surface-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 0 0 2px var(--surface);
    color: var(--ink);
    font-size: var(--fs-1);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  /* Name and state on hover or keyboard focus (screen readers get the aria-label). */
  .tile::after {
    content: attr(data-tip);
    position: absolute;
    bottom: calc(100% + 8px);
    left: 50%;
    z-index: 2;
    padding: var(--s1) var(--s2);
    border-radius: var(--r-chip);
    background: var(--surface-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      var(--shadow-soft);
    color: var(--ink);
    font-size: var(--fs-2);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transform: translate(-50%, 2px);
    transition:
      opacity var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .tile:hover::after,
  .tile:focus-visible::after {
    opacity: 1;
    transform: translate(-50%, 0);
  }

  @media (hover: hover) and (pointer: fine) {
    .tile:hover {
      transform: translateY(-3px) scale(1.04);
    }
  }

  @keyframes ping {
    from {
      transform: scale(0.6);
      opacity: 1;
    }

    to {
      transform: scale(1.6);
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tile,
    .tile::after {
      transition: opacity var(--t-quick);
    }

    .tile:hover {
      transform: none;
    }

    .state.work::after {
      animation: none;
      opacity: 0;
    }
  }
</style>
