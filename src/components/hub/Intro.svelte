<!-- The hub's intro (09, section 8 bis), at launch and on « Revoir l'intro »: a starry sky
     moving away from the center, Mochi waking up (eyes closed, a blink, a breath), a small
     ball that bounces in and that he catches with a squash and a glow, a sway, then the
     home shows. At launch (`greet`), he waves hello before the home (prompt 12). One clock
     drives it all and stops with it, or while the island is hidden. Reduced motion: still
     stars, no ball, Mochi's still frames, the same beats. -->
<script lang="ts">
  import { BALL_MS, introBeat, introLength } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { reduced, stillness } from "../../lib/motion.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { sound } from "../../lib/stores/sound.svelte";

  let { ondone, greet = false }: { ondone: () => void; greet?: boolean } = $props();

  const STARS = 140;
  /** A star goes this far from the center before it starts again near it. */
  const REACH = 420;

  type Star = { angle: number; radius: number; speed: number; size: number };

  let width = $state(0);
  let height = $state(0);
  let canvas = $state<HTMLCanvasElement>();
  let t = $state(0);
  const beat = $derived(introBeat(t, greet));
  const length = $derived(introLength(greet));

  // The greeting has its little tune.
  $effect(() => {
    if (beat.phase === "hello") sound.play("hello");
  });

  const stars: Star[] = Array.from({ length: STARS }, () => ({
    angle: Math.random() * Math.PI * 2,
    radius: Math.random() * 120,
    speed: 20 + Math.random() * 80,
    size: Math.random() < 0.15 ? 2 : 1,
  }));

  function paint(seconds: number): void {
    const context = canvas?.getContext("2d");
    if (!context || width === 0) return;
    const ratio = devicePixelRatio;
    const style = getComputedStyle(canvas!);
    const colors = [style.getPropertyValue("--ink").trim(), style.getPropertyValue("--accent-text").trim()];
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);
    for (const star of stars) {
      star.radius += star.speed * seconds;
      if (star.radius > REACH) {
        star.radius = Math.random() * 20;
        star.angle = Math.random() * Math.PI * 2;
      }
      const x = width / 2 + Math.cos(star.angle) * star.radius * 1.6;
      const y = height / 2 + Math.sin(star.angle) * star.radius * 0.6;
      context.globalAlpha = Math.min(1, star.radius / 120) * 0.8;
      context.fillStyle = colors[star.size > 1 ? 1 : 0];
      context.fillRect(Math.round(x), Math.round(y), star.size, star.size);
    }
    context.globalAlpha = 1;
  }

  $effect(() => {
    if (stillness.current || width === 0) return;
    const still = reduced();
    let frame = 0;
    let last = performance.now();
    paint(0);
    const tick = (now: number): void => {
      const seconds = Math.min(0.05, (now - last) / 1000);
      last = now;
      t += seconds * 1000;
      if (!still) paint(seconds);
      if (t >= length) {
        ondone();
        return;
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
</script>

<div class="intro" bind:clientWidth={width} bind:clientHeight={height}>
  <canvas bind:this={canvas} width={Math.round(width * devicePixelRatio)} height={Math.round(height * devicePixelRatio)} aria-hidden="true"></canvas>
  <span class="mochi" aria-hidden="true">
    <span class="halo" class:caught={beat.phase === "catch"}>
      <Mochi move={beat.move} face={beat.face} scale={5} shadow={false} label="" dressed />
    </span>
  </span>
  {#if beat.phase === "ball"}
    <i class="ball" style:animation-duration="{BALL_MS + 150}ms" aria-hidden="true"></i>
  {/if}
  <p class="visually-hidden" role="status">
    {beat.phase === "hello" ? i18n.t.hub.intro.hello(settings.value.mochi.name) : i18n.t.hub.intro.wakes(settings.value.mochi.name)}
  </p>
</div>

<style>
  .intro {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: var(--r-card);
    background: var(--panel);
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .mochi {
    position: absolute;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    animation: appear 0.7s var(--ease-out) both;
  }

  .halo {
    display: block;
    filter: drop-shadow(0 0 24px color-mix(in srgb, var(--ink) 12%, transparent));
  }

  .halo.caught {
    animation: glow 0.8s var(--ease-out);
  }

  /* A small ball that bounces in from the left and lands in Mochi's arms. */
  .ball {
    position: absolute;
    left: calc(50% - 90px);
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--ink);
    box-shadow: 0 0 12px color-mix(in srgb, var(--ink) 50%, transparent);
    opacity: 0;
    animation: ball cubic-bezier(0.3, 0.6, 0.5, 1) forwards;
  }

  @keyframes appear {
    from {
      opacity: 0;
      transform: scale(0.6);
      filter: blur(8px);
    }
  }

  @keyframes glow {
    50% {
      filter: drop-shadow(0 0 30px color-mix(in srgb, var(--ink) 45%, transparent));
    }
  }

  @keyframes ball {
    0% {
      opacity: 0;
      transform: translate(-80px, -40px);
    }

    15% {
      opacity: 1;
    }

    35% {
      transform: translate(-30px, 18px);
    }

    55% {
      transform: translate(0, -8px);
    }

    75% {
      transform: translate(28px, 18px);
    }

    90% {
      opacity: 1;
      transform: translate(48px, 6px) scale(0.8);
    }

    100% {
      opacity: 0;
      transform: translate(58px, 4px) scale(0.2);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mochi {
      animation-name: fade;
    }

    .halo.caught {
      animation: none;
    }

    .ball {
      display: none;
    }

    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
