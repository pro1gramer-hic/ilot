<!-- The thin progress line of the Musique opening, which can be dragged (prompt 12, lot 4):
     the played part in ink over a faint line, as before. Under the pointer or the keyboard
     focus it thickens and shows its knob; a press puts the knob under the pointer at once,
     then it follows the pointer 1:1 (pointer capture) and the track moves there on
     release. Arrows move by 5 s, Page Up and Page Down by 30 s, Home and End to the ends.
     While it is held, the time under the knob shows above it. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type Track, formatTime, positionAt, progress, seekAt, timeLabel } from "../../lib/media";

  let { track, now, onseek }: { track: Track; now: number; onseek: (seconds: number) => void } = $props();

  const STEP = 5;
  const PAGE = 30;

  /** Share under the pointer while the line is held. */
  let scrub = $state<number | null>(null);
  const played = $derived(scrub ?? progress(track, now));

  function share(event: PointerEvent): number {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    return Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
  }

  function down(event: PointerEvent): void {
    if (event.button !== 0) return;
    // The island under it neither sinks nor opens the panel: the press is the line's.
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    scrub = share(event);
  }

  function move(event: PointerEvent): void {
    if (scrub !== null) scrub = share(event);
  }

  function up(event: PointerEvent): void {
    if (scrub === null) return;
    const seconds = seekAt(track, share(event));
    scrub = null;
    if (seconds !== null) onseek(seconds);
  }

  function key(event: KeyboardEvent): void {
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
    onseek(Math.min(track.duration, Math.max(0, Math.round(target[event.key]))));
  }
</script>

<div
  class="seek"
  class:held={scrub !== null}
  class:running={track.playing && scrub === null}
  style:--p={played.toFixed(4)}
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
  onclick={(event) => event.stopPropagation()}
  onkeydown={key}
>
  <span class="line" aria-hidden="true"><i class="ink"></i></span>
  <i class="knob" aria-hidden="true"></i>
  {#if scrub !== null}
    <span class="time" aria-hidden="true">{formatTime(scrub * track.duration)}</span>
  {/if}
</div>

<style>
  /* A 14 px tall target around a 3 px line: easy to catch, same place as the old line. */
  .seek {
    position: relative;
    height: 14px;
    margin: var(--gap-mark) 0 calc(-1 * var(--s1) - 1px);
    cursor: pointer;
    touch-action: none;
    border-radius: var(--r-chip);
    container-type: inline-size;
  }

  .seek:focus-visible {
    outline-offset: 1px;
  }

  .line {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 3px;
    border-radius: var(--r-pill);
    background: var(--line-2);
    overflow: hidden;
    transform: translateY(-50%) scaleY(1);
    transition: transform var(--t-quick) var(--ease-out);
  }

  .ink {
    display: block;
    height: 100%;
    background: var(--ink);
    transform-origin: left;
    transform: scaleX(var(--p));
  }

  /* It moves once a second, smoothly while it plays, at once while held. */
  .running .ink,
  .running .knob {
    transition: transform 1s linear;
  }

  .knob {
    position: absolute;
    left: 0;
    top: 50%;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    background: var(--ink);
    box-shadow: var(--shadow-soft);
    opacity: 0;
    /* It rides on the line: 100cqw is the line's width (the seek is a size container). */
    transform: translateX(calc(var(--p) * 100cqw)) scale(0.4);
    transition:
      opacity var(--t-quick) var(--ease-out),
      transform var(--t-quick) var(--ease-out);
  }

  .seek:focus-visible .line,
  .held .line {
    transform: translateY(-50%) scaleY(1.67);
  }

  .seek:focus-visible .knob,
  .held .knob {
    opacity: 1;
    transform: translateX(calc(var(--p) * 100cqw)) scale(1);
  }

  /* A mouse over it shows the knob before the press; a touch does not fake a hover. */
  @media (hover: hover) and (pointer: fine) {
    .seek:hover .line {
      transform: translateY(-50%) scaleY(1.67);
    }

    .seek:hover .knob {
      opacity: 1;
      transform: translateX(calc(var(--p) * 100cqw)) scale(1);
    }
  }

  .seek.held .knob {
    transform: translateX(calc(var(--p) * 100cqw)) scale(1.15);
  }

  .time {
    position: absolute;
    bottom: calc(100% + var(--s1));
    left: calc(var(--p) * 100%);
    transform: translateX(-50%);
    padding: 1px var(--s1);
    border-radius: var(--r-chip);
    background: var(--surface-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      var(--shadow-soft);
    color: var(--ink);
    font: 600 var(--fs-1) var(--font-mono);
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .line,
    .knob,
    .running .ink,
    .running .knob {
      transition: opacity var(--t-quick);
    }
  }
</style>
