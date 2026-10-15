<!-- Mini-player window (docs/09-DEMO-FINALE.md, sections 5 and 5 bis): a 300 × 64 pill
     with the turning record, the title, play-pause and the arrow that sends it back; for a
     video, a 352 × 198 frame with the picture Windows gives (never a copy of a protected
     video) and the same buttons over it. It appears as a drop that grows; it follows the
     pointer and keeps a little speed when let go; dropped near the island, or sent back by
     the arrow (or Escape), it shrinks into a drop, flows to the island and closes. -->
<script lang="ts">
  import { onMount } from "svelte";
  import Artwork from "./components/Artwork.svelte";
  import Icon from "./components/Icon.svelte";
  import Vinyl from "./components/Vinyl.svelte";
  import Glyph from "./components/music/Glyph.svelte";
  import RoundButton from "./components/music/RoundButton.svelte";
  import Transport from "./components/music/Transport.svelte";
  import { type Point, type Sample, keepRecent, nearHome, throwTarget, velocityOf } from "./components/music/fling";
  import { moveWindow, resizeWindow, screenBounds, windowAt } from "./components/music/mini-window";
  import { i18n } from "./lib/i18n/i18n.svelte";
  import { buddyHome, closeMiniPlayer } from "./lib/ipc";
  import { emptyState, isVideo, openLabel, sourceLabel, subtitle } from "./lib/media";
  import { EXIT_MS, reduced } from "./lib/motion.svelte";
  import { Spring } from "./lib/spring.svelte";
  import { media } from "./lib/stores/media.svelte";

  /** The pill, the video frame and the drop, in pixels. */
  const AUDIO = { width: 300, height: 64 };
  const VIDEO = { width: 352, height: 198 };
  const DROP = 40;
  /** Room around the pill for its shadow: the window is this much larger on each side. */
  const MARGIN = 24;
  /** Below this width the drop shows no content; it is all there 100 px further. */
  const REVEAL_FROM = 120;
  const REVEAL_OVER = 100;
  /** A press that moves less than this is a click, not a drag. */
  const DRAG_SLOP = 3;
  /** Time for the drop to flow into the island before the window closes. */
  const DOCK_MS = 420;

  const track = $derived(media.shown);
  const video = $derived(isVideo(track));
  const size = $derived(video ? VIDEO : AUDIO);
  const empty = $derived(emptyState(media.tab));

  const width = new Spring(DROP);
  const height = new Spring(DROP);
  /** Where the window goes after a throw, or on its way to the island. */
  const x = new Spring(0);
  const y = new Spring(0);

  let docking = $state(false);
  let gone = $state(false);
  let flying = false;
  let home: Point | null = null;
  let drag: { from: Point; window: Point; samples: Sample[]; moved: boolean } | null = null;

  const reveal = $derived(Math.min(1, Math.max(0, (width.value - REVEAL_FROM) / REVEAL_OVER)));
  const radius = $derived(video && width.value > REVEAL_FROM ? null : Math.min(height.value / 2, 32));

  // The window speaks the interface's language, and says so (title, screen readers).
  $effect(() => {
    document.documentElement.lang = i18n.locale;
    document.title = `${i18n.t.music.mini.window} · îlot`;
  });

  // The drop grows into the player, and shrinks back when it docks (with reduced motion
  // it keeps its size and only fades).
  $effect(() => {
    const shrink = docking && !reduced();
    width.set(shrink ? DROP : size.width);
    height.set(shrink ? DROP : size.height);
  });

  // The window fits the player, its shadow included.
  $effect(() => {
    void resizeWindow(size.width + 2 * MARGIN, size.height + 2 * MARGIN);
  });

  // A throw and the way back to the island move the window along its springs.
  $effect(() => {
    const to = { x: x.value, y: y.value };
    if (flying) moveWindow(to);
  });

  onMount(() => {
    locateHome();
    const escape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") dock();
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  });

  /** Where the island sits on the screen now (it may have moved). Unknown, the player
   * still docks by its arrow, without the flight. */
  function locateHome(): void {
    buddyHome().then(
      (point) => (home = point),
      () => (home = null),
    );
  }

  /** Sends the player back into the island, then closes its window. */
  function dock(): void {
    if (docking) return;
    docking = true;
    if (home) {
      const at = windowAt();
      x.snap(at.x);
      y.snap(at.y);
      flying = true;
      x.set(home.x - window.innerWidth / 2);
      y.set(home.y - window.innerHeight / 2);
    }
    setTimeout(
      () => {
        gone = true;
        void closeMiniPlayer();
      },
      reduced() ? EXIT_MS : DOCK_MS,
    );
  }

  function down(event: PointerEvent): void {
    if (event.button !== 0 || docking || (event.target as Element).closest("button")) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    flying = false;
    const from = { x: event.screenX, y: event.screenY };
    drag = { from, window: windowAt(), samples: [{ ...from, t: event.timeStamp }], moved: false };
    locateHome();
  }

  function move(event: PointerEvent): void {
    if (!drag) return;
    const dx = event.screenX - drag.from.x;
    const dy = event.screenY - drag.from.y;
    if (!drag.moved && Math.hypot(dx, dy) < DRAG_SLOP) return;
    drag.moved = true;
    drag.samples = keepRecent(drag.samples, { x: event.screenX, y: event.screenY, t: event.timeStamp });
    moveWindow({ x: drag.window.x + dx, y: drag.window.y + dy });
  }

  function up(event: PointerEvent): void {
    if (!drag) return;
    const { from, window: start, samples, moved } = drag;
    drag = null;
    if (!moved) return;
    const at = { x: start.x + event.screenX - from.x, y: start.y + event.screenY - from.y };
    const center = { x: at.x + window.innerWidth / 2, y: at.y + window.innerHeight / 2 };
    if (home && nearHome(center, home)) {
      dock();
      return;
    }
    const speed = velocityOf(samples);
    const box = { width: window.innerWidth, height: window.innerHeight };
    const rest = throwTarget(at, speed, box, screenBounds());
    x.snap(at.x);
    y.snap(at.y);
    x.push(speed.x);
    y.push(speed.y);
    flying = true;
    x.set(rest.x);
    y.set(rest.y);
  }
</script>

<main class="mini" class:docking class:gone data-state={gone ? "docked" : docking ? "docking" : "open"}>
  <div
    class="pill"
    class:video
    role="group"
    aria-label={i18n.t.music.mini.label}
    style:width="{width.value}px"
    style:height="{height.value}px"
    style:border-radius={radius === null ? undefined : `${radius}px`}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (drag = null)}
    ondragstart={(event) => event.preventDefault()}
  >
    <div class="content" style:opacity={reveal} inert={reveal < 0.5}>
      {#if video && track}
        <span class="picture"><Artwork {track} /></span>
        <div class="overlay">
          <p class="text">
            <b title={track.title}>{track.title}</b>
            <span>{sourceLabel(track)}</span>
          </p>
          <RoundButton size={32} aria-label={openLabel(track)} title={openLabel(track)} onclick={() => media.open(track)}>
            <Icon name="open" size={13} />
          </RoundButton>
          <Transport {track} toggleOnly solid={false} />
          <RoundButton size={32} aria-label={i18n.t.music.dock} title={i18n.t.music.dock} onclick={dock}>
            <Glyph name="dock" size={13} />
          </RoundButton>
        </div>
      {:else}
        <span class="round"><Vinyl fill artwork={track?.artwork} playing={track?.playing ?? false} /></span>
        <p class="text">
          {#if track}
            <b title={track.title}>{track.title}</b>
            <span>{media.notice ?? subtitle(track)}</span>
          {:else if media.error}
            <b>{i18n.t.music.mini.unavailable}</b>
            <span title={media.error}>{media.error}</span>
          {:else}
            <b>{empty.title}</b>
            <span title={empty.hint}>{empty.hint}</span>
          {/if}
        </p>
        <Transport {track} toggleOnly solid={false} />
        <RoundButton aria-label={i18n.t.music.dock} title={i18n.t.music.dock} onclick={dock}>
          <Glyph name="dock" size={14} />
        </RoundButton>
      {/if}
    </div>
  </div>
</main>

<style>
  /* The window is transparent: only the pill is drawn, centered, with room for its shadow. */
  .mini {
    height: 100vh;
    display: grid;
    place-items: center;
    overflow: hidden;
  }

  .pill {
    position: relative;
    overflow: hidden;
    background: var(--island);
    box-shadow:
      inset 0 0 0 1px var(--line),
      var(--hi),
      0 var(--s2) var(--s4) color-mix(in srgb, var(--opaque) 55%, transparent),
      0 var(--gap-mark) var(--s2) color-mix(in srgb, var(--opaque) 40%, transparent);
    cursor: grab;
    touch-action: none;
    transition: opacity var(--t-exit) var(--ease-out);
  }

  .pill:active {
    cursor: grabbing;
  }

  .pill.video {
    border-radius: var(--r-card);
  }

  .gone .pill {
    opacity: 0;
  }

  .content {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 0 var(--s3);
  }

  .round {
    position: relative;
    width: 48px;
    height: 48px;
    flex: none;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }

  .text b,
  .text span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text b {
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .text span {
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  /* Video: the picture fills the frame; the buttons sit on a dark veil at the bottom. */
  .video .content {
    padding: 0;
  }

  .picture {
    position: absolute;
    inset: 0;
  }

  .overlay {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: var(--s1);
    padding: var(--s6) var(--s2) var(--s2) var(--s3);
    background: linear-gradient(transparent, color-mix(in srgb, var(--opaque) 72%, transparent));
  }

  .overlay .text span {
    color: color-mix(in srgb, var(--ink) 72%, transparent);
    font-size: var(--fs-1);
  }

  @media (prefers-reduced-motion: reduce) {
    .pill {
      animation: fade var(--t-enter) var(--ease-out);
    }

    .docking .pill {
      opacity: 0;
    }

    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
