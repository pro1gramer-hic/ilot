<!-- Progress drawn as a wave, with the time played and the length under it. Windows gives
     no sound, so the wave is the track's own soft silhouette (waveform in media.ts): it
     breathes slowly while the track plays and on screen, and stands still on pause. The
     played part is in ink, the rest set back. `dense` draws wide bars side by side (the
     Musique tab). With `onseek`, the wave is the seek bar: a click or a drag moves the
     track there, the arrows by 5 s, Page Up and Page Down by 30 s (never a live stream). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type Track, WAVE_FLOOR, formatTime, positionAt, progress, seekAt, timeEnds, timeLabel, waveform } from "../../lib/media";
  import { stillness } from "../../lib/motion.svelte";
  import { onScreen, watchInert } from "./presence.svelte";

  let {
    track,
    now,
    bars,
    height,
    dense = false,
    onseek,
  }: { track: Track | null; now: number; bars: number; height: number; dense?: boolean; onseek?: (seconds: number) => void } =
    $props();

  /** Keyboard steps, in seconds. */
  const STEP = 5;
  const PAGE = 30;

  const heights = $derived(track ? waveform(track.id, bars) : Array<number>(bars).fill(WAVE_FLOOR));
  const seekable = $derived(onseek !== undefined && track !== null && track.duration > 0);
  /** Share under the pointer while the wave is being dragged. */
  let scrub = $state<number | null>(null);
  const played = $derived(scrub ?? (track ? progress(track, now) : 0));
  const ends = $derived(track ? timeEnds(track, now) : null);

  let root = $state<HTMLElement>();

  $effect(() => watchInert());

  const breathing = $derived(!!track?.playing && root !== undefined && onScreen(root) && !stillness.current);

  function share(event: PointerEvent): number {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    return Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
  }

  function down(event: PointerEvent): void {
    if (!seekable || event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    scrub = share(event);
  }

  function move(event: PointerEvent): void {
    if (scrub !== null) scrub = share(event);
  }

  function up(event: PointerEvent): void {
    if (scrub === null || !track) return;
    const seconds = seekAt(track, share(event));
    scrub = null;
    if (seconds !== null) onseek?.(seconds);
  }

  function key(event: KeyboardEvent): void {
    if (!seekable || !track) return;
    const at = positionAt(track, now);
    const target: Record<string, number> = {
      ArrowLeft: at - STEP,
      ArrowDown: at - STEP,
      ArrowRight: at + STEP,
      ArrowUp: at + STEP,
      PageDown: at - PAGE,
      PageUp: at + PAGE,
      Home: 0,
      End: track.duration,
    };
    if (!(event.key in target)) return;
    event.preventDefault();
    onseek?.(Math.min(track.duration, Math.max(0, Math.round(target[event.key]))));
  }
</script>

{#snippet line(kind: "rest" | "ink", clip?: string)}
  <span class="line {kind}" style:clip-path={clip} aria-hidden="true">
    {#each heights as h, i (i)}<i style:--h={h} style:--i={i}></i>{/each}
  </span>
{/snippet}

<div class="wave" class:empty={!track} class:breathing class:dense bind:this={root}>
  {#if seekable && track}
    <div
      class="bar seek"
      class:scrubbing={scrub !== null}
      style:height="{height}px"
      role="slider"
      tabindex="0"
      aria-label={i18n.t.music.position}
      aria-valuemin={0}
      aria-valuemax={Math.round(track.duration)}
      aria-valuenow={Math.round(scrub === null ? positionAt(track, now) : scrub * track.duration)}
      aria-valuetext={scrub === null ? timeLabel(track, now) : formatTime(scrub * track.duration)}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={() => (scrub = null)}
      onkeydown={key}
    >
      {@render line("rest")}
      {@render line("ink", `inset(0 ${((1 - played) * 100).toFixed(2)}% 0 0)`)}
    </div>
  {:else}
    <div
      class="bar"
      style:height="{height}px"
      role="progressbar"
      aria-label={i18n.t.music.progress}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(played * 100)}
      aria-valuetext={track ? timeLabel(track, now) : i18n.t.music.nothingPlaying}
    >
      {@render line("rest")}
      <!-- Clipped, not resized: moving once a second repaints a few pixels, no layout. -->
      {@render line("ink", `inset(0 ${((1 - played) * 100).toFixed(2)}% 0 0)`)}
    </div>
  {/if}
  <div class="times" aria-hidden="true">
    <span class="elapsed">{scrub !== null && track ? formatTime(scrub * track.duration) : (ends?.elapsed ?? "")}</span>
    <span class="total">{ends?.total ?? ""}</span>
  </div>
</div>

<style>
  .wave {
    min-width: 0;
  }

  .bar {
    position: relative;
  }

  .seek {
    cursor: pointer;
    touch-action: none;
    border-radius: var(--r-chip);
  }

  .seek:focus-visible {
    outline-offset: 4px;
  }

  .line {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .dense .line {
    gap: var(--gap-mark);
  }

  .rest {
    color: var(--wave-rest);
    transition: color var(--t-quick);
  }

  .seek:hover .rest,
  .scrubbing .rest {
    color: color-mix(in srgb, var(--wave-rest), var(--ink) 18%);
  }

  .ink {
    color: var(--ink);
  }

  /* Each bar swells and settles, a little after its left neighbor: the breath travels
     along the wave. Paused, every bar keeps the height it had. */
  i {
    width: 2px;
    height: calc(var(--h) * 100%);
    border-radius: var(--r-pill);
    background: currentColor;
    animation: breathe var(--t-breath) ease-in-out infinite alternate both paused;
    animation-delay: calc(var(--i) * var(--wave-lag) * -1);
  }

  .dense i {
    flex: 1;
    width: auto;
    border-radius: var(--r-mark);
  }

  .breathing i {
    animation-play-state: running;
  }

  .empty i {
    animation: none;
  }

  @keyframes breathe {
    to {
      transform: scaleY(0.58);
    }
  }

  .times {
    display: flex;
    justify-content: space-between;
    margin-top: var(--s1);
    min-height: calc(var(--fs-1) * 1.35);
    color: var(--ink-3);
    font-size: var(--fs-1);
    line-height: 1.35;
  }

  .dense .times {
    margin-top: var(--s2);
    font-family: var(--font-mono);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    i {
      animation: none;
    }
  }
</style>
