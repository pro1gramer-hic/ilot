<!-- Color mark of a media source, always next to its name (never the only clue). -->
<script lang="ts">
  import type { MediaSource } from "../lib/media";

  let { source, hollow = false }: { source: MediaSource; hollow?: boolean } = $props();
</script>

<i class="dot" class:hollow data-source={source} aria-hidden="true"></i>

<style>
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
    background: var(--ink-3);
  }

  .dot[data-source="Spotify"] {
    --src: var(--src-spotify);
  }

  .dot[data-source="YouTube"] {
    --src: var(--src-youtube);
  }

  .dot[data-source="Crunchyroll"] {
    --src: var(--src-crunchyroll);
  }

  .dot[data-source] {
    background: var(--src, var(--ink-3));
  }

  /* No session for this source: a ring instead of a dot. */
  .dot.hollow {
    background: none;
    box-shadow: inset 0 0 0 1.25px var(--src, var(--ink-3));
  }
</style>
