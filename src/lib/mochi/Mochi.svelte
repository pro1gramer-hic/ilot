<script lang="ts">
  import { prefersReducedMotion } from "svelte/motion";
  import { devicePixelRatio } from "svelte/reactivity/window";
  import { settings } from "../stores/settings.svelte";
  import { wardrobe } from "../stores/wardrobe.svelte";
  import { browserClock, playFrames } from "./clock";
  import { backingSize, drawMochi, GRID_SIZE, lookToward } from "./engine";
  import { drawDressed, type Outfit } from "./outfits";
  import { isHead, type FaceId } from "./faces";
  import type { KindId } from "./kinds";
  import type { Expression } from "./moves";
  import type { PaletteId } from "./palettes";
  import { drawBox, drawThought, framePoseAt, stillFramePose, type MochiMove, type PoseFrame } from "./poses";

  interface Props {
    /** Body color; the one chosen in the settings by default. */
    palette?: PaletteId;
    /** Silhouette; the one chosen in the settings by default. */
    kind?: KindId;
    /** One of the reference's moves, or a pose of poses.ts (« think », « box »). */
    move?: MochiMove;
    /** Imposed face, at every instant; when absent, the move's own faces play. */
    face?: FaceId;
    /** Face at rest, in place of the move's plain `idle`; the settings' head by default. */
    rest?: FaceId;
    /** Big eyes (pointer over Mochi), unless a face is imposed. */
    big?: boolean;
    /**
     * Gaze, in place of the move's own: -1 (left), 0 or 1 (right), or a point in viewport
     * coordinates that Mochi looks toward once it leaves his box.
     */
    look?: number | { x: number };
    /** CSS pixels per unit of his 24-unit grid: 2 draws him in a 48 px square. */
    scale?: number;
    /** `false` hides the ground shadow (small previews). */
    shadow?: boolean;
    /** `false` shows a single still frame. */
    animated?: boolean;
    /** Accessible name of the image; empty when Mochi only decorates a labeled control. */
    label?: string;
    /** Mochi himself (not a session's icon): he wears what his wardrobe and the season
     * give him. */
    dressed?: boolean;
    /** An outfit of its own (the wardrobe's previews), in place of his. */
    outfit?: Outfit;
  }

  let {
    palette,
    kind,
    move = "breathe",
    face,
    rest,
    big = false,
    look,
    scale = 1,
    shadow = true,
    animated = true,
    label = "Mochi",
    dressed = false,
    outfit,
  }: Props = $props();

  let canvas = $state<HTMLCanvasElement>();
  let onScreen = $state(true);
  let visibility = $state<DocumentVisibilityState>("visible");

  const chosen = $derived(settings.value.mochi);
  const paletteNow = $derived(palette ?? chosen.palette);
  const kindNow = $derived(kind ?? chosen.kind);
  const restNow = $derived(rest ?? (isHead(chosen.face) ? chosen.face : undefined));
  // A point becomes -1, 0 or 1: the effect below only restarts when that number changes.
  const gaze = $derived.by(() => {
    if (typeof look !== "object") return look;
    const box = canvas?.getBoundingClientRect();
    return box ? lookToward(look.x, box.left, box.right) : 0;
  });

  // Drawn at the screen's pixel ratio, so he stays sharp at any size and on any screen.
  const pixels = $derived(backingSize(scale, devicePixelRatio.current ?? 1));
  // Still frame (the face of the moment) when animations are off, reduced motion is
  // requested, the page is hidden or the canvas is off screen: no timer at all then.
  const playing = $derived(
    animated && onScreen && visibility !== "hidden" && !prefersReducedMotion.current,
  );

  const wearing: Outfit | undefined = $derived(outfit ?? (dressed ? wardrobe.worn : undefined));

  // Animation clock: restarts when the move changes, so each move plays from its start,
  // and keeps running across palette, face, size or visibility changes.
  let origin = 0;
  let originMove: MochiMove | undefined;

  // His breathing plays for as long as an agent works, often hours: 10 frames/s are
  // enough for a breath and spare the window being composited 30 times a second.
  const EVERY: Partial<Record<MochiMove, number>> = { breathe: 3 };

  // His eyes glide to where he looks instead of jumping there (eyes on a sphere, prompt
  // 12): each 30 frames/s frame covers a third of the way left, about 150 ms in all, and
  // a slower move's frame covers as much as the frames it skips.
  const GLIDE = 0.35;
  let shownLook = 0;
  const glide = (target: number, every: number): number => {
    const next = shownLook + (target - shownLook) * (1 - (1 - GLIDE) ** every);
    shownLook = Math.abs(target - next) < 0.01 ? target : next;
    return shownLook;
  };

  $effect(() => {
    if (!canvas) return;
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) onScreen = entry.isIntersecting;
    });
    observer.observe(canvas);
    return () => observer.disconnect();
  });

  $effect(() => {
    const ctx = canvas?.getContext("2d");
    if (!ctx) return;
    const appearance = { palette: paletteNow, kind: kindNow, shadow, sphere: true };
    const moveNow = move;
    const expression: Expression = { face, big, rest: restNow, look: gaze };
    const size = pixels;
    const dress = wearing;
    const live = playing;
    const every = EVERY[moveNow] ?? 1;
    let drawn = "";
    const paint = (params: PoseFrame): void => {
      const frame = { ...params, ...appearance };
      frame.look = live ? glide(frame.look ?? 0, every) : (shownLook = frame.look ?? 0);
      // The same pose as the frame on screen: nothing to repaint (a waving arm always moves).
      const key = frame.wave ? "" : JSON.stringify(frame);
      if (key && key === drawn) return;
      drawn = key;
      if (frame.box) {
        drawBox(ctx, size, frame);
        return;
      }
      if (dress) drawDressed(ctx, size, frame, dress, browserClock.now(), live);
      else drawMochi(ctx, size, frame, browserClock.now());
      if (frame.thought) drawThought(ctx, frame.thought);
    };
    if (!live) {
      paint(stillFramePose(moveNow, expression));
      return;
    }
    if (moveNow !== originMove) {
      origin = browserClock.now();
      originMove = moveNow;
    }
    return playFrames((t) => paint(framePoseAt(moveNow, t, expression)), origin, undefined, every);
  });
</script>

<svelte:document bind:visibilityState={visibility} />

<span
  class="mochi"
  role={label ? "img" : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : true}
  style:--mochi-size="{GRID_SIZE * scale}px"
>
  <canvas bind:this={canvas} width={pixels} height={pixels} aria-hidden="true"></canvas>
</span>

<style>
  .mochi {
    display: block;
    width: fit-content;
  }

  canvas {
    display: block;
    width: var(--mochi-size);
    height: var(--mochi-size);
  }
</style>
