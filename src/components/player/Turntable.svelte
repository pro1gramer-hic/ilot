<!-- The record on its deck, with a tonearm: on play the arm swings onto the outer grooves,
     then creeps toward the label as the track goes on; on pause it goes back to its rest.
     With nothing to play, the platter is empty. The turntable takes the height of its
     slot. Decorative: the title, the time and the play button say the same in words. -->
<script lang="ts">
  import { rise, vanish } from "../../lib/motion.svelte";
  import Vinyl from "../Vinyl.svelte";

  let {
    loaded = false,
    artwork,
    playing = false,
    played = 0,
  }: { loaded?: boolean; artwork?: string | null; playing?: boolean; played?: number } = $props();

  // Geometry in record units: the record is 100 wide, the deck a little wider for the arm.
  const DECK = { width: 116, height: 100 };
  const PIVOT = { x: 105, y: 16 };
  /** The arm is drawn upright in its own box, pivot at (12, 14), stylus 58 lower. */
  const ARM = { width: 24, height: 80, pivotX: 12, pivotY: 14 };
  /** Angles (clockwise) that put the stylus on the outer grooves (radius 42) and, at the
   * end of the track, by the label (radius 26). At rest the arm stands off the record. */
  const ARM_REST = -4;
  const ARM_START = 18.7;
  const ARM_SWEEP = 15.9;

  const angle = $derived(playing ? ARM_START + ARM_SWEEP * Math.min(1, Math.max(0, played)) : ARM_REST);
  const pct = (value: number, of: number): string => `${(value / of) * 100}%`;
</script>

<div class="turntable" style:aspect-ratio="{DECK.width} / {DECK.height}" aria-hidden="true">
  <div class="platter">
    {#if loaded}
      <div class="record" in:rise={{ shift: 0 }} out:vanish>
        <Vinyl fill {artwork} {playing} />
      </div>
    {/if}
  </div>
  <svg class="deck" viewBox="0 0 {DECK.width} {DECK.height}">
    <circle class="spindle" cx="50" cy="50" r="1.4" />
    <circle class="base" cx={PIVOT.x} cy={PIVOT.y} r="7" />
  </svg>
  <svg
    class="arm"
    viewBox="0 0 {ARM.width} {ARM.height}"
    style:left={pct(PIVOT.x - ARM.pivotX, DECK.width)}
    style:top={pct(PIVOT.y - ARM.pivotY, DECK.height)}
    style:width={pct(ARM.width, DECK.width)}
    style:height={pct(ARM.height, DECK.height)}
    style:transform-origin="{pct(ARM.pivotX, ARM.width)} {pct(ARM.pivotY, ARM.height)}"
    style:transform="rotate({angle.toFixed(2)}deg)"
  >
    <rect class="weight" x="8" y="0.5" width="8" height="8" rx="2" />
    <line class="tube" x1="12" y1="8" x2="12" y2="62" />
    <circle class="cap" cx="12" cy="14" r="3" />
    <path class="head" d="M10 61h4l1.6 13h-7.2z" />
    <line class="lift" x1="15.2" y1="65" x2="19.5" y2="62.5" />
  </svg>
</div>

<style>
  .turntable {
    position: relative;
    height: 100%;
    flex: none;
  }

  /* The bare platter: a rubber mat with fine rings, seen when no record is on it. */
  .platter {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    aspect-ratio: 1;
    border-radius: 50%;
    background:
      repeating-radial-gradient(circle, var(--line) 0 0.6px, transparent 0.6px 6px),
      var(--well);
    box-shadow:
      0 0 0 1px var(--line-2),
      var(--shadow-soft);
  }

  .record {
    position: absolute;
    inset: 0;
  }

  svg {
    position: absolute;
    overflow: visible;
  }

  .deck {
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .spindle {
    fill: var(--arm);
  }

  .base {
    fill: var(--well-2);
    stroke: var(--line-2);
    stroke-width: 0.6;
  }

  /* Only the arm's box turns: one composited layer, no repaint. */
  .arm {
    filter: drop-shadow(var(--arm-shadow));
    transition: transform var(--t-arm) var(--ease-out);
  }

  .weight,
  .cap {
    fill: var(--arm);
  }

  .tube,
  .lift {
    stroke: var(--arm);
    stroke-linecap: round;
  }

  .tube {
    stroke-width: 1.6;
  }

  .lift {
    stroke-width: 1;
  }

  .head {
    fill: var(--arm-head);
    stroke: var(--arm);
    stroke-width: 0.6;
  }

  @media (prefers-reduced-motion: reduce) {
    .arm {
      transition: none;
    }
  }
</style>
