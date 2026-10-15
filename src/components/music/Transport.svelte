<!-- Previous / play-pause / next of the music surfaces, round, the play button filled in
     ink. `size` sets the buttons (36 px in the island, 38 and 48 in the Musique tab).
     `toggleOnly` keeps play-pause alone and `solid={false}` leaves it unfilled (the
     mini-player, where it sits beside the other round buttons). A command the app does
     not offer is shown disabled, so the buttons never jump around. The play and pause
     signs cross-fade. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import type { Track } from "../../lib/media";
  import { media } from "../../lib/stores/media.svelte";
  import Icon from "../Icon.svelte";
  import RoundButton from "./RoundButton.svelte";

  let {
    track,
    size = "md",
    toggleOnly = false,
    solid = true,
  }: { track: Track | null; size?: "md" | "lg"; toggleOnly?: boolean; solid?: boolean } = $props();

  const SIZES = { md: { side: 36, play: 36, sign: 13 }, lg: { side: 38, play: 48, sign: 16 } } as const;

  const controls = $derived(track?.controls ?? { playPause: false, next: false, previous: false });
  const playing = $derived(track?.playing ?? false);
  const px = $derived(SIZES[size]);
</script>

<div class="transport {size}" role="group" aria-label={i18n.t.music.transport.group}>
  {#if !toggleOnly}
    <RoundButton
      size={px.side}
      aria-label={i18n.t.music.transport.previous}
      disabled={!controls.previous}
      onclick={() => media.command("previous", track)}
    >
      <Icon name="previous" size={13} />
    </RoundButton>
  {/if}
  <RoundButton
    {solid}
    size={px.play}
    aria-label={playing ? i18n.t.music.transport.pause : i18n.t.music.transport.play}
    disabled={!controls.playPause}
    onclick={() => media.command(playing ? "pause" : "play", track)}
  >
    <span class="sign" class:on={!playing}><Icon name="play" size={px.sign} /></span>
    <span class="sign" class:on={playing}><Icon name="pause" size={px.sign} /></span>
  </RoundButton>
  {#if !toggleOnly}
    <RoundButton size={px.side} aria-label={i18n.t.music.transport.next} disabled={!controls.next} onclick={() => media.command("next", track)}>
      <Icon name="next" size={13} />
    </RoundButton>
  {/if}
</div>

<style>
  .transport {
    display: flex;
    align-items: center;
    gap: var(--gap-mark);
    flex: none;
  }

  .lg {
    gap: var(--s1);
  }

  .sign {
    grid-area: 1 / 1;
    opacity: 0;
    transform: scale(0.6);
    transition:
      opacity var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .sign.on {
    opacity: 1;
    transform: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .sign {
      transform: none;
      transition: opacity var(--t-quick);
    }
  }
</style>
