<!-- Spotify / YouTube / Crunchyroll as a segmented control: shows the session of a source,
     or its empty state. A thumb slides under the chosen segment on a spring (it jumps with
     reduced motion). A full dot means the source has a session open, a ring means none. -->
<script lang="ts">
  import { untrack } from "svelte";
  import { Spring } from "svelte/motion";
  import { i18n } from "../lib/i18n/i18n.svelte";
  import { MEDIA_SOURCES, sessionOf, tabStatus } from "../lib/media";
  import { SPRINGS, springTo } from "../lib/motion.svelte";
  import { media } from "../lib/stores/media.svelte";
  import SourceDot from "./SourceDot.svelte";

  const index = $derived(MEDIA_SOURCES.findIndex((source) => source === media.tab));
  const slide = new Spring(0, SPRINGS.indicator);
  /** Hidden while no segment is chosen: it then appears in place instead of sliding. */
  let shown = $state(false);

  $effect(() => {
    if (index < 0) {
      shown = false;
      return;
    }
    const appearing = untrack(() => !shown);
    void slide.set(index, appearing ? { instant: true } : springTo());
    shown = true;
  });
</script>

<div class="segments" role="group" aria-label={i18n.t.island.sources}>
  <span class="thumb" class:shown style:transform="translateX({(slide.current * 100).toFixed(2)}%)" aria-hidden="true"
  ></span>
  {#each MEDIA_SOURCES as source (source)}
    <button
      type="button"
      class:on={media.tab === source}
      aria-pressed={media.tab === source}
      aria-label={tabStatus(media.sessions, source)}
      title={tabStatus(media.sessions, source)}
      onclick={() => media.select(source)}
    >
      <span class="face"
        ><SourceDot {source} hollow={!sessionOf(media.sessions, source)} /><span class="label">{source}</span></span
      >
    </button>
  {/each}
</div>

<style>
  /* Buttons fill the whole height (32 px targets); the frame and the thumb are drawn
     inside them. */
  .segments {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    min-width: 0;
    height: 32px;
    border-radius: var(--r-ctl);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .thumb {
    position: absolute;
    left: 0;
    top: 0;
    width: calc(100% / 3);
    height: 100%;
    padding: 3px;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--t-quick);
  }

  .thumb.shown {
    opacity: 1;
  }

  .thumb::before {
    content: "";
    display: block;
    height: 100%;
    border-radius: var(--r-chip);
    background: var(--selected);
    box-shadow:
      var(--hi),
      0 0 0 1px var(--line-2);
  }

  button {
    position: relative;
    min-width: 0;
    height: 100%;
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    font-size: var(--fs-1);
    font-weight: 500;
  }

  .face {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s1);
    min-width: 0;
    padding: 0 var(--s1);
    transition: transform var(--t-quick) var(--ease-out);
  }

  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button:hover,
  button.on {
    color: var(--ink);
  }

  button:active .face {
    transform: scale(0.96);
  }

  button:focus-visible {
    outline-offset: -2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .face,
    button:active .face {
      transition: none;
      transform: none;
    }
  }
</style>
