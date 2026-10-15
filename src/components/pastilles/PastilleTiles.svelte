<!-- The right card of the hub's « Pastilles » view: one small Mochi per pastille, in its
     colors, with a state dot (a ring while something runs, red when it failed, hollow when
     it is off), and a dashed « + » tile to Réglages › Pastilles. Icons only, as the
     sessions of the home: the name and the line show on hover or keyboard focus, and
     screen readers hear them. One is chosen at a time (radio group, arrows move); the
     chosen one fills the left card and becomes the pill's. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { PALETTES } from "../../lib/mochi/palettes";
  import { lookOf, stanceOf } from "../../lib/pastilles";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import HubIcon from "../hub/HubIcon.svelte";

  let buttons = $state<HTMLButtonElement[]>([]);
  const t = $derived(i18n.t.pastilles);

  const tiles = $derived(
    pastilles.ids.map((id) => {
      const view = pastilles.view(id);
      return { id, look: lookOf(id, pastilles.config.tool, pastilles.catalog), tone: view?.tone ?? "idle", line: view?.line ?? "" };
    }),
  );

  function onkeydown(event: KeyboardEvent, index: number): void {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = (index + step + tiles.length) % tiles.length;
    pastilles.select(tiles[next].id);
    buttons[next]?.focus();
  }
</script>

<div class="box">
  <div class="tiles">
    <div class="group" role="radiogroup" aria-label={t.tiles.label}>
      {#each tiles as tile, index (tile.id)}
        {@const stance = stanceOf(tile.tone)}
        <button
          type="button"
          role="radio"
          class="tile"
          aria-checked={tile.id === pastilles.shown}
          aria-label={t.tiles.tile(tile.look.name, tile.line)}
          data-tip={tile.look.name}
          tabindex={tile.id === pastilles.shown ? 0 : -1}
          style:--sc={PALETTES[tile.look.palette].tiers[2]}
          bind:this={buttons[index]}
          onclick={() => pastilles.select(tile.id)}
          onkeydown={(event) => onkeydown(event, index)}
        >
          <Mochi palette={tile.look.palette} kind={tile.look.kind} move={stance.move} face={stance.face} scale={2} shadow={false} label="" />
          <i class="state {tile.tone}" aria-hidden="true"></i>
        </button>
      {/each}
    </div>
    <button
      type="button"
      class="tile add"
      aria-label={t.add}
      data-tip={t.add}
      onclick={() => island.openPanel("pastilles")}
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

  /* The state dot, cut out of the tile's corner. */
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

  .state.ok {
    background: var(--ok);
  }

  .state.err {
    background: var(--err);
  }

  .state.warn {
    background: var(--warn);
  }

  .state.busy {
    background: var(--accent);
  }

  .state.busy::after {
    content: "";
    position: absolute;
    inset: -3px;
    border-radius: 50%;
    border: 1.5px solid var(--accent);
    animation: ping 1.6s var(--ease-out) infinite;
  }

  /* Off: a ring, no fill. */
  .state.off {
    background: var(--surface);
    box-shadow:
      inset 0 0 0 2px var(--ink-3),
      0 0 0 2.5px var(--surface);
  }

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

    .state.busy::after {
      animation: none;
      opacity: 0;
    }
  }
</style>
