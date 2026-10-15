<!-- A record that turns at 33 rpm while the music plays: it slows down to a stop on pause
     and speeds up again on play. Fine grooves with smooth bands between the tracks, the
     cover as its label, and a reflection that stays still while the disc turns under it.
     The grooves are perfect rings: only the label needs to turn to show it, a small
     layer whose turning box never leaves the record. It only turns while it is on
     screen. Decorative: the title is written next to it. -->
<script lang="ts">
  import { reduced, stillness } from "../lib/motion.svelte";
  import { onScreen, watchInert } from "./player/presence.svelte";

  let {
    size = 18,
    fill = false,
    artwork,
    playing = false,
  }: { size?: number; fill?: boolean; artwork?: string | null; playing?: boolean } = $props();

  /** One turn at 33⅓ rpm. */
  const TURN_MS = 1800;
  /** A record takes a moment to reach its speed, and longer to stop. */
  const SPIN_UP_MS = 700;
  const SPIN_DOWN_MS = 1600;

  let root = $state<HTMLElement>();
  let label = $state<HTMLElement>();
  let spin: Animation | undefined;
  let rate = 0;
  let frame = 0;
  /** Play state seen by the last run: only a change of it speeds up or slows down. */
  let wasPlaying: boolean | undefined;

  $effect(() => watchInert());

  const visible = $derived(root ? onScreen(root) : false);

  $effect(() => {
    if (!label) return;
    spin = label.animate([{ transform: "rotate(0turn)" }, { transform: "rotate(1turn)" }], {
      duration: TURN_MS,
      iterations: Infinity,
    });
    spin.pause();
    return () => {
      cancelAnimationFrame(frame);
      spin?.cancel();
      spin = undefined;
    };
  });

  // Only the playback rate moves, a few frames at a time: the turning itself stays on
  // the compositor. Stopped, the animation is paused and costs nothing. Coming back on
  // screen, a playing record is already at speed.
  $effect(() => {
    const running = visible && !stillness.current && !reduced();
    const target = playing && running ? 1 : 0;
    const animation = spin;
    if (!animation) return;
    cancelAnimationFrame(frame);
    const ramp = running && wasPlaying !== undefined && wasPlaying !== playing;
    wasPlaying = playing;
    if (!ramp) {
      rate = target;
      animation.updatePlaybackRate(Math.max(rate, 0.0001));
      if (target > 0) animation.play();
      else animation.pause();
      return;
    }
    if (animation.playState !== "running") animation.play();
    const from = rate;
    const start = performance.now();
    const duration = (target > from ? SPIN_UP_MS : SPIN_DOWN_MS) * Math.abs(target - from);
    const step = (now: number): void => {
      const t = duration === 0 ? 1 : Math.min(1, (now - start) / duration);
      // Ease out when stopping (friction), ease in when starting (the motor pulls).
      const eased = target > from ? t * t : 1 - (1 - t) * (1 - t);
      rate = from + (target - from) * eased;
      animation.updatePlaybackRate(Math.max(rate, 0.0001));
      if (t < 1) frame = requestAnimationFrame(step);
      else if (target === 0) animation.pause();
    };
    frame = requestAnimationFrame(step);
  });
</script>

<span class="vinyl" class:fill style:--size="{size}px" aria-hidden="true" bind:this={root}>
  <span class="disc">
    <span class="label" bind:this={label}>
      {#if artwork}<img src={artwork} alt="" />{/if}
    </span>
  </span>
  <span class="shine"></span>
</span>

<style>
  .vinyl {
    position: relative;
    display: block;
    width: var(--size);
    height: var(--size);
    flex: none;
    border-radius: 50%;
  }

  .vinyl.fill {
    width: 100%;
    height: 100%;
  }

  .disc,
  .shine {
    position: absolute;
    inset: 0;
    border-radius: 50%;
  }

  /* Grooves: thin rings, every 2 px, from the rim to the label. On a large record, smooth
     bands show where a track ends (too thin to show on a small one). */
  .disc {
    display: grid;
    place-items: center;
    background:
      radial-gradient(
        circle closest-side,
        transparent 0 40%,
        var(--vinyl) 40% 46%,
        transparent 46% 61%,
        var(--vinyl) 61% 62.5%,
        transparent 62.5% 78%,
        var(--vinyl) 78% 79.5%,
        transparent 79.5% 96%,
        var(--vinyl) 96%
      ),
      repeating-radial-gradient(circle, var(--vinyl-groove) 0 0.6px, transparent 0.6px 2px),
      var(--vinyl);
    box-shadow: 0 0 0 1px var(--line);
  }

  .label {
    width: 40%;
    height: 40%;
    border-radius: 50%;
    overflow: hidden;
    background: var(--accent);
    /* The spindle hole. */
    mask: radial-gradient(circle, transparent 0 7%, var(--opaque) 8%);
    will-change: transform;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* Light from above: it stays put while the record turns. */
  .shine {
    background: conic-gradient(
      from 20deg,
      transparent 0 8%,
      var(--vinyl-shine) 13%,
      transparent 19% 58%,
      var(--vinyl-shine) 63%,
      transparent 69%
    );
    mask: radial-gradient(circle, transparent 0 21%, var(--opaque) 22%);
    opacity: 0.7;
    pointer-events: none;
  }
</style>
