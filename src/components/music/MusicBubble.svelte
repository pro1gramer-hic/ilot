<!-- Content of the music bubble stuck to the folded island while something plays (docs/
     09-DEMO-FINALE.md, section 3). The island draws the drop and its liquid edge around it
     at `BUBBLE_SIZES[mode]` (sizes.ts); this is what sits inside, after the « Musique » mode:
     `titre` a turning record, the title scrolling by and the equalizer; `disque` the record
     alone; `egaliseur` the equalizer alone; `aucun` nothing. A video shows the round picture
     Windows gives instead of a record. Without props it follows the « Musique » mode of
     the settings and what Windows plays. Given `onopen` or `ondetach`, it is a button of
     its own: a click opens the player, a pull of a few pixels detaches it; without them it
     is only a picture with its name, for the island's own control around it. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type Track, isVideo, subtitle } from "../../lib/media";
  import { stillness } from "../../lib/motion.svelte";
  import { media } from "../../lib/stores/media.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Vinyl from "../Vinyl.svelte";
  import Equalizer from "./Equalizer.svelte";
  import { arrive, depart } from "./arrive";
  import { BUBBLE_SIZES, type MusicMode } from "./sizes";

  let {
    mode: givenMode,
    track: givenTrack,
    onopen,
    ondetach,
  }: { mode?: MusicMode; track?: Track | null; onopen?: () => void; ondetach?: () => void } = $props();

  const mode = $derived(givenMode ?? settings.value.modes.music);
  const track = $derived(givenTrack === undefined ? media.current : givenTrack);
  const interactive = $derived(onopen !== undefined || ondetach !== undefined);

  /** Pull, in pixels, past which the bubble comes off as a mini-player. */
  const DETACH_PULL = 22;

  const label = $derived(track ? `${track.title}, ${subtitle(track)}` : "");
  const picture = $derived(track && isVideo(track) && track.artwork ? track.artwork : null);
  const playing = $derived(track?.playing ?? false);

  let pull: { x: number; y: number } | null = null;
  let pulled = false;

  // The cover inside is an image: without `ondragstart`, the browser would drag it away
  // and cancel the pull.
  function down(event: PointerEvent): void {
    if (event.button !== 0) return;
    pull = { x: event.clientX, y: event.clientY };
    pulled = false;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent): void {
    if (!pull || pulled) return;
    if (Math.hypot(event.clientX - pull.x, event.clientY - pull.y) > DETACH_PULL) {
      pulled = true;
      pull = null;
      ondetach?.();
    }
  }

  function up(): void {
    pull = null;
  }

  function click(): void {
    if (pulled) {
      pulled = false;
      return;
    }
    onopen?.();
  }
</script>

{#snippet inside(mode: MusicMode, track: Track)}
  {#key mode}
    <span class="content" in:arrive out:depart>
      {#if mode !== "egaliseur"}
        <span class="round">
          {#if picture}
            <img src={picture} alt="" />
          {:else}
            <Vinyl fill artwork={track.artwork} {playing} />
          {/if}
        </span>
      {/if}
      {#if mode === "titre"}
        {#key label}
          <span
            class="title"
            class:rolling={playing && !stillness.current}
            style:--roll="{Math.max(6, label.length * 0.24).toFixed(1)}s"
            in:arrive
          >
            <span class="roll">{label}<span class="twin" aria-hidden="true"><span class="gap"></span>{label}<span class="gap"></span></span></span>
          </span>
        {/key}
      {/if}
      {#if mode !== "disque"}
        <Equalizer {playing} source={track.source} thin={mode === "titre"} />
      {/if}
    </span>
  {/key}
{/snippet}

{#if mode !== "aucun" && track}
  {#if interactive}
    <button
      type="button"
      class="bubble {mode}"
      style:width="{BUBBLE_SIZES[mode].width}px"
      style:height="{BUBBLE_SIZES[mode].height}px"
      aria-label={i18n.t.music.bubble.open(label)}
      title={i18n.t.music.bubble.hint}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      ondragstart={(event) => event.preventDefault()}
      onclick={click}
    >
      {@render inside(mode, track)}
    </button>
  {:else}
    <span
      class="bubble {mode}"
      role="img"
      style:width="{BUBBLE_SIZES[mode].width}px"
      style:height="{BUBBLE_SIZES[mode].height}px"
      aria-label={i18n.t.music.bubble.playing(label)}
    >
      {@render inside(mode, track)}
    </span>
  {/if}
{/if}

<style>
  /* One cell no wider than the bubble: a long title scrolls inside, never widens it. */
  .bubble {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    place-items: center;
    border-radius: var(--r-pill);
    color: var(--ink);
    cursor: grab;
    touch-action: none;
  }

  button.bubble:active {
    cursor: grabbing;
  }

  span.bubble {
    cursor: inherit;
  }

  .content {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s2);
    width: 100%;
    height: 100%;
  }

  .titre .content {
    justify-content: flex-start;
    padding: 0 var(--s3) 0 var(--s2);
  }

  .round {
    position: relative;
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 50%;
    overflow: hidden;
  }

  .titre .round {
    width: 26px;
    height: 26px;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* The title scrolls on a loop, fading at both ends; written twice so the loop is seamless. */
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: var(--fs-2);
    font-weight: 500;
    white-space: nowrap;
    mask-image: linear-gradient(90deg, transparent, var(--opaque) 10%, var(--opaque) 88%, transparent);
  }

  .roll {
    display: inline-block;
  }

  /* Speed stays the same whatever the length of the title. */
  .rolling .roll {
    animation: roll var(--roll) linear infinite;
  }

  .gap {
    display: inline-block;
    width: var(--s8);
  }

  @keyframes roll {
    to {
      transform: translateX(-50%);
    }
  }

  /* Standing still: the title once, cut with an ellipsis. */
  @media (prefers-reduced-motion: reduce) {
    .roll {
      display: inline;
    }

    .rolling .roll {
      animation: none;
    }

    .twin {
      display: none;
    }

    .title {
      text-overflow: ellipsis;
    }
  }
</style>
