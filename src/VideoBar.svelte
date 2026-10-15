<!-- Commands of the small video player (docs/09-DEMO-FINALE.md, section 5 bis): a local
     window of îlot laid over the bottom edge of the player, since nothing can be added to
     the remote page itself (src-tauri/src/video.rs). The title and the source on a dark
     veil, then play-pause, « Ramener dans l'île » and « Fermer ». It fades away while the
     pointer is elsewhere, so the subtitles show, and comes back under the pointer or the
     keyboard focus. A drag anywhere off the buttons moves the player, with a little speed
     when let go; dropped near the island, or sent back (or Escape), the player melts into
     it. -->
<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./components/Icon.svelte";
  import Glyph from "./components/music/Glyph.svelte";
  import RoundButton from "./components/music/RoundButton.svelte";
  import Transport from "./components/music/Transport.svelte";
  import { type Point, type Sample, keepRecent, nearHome, throwTarget, velocityOf } from "./components/music/fling";
  import { screenBounds, windowAt } from "./components/music/mini-window";
  import { type VideoSite, closeVideo, videoBarMove } from "./lib/ipc";
  import { buddyHome } from "./lib/ipc";
  import { i18n } from "./lib/i18n/i18n.svelte";
  import { ownTrack, sourceLabel } from "./lib/media";
  import { Spring } from "./lib/spring.svelte";
  import { media } from "./lib/stores/media.svelte";

  let { site }: { site: VideoSite } = $props();

  /** Height of the player under the bar, in logical pixels (video.rs, `compact_size`). */
  const PLAYER_HEIGHT: Record<VideoSite, number> = { youtube: 200, crunchyroll: 198 };
  const SITE_NAME: Record<VideoSite, string> = { youtube: "YouTube", crunchyroll: "Crunchyroll" };
  /** The bar stays this long after the pointer leaves, then fades so the subtitles show. */
  const IDLE_MS = 2500;
  /** A press that moves less than this is a click, not a drag. */
  const DRAG_SLOP = 3;

  const track = $derived(ownTrack(media.sessions, site));

  let shown = $state(true);
  let hovered = false;
  let focused = false;
  let idle: ReturnType<typeof setTimeout> | undefined;
  let home: Point | null = null;
  let drag: { from: Point; at: Point; samples: Sample[]; moved: boolean } | null = null;

  /** Where the bar goes after a throw: springs, one per axis. */
  const x = new Spring(0);
  const y = new Spring(0);
  let flying = false;

  /** Moves the bar (its player follows), at most once a frame. */
  let wanted: Point | null = null;
  let frame = 0;
  function moveTo(to: Point): void {
    wanted = to;
    if (frame !== 0) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      const at = wanted;
      wanted = null;
      // A refused move leaves the player where it is.
      if (at) videoBarMove(Math.round(at.x), Math.round(at.y)).catch(() => {});
    });
  }

  // The window speaks the interface's language, and says so (title, screen readers).
  $effect(() => {
    document.documentElement.lang = i18n.locale;
    document.title = `${i18n.t.music.videoBar.label} · îlot`;
  });

  $effect(() => {
    const to = { x: x.value, y: y.value };
    if (flying) moveTo(to);
  });

  /** Shows the bar, and hides it again once nothing holds it. */
  function wake(): void {
    shown = true;
    clearTimeout(idle);
    idle = setTimeout(() => {
      if (!hovered && !focused && !drag) shown = false;
    }, IDLE_MS);
  }

  // A refused command shows, even with the pointer elsewhere.
  $effect(() => {
    if (media.notice) wake();
  });

  onMount(() => {
    wake();
    locateHome();
    const escape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") dock();
    };
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("keydown", escape);
      clearTimeout(idle);
    };
  });

  /** Where the island sits now. Unknown, the player still docks by its button. */
  function locateHome(): void {
    buddyHome().then(
      (point) => (home = point),
      () => (home = null),
    );
  }

  function dock(): void {
    flying = false;
    void media.dock();
  }

  /** The player's top-left corner for the bar's, in logical pixels. */
  function playerAt(bar: Point): Point {
    return { x: bar.x, y: bar.y - (PLAYER_HEIGHT[site] - window.innerHeight) };
  }

  function down(event: PointerEvent): void {
    if (event.button !== 0 || (event.target as Element).closest("button")) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    flying = false;
    const from = { x: event.screenX, y: event.screenY };
    drag = { from, at: windowAt(), samples: [{ ...from, t: event.timeStamp }], moved: false };
    locateHome();
  }

  function move(event: PointerEvent): void {
    wake();
    if (!drag) return;
    const dx = event.screenX - drag.from.x;
    const dy = event.screenY - drag.from.y;
    if (!drag.moved && Math.hypot(dx, dy) < DRAG_SLOP) return;
    drag.moved = true;
    drag.samples = keepRecent(drag.samples, { x: event.screenX, y: event.screenY, t: event.timeStamp });
    moveTo({ x: drag.at.x + dx, y: drag.at.y + dy });
  }

  function up(event: PointerEvent): void {
    if (!drag) return;
    const { from, at: start, samples, moved } = drag;
    drag = null;
    wake();
    if (!moved) return;
    const at = { x: start.x + event.screenX - from.x, y: start.y + event.screenY - from.y };
    const player = playerAt(at);
    const size = { width: window.innerWidth, height: PLAYER_HEIGHT[site] };
    if (home && nearHome({ x: player.x + size.width / 2, y: player.y + size.height / 2 }, home)) {
      dock();
      return;
    }
    // The whole player stays on screen, not only its bar.
    const speed = velocityOf(samples);
    const rest = throwTarget(player, speed, size, screenBounds());
    x.snap(at.x);
    y.snap(at.y);
    x.push(speed.x);
    y.push(speed.y);
    flying = true;
    x.set(rest.x);
    y.set(rest.y + (at.y - player.y));
  }
</script>

<main
  class="bar"
  class:shown
  aria-label={i18n.t.music.videoBar.label}
  onpointerenter={() => {
    hovered = true;
    wake();
  }}
  onpointerleave={() => {
    hovered = false;
    wake();
  }}
  onfocusin={() => {
    focused = true;
    wake();
  }}
  onfocusout={() => {
    focused = false;
    wake();
  }}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={() => (drag = null)}
  ondragstart={(event) => event.preventDefault()}
>
  <p class="text">
    <b title={track?.title}>{track?.title ?? SITE_NAME[site]}</b>
    <span class:notice={media.notice !== null} title={media.notice ?? undefined}>{media.notice ?? (track ? sourceLabel(track) : i18n.t.music.videoBar.inIlot)}</span>
  </p>
  <Transport {track} toggleOnly solid={false} />
  <RoundButton size={32} aria-label={i18n.t.music.dock} title={i18n.t.music.dock} onclick={dock}>
    <Glyph name="dock" size={13} />
  </RoundButton>
  <RoundButton size={32} aria-label={i18n.t.music.videoBar.close} title={i18n.t.music.videoBar.close} onclick={() => void closeVideo()}>
    <Icon name="close" size={12} />
  </RoundButton>
</main>

<style>
  /* The window is transparent: the video shows through, under a veil that darkens toward
     the bottom edge, where the player's own corners round off (Windows 11: 8 px). */
  .bar {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    gap: var(--s1);
    padding: var(--s6) var(--s2) var(--s2) var(--s3);
    border-radius: 0 0 var(--s2) var(--s2);
    background: linear-gradient(transparent, color-mix(in srgb, var(--opaque) 72%, transparent));
    color: var(--ink);
    cursor: grab;
    touch-action: none;
    opacity: 0;
    transition: opacity var(--t-enter) var(--ease-out);
  }

  /* Back at once under the pointer; away gently. */
  .bar.shown {
    opacity: 1;
    transition-duration: var(--t-quick);
  }

  .bar:active {
    cursor: grabbing;
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
    font-size: var(--fs-sub);
    font-weight: 600;
  }

  .text span {
    color: color-mix(in srgb, var(--ink) 72%, transparent);
    font-size: var(--fs-1);
  }

  /* A command the player refused, in words (never the color alone). */
  .text span.notice {
    color: var(--ink);
  }
</style>
