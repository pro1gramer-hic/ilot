<!-- « Nouveau morceau » opening of the island (420 × 56, docs/09-DEMO-FINALE.md, section 4):
     the round cover, the title and the artist on one line, and a small record turning at
     the end. The island shows it for 4 s when the track changes and the player is closed.
     Without `track`, it shows what Windows plays. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type Track, subtitle } from "../../lib/media";
  import { media } from "../../lib/stores/media.svelte";
  import Artwork from "../Artwork.svelte";
  import Vinyl from "../Vinyl.svelte";
  import { arrive } from "./arrive";
  import { NEW_TRACK_SIZE } from "./sizes";

  let { track: given }: { track?: Track | null } = $props();

  const track = $derived(given === undefined ? media.current : given);
</script>

{#if track}
  <div class="new" style:width="{NEW_TRACK_SIZE.width}px" style:height="{NEW_TRACK_SIZE.height}px" role="status">
    <span class="cover" in:arrive={{ index: 0 }}><Artwork {track} /></span>
    {#key track.id}
      <p class="text" in:arrive={{ index: 1 }}>
        <span class="label">{i18n.t.music.newTrack}</span>
        <span class="line"><b>{track.title}</b>, <span class="artist">{subtitle(track)}</span></span>
      </p>
    {/key}
    <span class="round" in:arrive={{ index: 2 }}><Vinyl size={28} artwork={track.artwork} playing={track.playing} /></span>
  </div>
{/if}

<style>
  .new {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 0 var(--s4) 0 var(--s2);
    color: var(--ink);
  }

  .cover {
    position: relative;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 50%;
    overflow: hidden;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }

  .label {
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .line {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-3);
  }

  b {
    font-weight: 600;
  }

  .artist {
    color: var(--ink-2);
    font-weight: 500;
  }

  .round {
    flex: none;
  }
</style>
