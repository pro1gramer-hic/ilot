<!-- The island (docs/09-DEMO-FINALE.md, sections 2 to 4): a pill the core places anywhere
     on the screen, and openings of their own size, all on springs. A liquid layer draws
     the black shape (and the music bubble stuck to it); above it come the relief (rims,
     a light under the cursor, a slight tilt), the opening's content and Mochi. The hub
     (a click on the pill) and the large panel (« Réglages ») are cards of their own,
     under the pill or above it. « Transparente au repos » makes it see-through, in place,
     while nothing goes on (stores/rest.svelte.ts). -->
<script lang="ts">
  import { untrack } from "svelte";
  import { inTauri, islandDragStart, islandMoveBy, releaseFocus } from "../lib/ipc";
  import { durationOf } from "../lib/island/banners";
  import { island } from "../lib/island/controller.svelte";
  import {
    BAR_HEIGHT,
    EDGE,
    type IslandMode,
    NEAR_PX,
    WINDOW_HEIGHT,
    WINDOW_WIDTH,
    bubbleBox,
    distanceTo,
    hubBox,
    hubHeight,
    islandBox,
    panelBox,
    scaleFor,
    sheenAt,
    stretchOf,
    tiltFor,
  } from "../lib/island/model";
  import type { TransitionConfig } from "svelte/transition";
  import { ENTER_MS, bloom, reduced, wilt } from "../lib/motion.svelte";
  import { i18n } from "../lib/i18n/i18n.svelte";
  import { RESPONSE_S } from "../lib/spring";
  import { Spring } from "../lib/spring.svelte";
  import { banners } from "../lib/stores/banners.svelte";
  import { chat } from "../lib/stores/chat.svelte";
  import { media } from "../lib/stores/media.svelte";
  import { settings } from "../lib/stores/settings.svelte";
  import Bubble from "./Bubble.svelte";
  import Hub from "./hub/Hub.svelte";
  import Confetti from "./island/Confetti.svelte";
  import DropView from "./island/DropView.svelte";
  import EventView from "./island/EventView.svelte";
  import FileView from "./island/FileView.svelte";
  import IslandMochi from "./island/IslandMochi.svelte";
  import MessageView from "./island/MessageView.svelte";
  import PillView from "./island/PillView.svelte";
  import WardrobeView from "./island/WardrobeView.svelte";
  import VoiceDot from "./island/VoiceDot.svelte";
  import VoiceView from "./island/VoiceView.svelte";
  import TranslationView from "./island/TranslationView.svelte";
  import MenuView from "./MenuView.svelte";
  import MusicBubble from "./music/MusicBubble.svelte";
  import MusicOpening from "./music/MusicOpening.svelte";
  import NewTrack from "./music/NewTrack.svelte";
  import PanelView from "./PanelView.svelte";
  import PermissionView from "./PermissionView.svelte";

  /** A spring `times` as stiff as the island's own (the demo's K × factor). */
  const stiffer = (times: number): number => RESPONSE_S / Math.sqrt(times);

  const W = new Spring(island.size.width);
  const H = new Spring(island.size.height);
  const R = new Spring(island.size.radius);
  const S = new Spring(1, stiffer(1.4), 0.7);
  const AX = new Spring(island.placement.anchorX, stiffer(0.7), 0.78);
  const AY = new Spring(island.placement.anchorY, stiffer(0.7), 0.78);
  const TX = new Spring(0, stiffer(0.5), 0.9);
  const TY = new Spring(0, stiffer(0.5), 0.9);
  /** Sideways shake of a failure: kicked, it swings a few times and settles. */
  const XO = new Spring(0, stiffer(3), 0.12);
  /** The panel, from 0 (closed) to 1 (open). */
  const P = new Spring(0, stiffer(0.9), 0.86);
  /** The hub comes out of the island on the panel's spring; its height has its own. */
  const HB = new Spring(0, stiffer(0.9), 0.86);
  const HH = new Spring(hubHeight(island.hubView));
  /** The music bubble: its size, its width (the scrolling title is wider), its gap. */
  const SD = new Spring(0, RESPONSE_S, 0.72);
  const SW = new Spring(40);
  const SO = new Spring(-40, stiffer(0.8), 0.8);

  /** The browser preview moves the island itself (the core does it in the app). */
  const preview = $state({ dragging: false });

  // A poke, a landing, a swallow: the island gives a little and springs back.
  island.kick = (velocity) => H.push(velocity);

  $effect(() => {
    const { width, height, radius } = island.size;
    const stretched = height + (island.mode === "pill" ? stretchOf(island.pull) * 0.85 : 0);
    W.set(width);
    H.set(stretched);
    R.set(Math.min(radius, stretched / 2));
  });

  $effect(() => {
    if (preview.dragging) return;
    AX.set(island.placement.anchorX);
    AY.set(island.placement.anchorY);
  });

  /** The island's box now, as its springs draw it (window coordinates). */
  const box = $derived.by(() => {
    const width = W.value;
    const height = H.value;
    const x = Math.min(Math.max(EDGE, AX.value - width / 2), Math.max(EDGE, WINDOW_WIDTH - width - EDGE));
    const y = island.placement.opensDown ? AY.value - BAR_HEIGHT / 2 : AY.value + BAR_HEIGHT / 2 - height;
    return { x, y, width, height };
  });

  const relief = $derived(settings.value.modes.relief);
  const near = $derived(island.mode === "pill" && distanceTo(box, island.pointer.x, island.pointer.y) < NEAR_PX);

  $effect(() => {
    S.set(scaleFor({ dragging: island.dragging, pressed: island.pressed, near }));
  });

  $effect(() => {
    const lean = relief === "3d" ? tiltFor(island.pointer, box, island.mode !== "pill") : { x: 0, y: 0 };
    TX.set(lean.x);
    TY.set(lean.y);
  });

  const light = $derived(sheenAt(island.pointer, box));
  const transform = $derived.by(() => {
    const tilt = Math.abs(TX.value) + Math.abs(TY.value) > 0.02;
    const lean = tilt ? `perspective(900px) rotateX(${TX.value}deg) rotateY(${TY.value}deg) ` : "";
    return `translateX(${XO.value}px) ${lean}scale(${S.value})`;
  });

  // A failure shakes its head, once the opening has its size.
  let shaken = -1;
  $effect(() => {
    if (banners.current?.kind !== "failed" || banners.serial === shaken) return;
    shaken = banners.serial;
    const timer = setTimeout(() => XO.push(760), 140);
    return () => clearTimeout(timer);
  });

  // The music bubble sticks to the folded pill while music plays (« Musique dans l'île »).
  const bubbleShown = $derived(
    island.mode === "pill" && media.shown?.playing === true && settings.value.modes.music !== "aucun",
  );
  $effect(() => {
    SD.set(bubbleShown ? 40 : 0);
    SO.set(bubbleShown ? 10 : -40);
    SW.set(settings.value.modes.music === "titre" ? 190 : 40);
  });
  const bubble = $derived.by(() => {
    const size = Math.max(0, SD.value);
    return bubbleBox(box, Math.max(size, (SW.value * size) / 40), size, SO.value);
  });
  const liquid = $derived(bubble.height > 0.5);

  // Click-through follows where the island, the bubble and the panel are headed (not
  // every frame of their springs).
  $effect(() => {
    const rest = islandBox(island.size, island.placement);
    const width = settings.value.modes.music === "titre" ? 190 : 40;
    island.syncShape(bubbleShown ? [{ ...bubbleBox(rest, width, 40, 10), radius: 20 }] : []);
  });

  // The panel: it comes from the island, a little blurred and smaller, and leaves faster.
  $effect(() => P.set(island.panelOpen ? 1 : 0));
  const panelShown = $derived(island.panelOpen || P.value > 0.01);
  const panel = $derived(panelBox(island.placement));
  const panelStyle = $derived.by(() => {
    const p = Math.min(1.05, P.value);
    const down = island.placement.opensDown;
    const originX = box.x + box.width / 2 - panel.x;
    const blur = p < 0.98 ? `filter: blur(${((1 - Math.min(1, p)) * 6).toFixed(2)}px);` : "";
    return (
      `left: ${panel.x}px; top: ${panel.y}px; width: ${panel.width}px; height: ${panel.height}px;` +
      `opacity: ${Math.min(1, Math.max(0, p))}; transform-origin: ${originX}px ${down ? -40 : panel.height + 40}px;` +
      `transform: translateY(${(1 - p) * (down ? -18 : 18)}px) scale(${0.94 + 0.06 * p}); ${blur}`
    );
  });

  // The hub comes out of the island the same way, below it or above it; it stays against
  // the island's shape as it is drawn (a banner pushes it along), and its height follows
  // the view on a spring of its own. Its content keeps the size it is heading to.
  $effect(() => HB.set(island.hubOpen ? 1 : 0));
  $effect(() => HH.set(hubHeight(island.hubView)));
  const hubShown = $derived(island.hubOpen || HB.value > 0.01);
  const hub = $derived(hubBox(box, island.placement.opensDown, HH.value));
  const hubTarget = $derived(hubBox(islandBox(island.size, island.placement), island.placement.opensDown, hubHeight(island.hubView)));
  const hubStyle = $derived.by(() => {
    const p = Math.min(1.05, HB.value);
    const down = island.placement.opensDown;
    const originX = box.x + box.width / 2 - hub.x;
    const blur = p < 0.98 ? `filter: blur(${((1 - Math.min(1, p)) * 6).toFixed(2)}px);` : "";
    return (
      `left: ${hub.x}px; top: ${hub.y}px; width: ${hub.width}px; height: ${hub.height}px;` +
      `opacity: ${Math.min(1, Math.max(0, p))}; transform-origin: ${originX}px ${down ? -BAR_HEIGHT / 2 : hub.height + BAR_HEIGHT / 2}px;` +
      `transform: translateY(${(1 - p) * (down ? -14 : 14)}px) scale(${0.92 + 0.08 * p}); ${blur}`
    );
  });

  /** With reduced motion the springs jump: the hub fades in and out instead. */
  const fade: (node: Element) => TransitionConfig = () =>
    reduced() ? { duration: ENTER_MS, css: (t) => `opacity: ${t}` } : { duration: 0 };

  // Folding back while îlot holds the keyboard would leave keystrokes going nowhere: the
  // focus goes back to the window the user was in.
  let previous: IslandMode = island.mode;
  $effect(() => {
    const mode = island.mode;
    const open = island.panelOpen;
    untrack(() => {
      const folded = previous !== mode && mode === "pill" && !open;
      if (folded && island.focused) void releaseFocus();
      previous = mode;
    });
  });
  let hubWasOpen = island.hubOpen;
  $effect(() => {
    const open = island.hubOpen;
    untrack(() => {
      const folded = hubWasOpen && !open && !island.panelOpen && island.mode === "pill";
      if (folded && island.focused) void releaseFocus();
      hubWasOpen = open;
    });
  });

  /** Each opening's content gets a new entrance; the same kind shown again too. */
  const layerKey = $derived.by(() => {
    switch (island.mode) {
      case "permission":
        return `permission:${island.permission?.id}`;
      case "finished":
      case "failed":
      case "message":
      case "music":
      case "track":
      case "file":
      case "toast":
      case "voice":
      case "translation":
        return `${island.mode}:${banners.serial}`;
      default:
        return island.mode;
    }
  });

  /** Openings that fit their content report its height (only the one on screen counts). */
  function measure(node: HTMLElement, mode: IslandMode): { destroy: () => void } {
    const observer = new ResizeObserver(() => {
      if (island.mode === mode) island.measures.contentHeight = node.offsetHeight;
    });
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }

  // Moving the island: past 6 px, the core carries the window under the cursor (in the
  // browser preview, the page moves the anchor itself). A click right after does nothing.
  const DRAG_FROM_PX = 6;
  const STEP_PX = 24;
  let press: { x: number; y: number; ax: number; ay: number } | null = null;
  let justDragged = false;

  function inControl(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest("button, input, textarea, a, form, [data-mochi]") !== null;
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0 || inControl(event.target)) return;
    island.pressed = true;
    if (island.mode === "pill") press = { x: event.clientX, y: event.clientY, ax: AX.value, ay: AY.value };
  }

  function onwindowmove(event: PointerEvent): void {
    if (!inTauri) {
      const inside = distanceTo(box, event.clientX, event.clientY) === 0;
      island.pointer = { x: event.clientX, y: event.clientY, inside, near: distanceTo(box, event.clientX, event.clientY) < 160 };
    }
    if (!press) return;
    const dx = event.clientX - press.x;
    const dy = event.clientY - press.y;
    if (!island.dragging && Math.hypot(dx, dy) < DRAG_FROM_PX) return;
    if (!island.dragging) {
      island.pressed = false;
      island.closePanel();
      if (inTauri) {
        press = null;
        void islandDragStart();
        return;
      }
      island.dragging = true;
      preview.dragging = true;
    }
    AX.snap(Math.min(Math.max(press.ax + dx, island.size.width / 2 + EDGE), WINDOW_WIDTH - island.size.width / 2 - EDGE));
    AY.snap(Math.min(Math.max(press.ay + dy, EDGE + BAR_HEIGHT / 2), WINDOW_HEIGHT - EDGE - BAR_HEIGHT / 2));
  }

  function onwindowup(): void {
    island.pressed = false;
    if (preview.dragging) {
      preview.dragging = false;
      island.dragging = false;
      justDragged = true;
      island.placement = { anchorX: AX.value, anchorY: AY.value, opensDown: AY.value < WINDOW_HEIGHT * 0.55 };
    }
    press = null;
  }

  function onclick(event: MouseEvent): void {
    if (justDragged) {
      justDragged = false;
      return;
    }
    if (!inControl(event.target)) island.activate();
  }

  function oncontextmenu(event: MouseEvent): void {
    event.preventDefault();
    island.openMenu();
  }

  /** Alt + arrows move the island by 24 px; Enter or Space opens what it shows. */
  function onkeydown(event: KeyboardEvent): void {
    if (event.target !== event.currentTarget) return;
    if (event.altKey && event.key.startsWith("Arrow")) {
      event.preventDefault();
      const dx = event.key === "ArrowLeft" ? -STEP_PX : event.key === "ArrowRight" ? STEP_PX : 0;
      const dy = event.key === "ArrowUp" ? -STEP_PX : event.key === "ArrowDown" ? STEP_PX : 0;
      if (inTauri) void islandMoveBy(dx, dy);
      else island.placement = { ...island.placement, anchorX: island.placement.anchorX + dx, anchorY: island.placement.anchorY + dy };
    } else if ((event.key === "Enter" || event.key === " ") && island.mode !== "permission") {
      event.preventDefault();
      island.activate();
    }
  }

  let confetti = $state<Confetti>();
  // A finished task: Mochi jumps and a few pixel confetti fly off the island's side.
  let celebrated = -1;
  $effect(() => {
    if (banners.current?.kind !== "finished" || banners.serial === celebrated || reduced()) return;
    celebrated = banners.serial;
    const { x, y, width, height } = box;
    const timer = setTimeout(() => confetti?.burst(x + Math.min(120, width / 3), island.placement.opensDown ? y + height + 10 : y - 10, 18), 220);
    return () => clearTimeout(timer);
  });

  const timeLeft = $derived(banners.current && island.mode === banners.current.kind ? banners.current : undefined);
  const label = $derived(island.mode === "pill" ? i18n.t.island.regionPill : i18n.t.island.region);
</script>

<svelte:window onpointermove={onwindowmove} onpointerup={onwindowup} />

<svg class="defs" width="0" height="0" aria-hidden="true">
  <filter id="goo" x="-20%" y="-20%" width="140%" height="140%">
    <feGaussianBlur in="SourceGraphic" stdDeviation="7" result="blur" />
    <feColorMatrix in="blur" mode="matrix" values="1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 22 -10" result="goo" />
    <feComposite in="SourceGraphic" in2="goo" operator="atop" />
  </filter>
</svg>

<!-- The black matter: the island and the bubble melt into each other while it shows. -->
<div class="matter tuck {relief}" class:liquid class:resting={island.resting} aria-hidden="true">
  <i
    class="blob"
    style:left="{box.x}px"
    style:top="{box.y}px"
    style:width="{box.width}px"
    style:height="{box.height}px"
    style:border-radius="{R.value}px"
    style:transform={transform}
  ></i>
  {#if liquid}
    <i
      class="blob"
      style:left="{bubble.x}px"
      style:top="{bubble.y}px"
      style:width="{bubble.width}px"
      style:height="{bubble.height}px"
      style:border-radius="{bubble.height / 2}px"
    ></i>
  {/if}
</div>

{#if island.mode === "permission"}
  <i
    class="halo"
    class:risk={island.permission?.risk === "high"}
    style:left="{box.x}px"
    style:top="{box.y}px"
    style:width="{box.width}px"
    style:height="{box.height}px"
    style:border-radius="{R.value}px"
    style:transform={transform}
    in:bloom={{ delay: 0 }}
    out:wilt
    aria-hidden="true"
  ></i>
{/if}

<!-- The island takes the keyboard (Alt + arrows move it, Enter opens it) and the pointer
     (drag, click), around buttons of its own. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="island tuck {relief}"
  class:dragging={island.dragging}
  class:locked={island.mode === "permission"}
  class:resting={island.resting}
  data-mode={island.mode}
  data-resting={island.resting}
  data-width={Math.round(box.width)}
  data-height={Math.round(box.height)}
  role="region"
  aria-label={label}
  tabindex="0"
  style:left="{box.x}px"
  style:top="{box.y}px"
  style:width="{box.width}px"
  style:height="{box.height}px"
  style:border-radius="{R.value}px"
  style:transform={transform}
  style:--mx="{light.x}%"
  style:--my="{light.y}%"
  onpointerenter={() => island.setHovered(true)}
  onpointerleave={() => island.setHovered(false)}
  {onpointerdown}
  {onclick}
  {oncontextmenu}
  {onkeydown}
>
  <i class="sheen" aria-hidden="true"></i>
  {#key layerKey}
    <div class="layer" style:width="{island.size.width}px" in:bloom out:wilt use:measure={island.mode}>
      {#if island.mode === "pill"}
        <PillView />
      {:else if island.mode === "finished" || island.mode === "failed" || island.mode === "toast"}
        <EventView />
      {:else if island.mode === "permission"}
        <PermissionView />
      {:else if island.mode === "message"}
        <MessageView />
      {:else if island.mode === "music"}
        <MusicOpening />
      {:else if island.mode === "track"}
        <NewTrack />
      {:else if island.mode === "drop"}
        <DropView />
      {:else if island.mode === "wardrobe"}
        <WardrobeView />
      {:else if island.mode === "voice"}
        <VoiceView />
      {:else if island.mode === "translation"}
        <TranslationView />
      {:else if island.mode === "file"}
        <FileView />
      {:else}
        <MenuView />
      {/if}
    </div>
  {/key}
  {#if island.mode === "failed"}
    <i class="fail-edge" aria-hidden="true"></i>
  {/if}
  {#if timeLeft}
    {#key banners.serial}
      <i
        class="time"
        class:err={timeLeft.kind === "failed"}
        class:paused={banners.paused}
        style:animation-duration="{durationOf(timeLeft)}ms"
        aria-hidden="true"
      ></i>
    {/key}
  {/if}
  <IslandMochi width={box.width} height={box.height} />
  <VoiceDot />
</div>

{#if liquid}
  <div
    class="bubble"
    style:left="{bubble.x}px"
    style:top="{bubble.y}px"
    style:width="{bubble.width}px"
    style:height="{bubble.height}px"
    style:opacity={Math.min(1, Math.max(0, (bubble.height - 20) / 16))}
  >
    <MusicBubble onopen={() => island.openPanel("music")} ondetach={() => void media.detach()} />
  </div>
{/if}

{#if hubShown}
  <div class="hub-card {relief}" style={hubStyle} inert={!island.hubOpen} in:fade out:fade>
    <div class="hub-inner" style:height="{hubTarget.height}px">
      <Hub onburst={(x, y) => confetti?.burst(x, y, 24)} onopenchat={(model, provider) => chat.open({ provider, model })} />
    </div>
  </div>
{/if}

{#if panelShown}
  <div class="panel" style={panelStyle} inert={!island.panelOpen}>
    <PanelView />
  </div>
{/if}

<Bubble {box} opensDown={island.placement.opensDown} />
<Confetti bind:this={confetti} />

<style>
  .defs {
    position: absolute;
  }

  /* The liquid layer: plain shadows, or the goo filter while the bubble shows. */
  .matter {
    position: absolute;
    inset: 0;
    pointer-events: none;
    filter: var(--drop);
  }

  .matter.\33 d {
    filter: var(--drop-3d);
  }

  .matter.liquid {
    filter: url(#goo) var(--drop);
  }

  .matter.\33 d.liquid {
    filter: url(#goo) var(--drop-3d);
  }

  .blob,
  .halo {
    position: absolute;
    transform-origin: 50% 50%;
  }

  .blob {
    background: var(--island);
  }

  /* The permission breathes an amber light: two fixed glows, only their opacity moves. */
  .halo {
    pointer-events: none;
    box-shadow: var(--halo-rest);
  }

  .halo::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: var(--halo-peak);
    opacity: 0;
    animation: halo var(--t-halo) ease-in-out infinite;
  }

  .halo.risk::after {
    box-shadow: var(--halo-risk);
  }

  @keyframes halo {
    50% {
      opacity: 1;
    }
  }

  .island {
    position: absolute;
    overflow: hidden;
    color: var(--ink);
    cursor: pointer;
    transform-origin: 50% 50%;
    box-shadow: var(--rim-plat);
    outline-offset: 3px;
  }

  .island.leger {
    box-shadow: var(--rim-leger);
  }

  .island.\33 d {
    box-shadow: var(--rim-3d);
  }

  .island.locked {
    cursor: default;
  }

  .island.dragging {
    cursor: grabbing;
  }

  .island.dragging .layer {
    pointer-events: none;
  }

  /* A light that follows the cursor, a lit top edge and a darker bottom. */
  .sheen {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.4s;
    background:
      radial-gradient(240px 110px at var(--mx, 50%) var(--my, -20%), var(--sheen-spot), transparent 70%),
      linear-gradient(180deg, var(--sheen-top), transparent 40%, transparent 72%, var(--sheen-low));
  }

  .leger .sheen {
    opacity: 0.55;
  }

  .\33 d .sheen {
    opacity: 1;
  }

  /* Every opening's content sits at the top center, at the opening's own width. */
  .layer {
    position: absolute;
    top: 0;
    left: 50%;
    translate: -50% 0;
  }

  .fail-edge {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: var(--fail-edge);
    pointer-events: none;
  }

  /* Time left before the opening folds: it stops while hovered or covered. */
  .time {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: 2px;
    background: var(--accent);
    opacity: 0.55;
    transform-origin: left;
    animation: time-left linear forwards;
    pointer-events: none;
  }

  .time.err {
    background: var(--err);
  }

  .time.paused {
    animation-play-state: paused;
  }

  @keyframes time-left {
    from {
      transform: scaleX(0);
    }
  }

  .bubble {
    position: absolute;
    display: grid;
    overflow: hidden;
    border-radius: var(--r-pill);
  }

  .panel {
    position: absolute;
    display: grid;
  }

  /* The hub is made of the island's own black matter, with the same rims. */
  .hub-card {
    position: absolute;
    overflow: hidden;
    border-radius: var(--r-panel);
    background: var(--island);
    box-shadow: var(--rim-plat), var(--shadow);
  }

  .hub-card.leger {
    box-shadow: var(--rim-leger), var(--shadow);
  }

  .hub-card.\33 d {
    box-shadow: var(--rim-3d), var(--shadow);
  }

  /* « Transparente au repos »: the island stays in place, clickable, and turns
     see-through; it comes back faster than it fades. */
  .tuck {
    transition: opacity var(--t-enter) var(--ease-out);
  }

  .tuck.resting {
    opacity: var(--rest-opacity);
    transition-duration: var(--t-in);
  }

  @media (prefers-reduced-motion: reduce) {
    .halo::after {
      animation: none;
      opacity: 0.5;
    }

  }
</style>
