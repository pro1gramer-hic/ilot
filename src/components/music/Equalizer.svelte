<!-- Equalizer of the island: a few bars driven by the output's peak level only (meter.ts),
     never by the sound itself. One level, smoothed like a needle (quick up, slow down),
     ripples from the middle bar to the others. The bars wear the color of the source; there
     is no text in them. They run only while the music plays and the equalizer can be seen,
     and lie flat at once when it pauses or hides. With reduced motion they stand still. -->
<script lang="ts">
  import { type MediaSource, barHeights, meterLevel, smoothLevel } from "../../lib/media";
  import { reduced, stillness } from "../../lib/motion.svelte";
  import { onScreen, watchInert } from "../player/presence.svelte";
  import { meter } from "./meter";

  let {
    playing,
    source,
    bars = 5,
    height = 16,
    thin = false,
  }: { playing: boolean; source: MediaSource; bars?: number; height?: number; thin?: boolean } = $props();

  /** Frames of smoothed levels kept for the ripple (the longest bar delay, and a margin). */
  const HISTORY = 12;
  /** Heights of standing bars (reduced motion), so they still read as an equalizer. */
  const STILL = [0.45, 0.8, 0.6, 0.95, 0.5, 0.7, 0.35];

  let root = $state<HTMLElement>();

  $effect(() => watchInert());

  const visible = $derived(root ? onScreen(root) : false);
  const live = $derived(playing && visible && !stillness.current && !reduced());

  function lay(heights: readonly number[]): void {
    if (!root) return;
    [...root.children].forEach((bar, i) => (bar as HTMLElement).style.setProperty("--h", String(heights[i] ?? 0)));
  }

  $effect(() => {
    if (!live) {
      lay(playing && reduced() ? STILL : []);
      return;
    }
    const stop = meter.listen();
    const count = bars;
    let level = 0;
    const history: number[] = [];
    let frame = requestAnimationFrame(function step() {
      level = smoothLevel(level, meterLevel(meter.peak));
      history.unshift(level);
      history.length = Math.min(history.length, HISTORY);
      lay(barHeights(history, count));
      frame = requestAnimationFrame(step);
    });
    return () => {
      cancelAnimationFrame(frame);
      stop();
      lay([]);
    };
  });
</script>

<span class="eq" class:thin data-source={source} style:height="{height}px" aria-hidden="true" bind:this={root}>
  {#each { length: bars }, i (i)}<i></i>{/each}
</span>

<style>
  .eq {
    --w: 3px;
    display: inline-flex;
    align-items: center;
    gap: var(--gap-mark);
    flex: none;
    color: var(--ink-2);
    contain: layout;
  }

  .thin {
    --w: var(--stroke);
  }

  .eq[data-source="Spotify"] {
    color: var(--src-spotify);
  }

  .eq[data-source="YouTube"] {
    color: var(--src-youtube);
  }

  .eq[data-source="Crunchyroll"] {
    color: var(--src-crunchyroll);
  }

  /* A flat bar is a dot as wide as the bar; it grows from there to the full height. */
  i {
    --h: 0;
    width: var(--w);
    height: max(var(--w), calc(var(--h) * 100%));
    border-radius: var(--r-pill);
    background: currentColor;
  }
</style>
