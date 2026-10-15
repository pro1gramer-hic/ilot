<!-- Musique tab of the panel (docs/09-DEMO-FINALE.md, sections 5 bis and 8): the deck with
     its record and tonearm, or, for a video, the picture Windows gives in 16:9 (never an
     image copied from a protected video); the source (Spotify, YouTube, Crunchyroll); the
     title; the wave, which is the seek bar; the commands (with shuffle and repeat when the
     session allows them) and the volume, plus Spotify's own volume; what played
     before; « Détacher le lecteur », or for a video « Détacher la vidéo » (a real player
     that has the right to show it, src-tauri/src/video.rs) and « Ramener la vidéo » once it
     is out. The Crunchyroll tab opens îlot's own Crunchyroll window. `ondetach` runs once
     something came off (the panel closes); a key left to press keeps the panel open. -->
<script lang="ts">
  import {
    MEDIA_SOURCES,
    type TabSource,
    type Track,
    detachLabel,
    emptyState,
    formatTime,
    isVideo,
    playedBefore,
    progress,
    sessionOf,
    sourceLabel,
    subtitle,
  } from "../../lib/media";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { media } from "../../lib/stores/media.svelte";
  import Artwork from "../Artwork.svelte";
  import SourceDot from "../SourceDot.svelte";
  import SourceTabs from "../SourceTabs.svelte";
  import Vinyl from "../Vinyl.svelte";
  import AppVolume from "../music/AppVolume.svelte";
  import Glyph from "../music/Glyph.svelte";
  import PlayModes from "../music/PlayModes.svelte";
  import Transport from "../music/Transport.svelte";
  import Volume from "../music/Volume.svelte";
  import YoutubeLink from "../music/YoutubeLink.svelte";
  import { arrive } from "../music/arrive";
  import Turntable from "../player/Turntable.svelte";
  import Wave from "../player/Wave.svelte";
  import Button from "../settings/Button.svelte";

  let { ondetach }: { ondetach?: () => void } = $props();

  const t = $derived(i18n.t.panel.music);
  const track = $derived(media.shown);
  const video = $derived(isVideo(track) || (track === null && media.tab !== null && media.tab !== "Spotify"));
  const empty = $derived(emptyState(media.tab, true));
  const before = $derived(playedBefore(media.recent, track));
  const out = $derived(isVideo(track) && media.video.detached !== null);
  const via = $derived(media.video.extension ? t.viaExtension : "");

  /** The source tab that shows this media again, while its session is still open. */
  function tabShowing(item: Track): TabSource | null {
    const tab = MEDIA_SOURCES.find((s) => s === item.source);
    return tab && sessionOf(media.sessions, tab)?.id === item.id ? tab : null;
  }

  /** « Kairo, Spotify », or the source alone when the session names no artist. */
  function origin(item: Track): string {
    const artist = item.artist.trim();
    return artist ? `${artist}, ${sourceLabel(item)}` : sourceLabel(item);
  }

  async function detach(): Promise<void> {
    if (await media.detach(track)) ondetach?.();
  }
</script>

{#snippet row(item: Track)}
  <span class="thumb" class:wide={isVideo(item)}>
    {#if isVideo(item)}<Artwork track={item} />{:else}<Vinyl size={28} artwork={item.artwork} />{/if}
  </span>
  <span class="text">
    <b>{item.title}</b>
    <span class="meta"><SourceDot source={item.source} /><span class="where">{origin(item)}</span></span>
  </span>
  <time>{item.duration > 0 ? formatTime(item.duration) : t.live}</time>
{/snippet}

<section class="music" class:video aria-label={i18n.t.panel.tabs.music}>
  <div class="media" in:arrive={{ index: 0 }}>
    {#if video}
      <figure class="frame">
        {#if track}<Artwork {track} />{/if}
        <figcaption class="visually-hidden">{track ? t.picture(track.title) : empty.title}</figcaption>
      </figure>
      {#if media.tab === "YouTube"}
        <YoutubeLink />
      {/if}
      {#if media.tab === "Crunchyroll"}
        <div class="watch">
          <p class="note">{t.crunchyroll(via)}</p>
          <Button onclick={() => media.openCrunchyroll()}>{t.watchInIlot}</Button>
        </div>
      {/if}
    {:else}
      <div class="plate">
        <div class="table">
          <Turntable loaded={track !== null} artwork={track?.artwork} playing={track?.playing ?? false} played={track ? progress(track, media.now) : 0} />
        </div>
        <span class="rpm">{t.rpm}</span>
      </div>
    {/if}
  </div>

  <div class="side">
    <div class="sources" in:arrive={{ index: 1 }}><SourceTabs /></div>

    {#key track?.id}
      <div class="heading" in:arrive={{ index: 2 }}>
        {#if track}
          <span class="label">{sourceLabel(track)}</span>
          <h3 title={track.title}>{track.title}</h3>
          {#if media.notice}
            <p class="sub notice" role="alert">{media.notice}</p>
          {:else if media.tip}
            <p class="sub tip" role="status">{media.tip}</p>
          {:else}
            <p class="sub">{subtitle(track)}</p>
          {/if}
        {:else if media.error}
          <h3>{t.unavailable}</h3>
          <p class="sub notice" role="alert">{media.error}</p>
        {:else}
          <h3>{empty.title}</h3>
          <p class="sub hint">{empty.hint}</p>
        {/if}
      </div>
    {/key}

    <div class="wave" in:arrive={{ index: 3 }}>
      <Wave {track} now={media.now} bars={64} height={44} dense onseek={(seconds) => media.seek(seconds, track)} />
    </div>

    <div class="controls" in:arrive={{ index: 4 }}>
      <span class="play">
        <Transport {track} size="lg" />
        <PlayModes {track} size={38} />
      </span>
      <Volume />
    </div>
    {#if track?.source === "Spotify"}
      <div class="app-volume"><AppVolume /></div>
    {/if}

    <div class="before" in:arrive={{ index: 5 }}>
      <div class="before-head">
        <h4 class="label" id="played-before">{t.playedBefore}</h4>
        <Button disabled={track === null} onclick={detach}><Glyph name={out ? "dock" : "detach"} size={12} />{detachLabel(track, out)}</Button>
      </div>
      {#if before.length > 0}
        <ul aria-labelledby="played-before">
          {#each before as item (item.id)}
            {@const tab = tabShowing(item)}
            {@const videoId = item.videoId}
            <li>
              {#if tab}
                <button type="button" class="row" aria-label={t.show(item.title)} onclick={() => media.select(tab)}>{@render row(item)}</button>
              {:else if videoId}
                <button type="button" class="row" aria-label={t.play(item.title)} onclick={() => media.playVideo(videoId)}>{@render row(item)}</button>
              {:else}
                <div class="row">{@render row(item)}</div>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="empty">{t.noHistory}</p>
      {/if}
    </div>
  </div>
</section>

<style>
  .music {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    gap: 28px;
    align-items: start;
    min-width: 0;
  }

  .music.video {
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
  }

  .media {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-width: 0;
  }

  /* A dark plate under the turntable, lit from above. */
  .plate {
    position: relative;
    width: 260px;
    height: 260px;
    border-radius: var(--r-open);
    border: 1px solid var(--line);
    background: linear-gradient(180deg, var(--surface-2), var(--panel));
    box-shadow: var(--hi);
  }

  .table {
    position: absolute;
    left: 22px;
    top: 22px;
    height: 186px;
  }

  .rpm {
    position: absolute;
    left: var(--s4);
    bottom: var(--s3);
    color: var(--ink-3);
    font-family: var(--font-mono);
    font-size: var(--fs-1);
  }

  .frame {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: var(--r-card);
    border: 1px solid var(--line);
    background: var(--surface);
    overflow: hidden;
  }

  .watch {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--s2);
  }

  .note {
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  .side {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .sources {
    width: 300px;
    max-width: 100%;
  }

  .heading {
    margin-top: var(--s4);
    min-width: 0;
  }

  .label {
    display: block;
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  h3 {
    margin-top: 2px;
    font-size: calc(var(--fs-display) - var(--s1));
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.15;
    text-wrap: balance;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }

  .sub {
    margin-top: var(--s1);
    color: var(--ink-2);
    font-size: var(--fs-4);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub.hint {
    white-space: normal;
    font-size: var(--fs-3);
  }

  .notice {
    color: var(--err);
    font-size: var(--fs-3);
    white-space: normal;
  }

  /* A key left to press: plain ink, it is no error. */
  .tip {
    color: var(--ink);
    font-size: var(--fs-3);
    white-space: normal;
  }

  .wave {
    margin-top: var(--s4);
  }

  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    margin-top: var(--s3);
  }

  /* Transport, then shuffle and repeat when the session allows them. */
  .play {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  /* Spotify's own volume, under the PC's. */
  .app-volume {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--s1);
  }

  .before {
    margin-top: var(--s5);
  }

  .before-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2) var(--s3);
    margin-bottom: var(--s3);
  }

  h4 {
    font-weight: 500;
    white-space: nowrap;
  }

  ul {
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    overflow: hidden;
  }

  li + li {
    border-top: 1px solid var(--line);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s3);
    width: 100%;
    padding: 11px var(--s4);
    text-align: left;
    font-size: var(--fs-3);
  }

  button.row {
    transition: background var(--t-quick);
  }

  button.row:hover {
    background: var(--surface);
  }

  button.row:focus-visible {
    outline-offset: -2px;
  }

  .thumb {
    position: relative;
    flex: none;
    width: 28px;
    height: 28px;
  }

  .thumb.wide {
    width: 50px;
    border-radius: var(--r-chip);
    overflow: hidden;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.35;
  }

  .text b {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: var(--s1);
    min-width: 0;
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .where {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  time {
    flex: none;
    color: var(--ink-3);
    font-family: var(--font-mono);
    font-size: var(--fs-2);
  }

  .empty {
    padding: var(--s4);
    border: 1px dashed var(--line-2);
    border-radius: var(--r-card);
    color: var(--ink-3);
    font-size: var(--fs-3);
  }

  @media (prefers-reduced-motion: reduce) {
    button.row {
      transition: none;
    }
  }
</style>
