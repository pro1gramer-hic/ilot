<!-- A few pixel confetti when a task ends (docs/09-DEMO-FINALE.md, section 4): squares
     that fall on 2 px steps, in the theme's colors. The frame loop runs only while some
     are in the air; none at all under reduced motion. -->
<script lang="ts">
  import { WINDOW_HEIGHT, WINDOW_WIDTH } from "../../lib/island/model";
  import { reduced } from "../../lib/motion.svelte";

  type Piece = { x: number; y: number; vx: number; vy: number; life: number; size: number; color: string };

  const GRAVITY = 600;
  const COLORS = ["--accent", "--ok", "--warn", "--ai-claude", "--ai-gemini", "--ink"];

  let canvas = $state<HTMLCanvasElement>();
  let pieces: Piece[] = [];
  let frame = 0;
  let last = 0;

  /** Throws `count` confetti from (`x`, `y`), in window coordinates. */
  export function burst(x: number, y: number, count: number): void {
    if (reduced() || !canvas) return;
    const style = getComputedStyle(canvas);
    const colors = COLORS.map((name) => style.getPropertyValue(name).trim());
    for (let i = 0; i < count; i += 1) {
      const angle = -Math.PI / 2 + (Math.random() - 0.5) * 2.2;
      const speed = 160 + Math.random() * 220;
      pieces.push({
        x,
        y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 0.9 + Math.random() * 0.6,
        size: Math.random() < 0.5 ? 2 : 4,
        color: colors[i % colors.length],
      });
    }
    if (frame === 0) {
      last = performance.now();
      frame = requestAnimationFrame(step);
    }
  }

  function step(now: number): void {
    const context = canvas?.getContext("2d");
    if (!context) return;
    const seconds = Math.min(0.05, (now - last) / 1000);
    last = now;
    context.clearRect(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT);
    pieces = pieces.filter((piece) => (piece.life -= seconds) > 0);
    for (const piece of pieces) {
      piece.vy += GRAVITY * seconds;
      piece.x += piece.vx * seconds;
      piece.y += piece.vy * seconds;
      context.globalAlpha = Math.min(1, piece.life * 2);
      context.fillStyle = piece.color;
      context.fillRect(Math.round(piece.x / 2) * 2, Math.round(piece.y / 2) * 2, piece.size, piece.size);
    }
    context.globalAlpha = 1;
    frame = pieces.length > 0 ? requestAnimationFrame(step) : 0;
  }

  $effect(() => () => cancelAnimationFrame(frame));
</script>

<canvas bind:this={canvas} width={WINDOW_WIDTH} height={WINDOW_HEIGHT} aria-hidden="true"></canvas>

<style>
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    image-rendering: pixelated;
  }
</style>
