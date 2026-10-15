<!-- « Musique » opening of the island (500 × 96, docs/09-DEMO-FINALE.md, section 4): the
     cover, the source, the title and the artist over a thin progress line that can be
     dragged (SeekLine), then previous, play-pause, next and « Détacher » (« Ramener la
     vidéo » once a video is out). For a video, the cover is the picture Windows gives.
     Without `track`, it shows what Windows plays (nothing when nothing plays). `ondetach`
     runs after something came off (the island closes the opening); a key left to press
     shows in place of the artist. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type Track, detachLabel, isVideo, sourceLabel, subtitle } from "../../lib/media";
  import { media } from "../../lib/stores/media.svelte";
  import Artwork from "../Artwork.svelte";
  import Glyph from "./Glyph.svelte";
  import RoundButton from "./RoundButton.svelte";
  import SeekLine from "./SeekLine.svelte";
  import Transport from "./Transport.svelte";
  import { arrive } from "./arrive";
  import { OPENING_SIZE } from "./sizes";

  let { track: given, ondetach }: { track?: Track | null; ondetach?: () => void } = $props();

  const track = $derived(given === undefined ? media.current : given);
  const out = $derived(isVideo(track) && media.video.detached !== null);

  async function detach(): Promise<void> {
    if (await media.detach(track)) ondetach?.();
  }
</script>

{#if track}
  <div class="opening" style:width="{OPENING_SIZE.width}px" style:height="{OPENING_SIZE.height}px">
    <span class="cover" in:arrive={{ index: 0 }}><Artwork {track} /></span>
    {#key track.id}
      <div class="text" in:arrive={{ index: 1 }}>
        <span class="label">{sourceLabel(track)}</span>
        <b class="title" title={track.title}>{track.title}</b>
        {#if media.tip}
          <span class="artist tip" role="status">{media.tip}</span>
        {:else}
          <span class="artist">{subtitle(track)}</span>
        {/if}
        {#if track.duration <= 0}
          <span class="live">{i18n.t.music.live}</span>
        {:else}
          <SeekLine {track} now={media.now} onseek={(seconds) => media.seek(seconds, track)} />
        {/if}
      </div>
    {/key}
    <div class="actions" in:arrive={{ index: 2 }}>
      <Transport {track} />
      <RoundButton aria-label={detachLabel(track, out)} title={detachLabel(track, out)} onclick={detach}>
        <Glyph name={out ? "dock" : "detach"} />
      </RoundButton>
    </div>
  </div>
{/if}

<style>
  .opening {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: 0 var(--s4) 0 var(--s5);
    color: var(--ink);
  }

  .cover {
    position: relative;
    width: 60px;
    height: 60px;
    flex: none;
    border-radius: var(--r-ctl);
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

  .title {
    font-size: var(--fs-body);
    font-weight: 600;
    letter-spacing: -0.005em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist {
    color: var(--ink-2);
    font-size: var(--fs-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tip {
    color: var(--ink);
  }

  .live {
    margin-top: var(--s1);
    color: var(--ink-3);
    font-size: var(--fs-1);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--gap-mark);
    flex: none;
  }
</style>
