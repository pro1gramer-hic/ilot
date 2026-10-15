<!-- Cover of a session as a small thumbnail, cropped to its frame; a soft gradient when the
     session gives no cover. -->
<script lang="ts">
  import type { Track } from "../lib/media";

  let { track }: { track: Track | null } = $props();
</script>

<div class="artwork">
  {#if track?.artwork}
    <img src={track.artwork} alt="" />
  {/if}
</div>

<style>
  .artwork {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background:
      radial-gradient(circle at 18% 12%, color-mix(in srgb, var(--accent-2) 55%, transparent), transparent 55%),
      radial-gradient(circle at 85% 95%, color-mix(in srgb, var(--accent) 70%, transparent), transparent 60%),
      var(--card-2);
    border-radius: inherit;
  }

  img {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* A 1 px edge over the picture, so a dark cover keeps its shape on a dark card. */
  .artwork::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px var(--line);
  }
</style>
