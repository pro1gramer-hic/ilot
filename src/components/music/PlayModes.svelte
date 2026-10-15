<!-- Shuffle and repeat of the shown session (prompt 12, lot 4), through Windows' media
     controls. Each button shows only when the session lets others change it (SMTC
     `IsShuffleEnabled`, `IsRepeatEnabled`): nothing says Spotify does, so the row may stay
     empty. On, a button turns to the readable accent with a small dot under it; repeat
     goes off, the list, the title, as in Spotify. A refusal comes back as the player's
     notice and the button returns to what it was. Line signs, 16 px grid. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type Track, nextRepeat, repeatLabel } from "../../lib/media";
  import { media } from "../../lib/stores/media.svelte";
  import RoundButton from "./RoundButton.svelte";

  let { track, size = 36 }: { track: Track | null; size?: number } = $props();

  const shuffleOn = $derived(track?.shuffle === true);
  const repeat = $derived(track?.repeat ?? "none");
</script>

{#if track && (track.controls.shuffle || track.controls.repeat)}
  <div class="modes" role="group" aria-label={i18n.t.music.modes.group}>
    {#if track.controls.shuffle}
      <span class="mode" class:on={shuffleOn}>
        <RoundButton
          {size}
          aria-pressed={shuffleOn}
          aria-label={i18n.t.music.modes.shuffle}
          title={shuffleOn ? i18n.t.music.modes.shuffleOn : i18n.t.music.modes.shuffleOff}
          onclick={() => media.setMode({ shuffle: !shuffleOn }, track)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2 4.5h2.2c3.1 0 4.5 7 7.6 7H14M12 9.5l2 2-2 2M2 11.5h2.2c1.2 0 2-1 2.7-2.4M9.1 6.9c.7-1.4 1.5-2.4 2.7-2.4H14M12 2.5l2 2-2 2" />
          </svg>
        </RoundButton>
      </span>
    {/if}
    {#if track.controls.repeat}
      <span class="mode" class:on={repeat !== "none"}>
        <RoundButton
          {size}
          aria-pressed={repeat !== "none"}
          aria-label={i18n.t.music.modes.repeat(repeatLabel(repeat))}
          title={i18n.t.music.modes.repeatTitle(repeatLabel(repeat), repeatLabel(nextRepeat(repeat)))}
          onclick={() => media.setMode({ repeat: nextRepeat(repeat) }, track)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2.5 7.5v-1a2 2 0 0 1 2-2h9M11.5 2.5l2 2-2 2M13.5 8.5v1a2 2 0 0 1-2 2h-9M4.5 13.5l-2-2 2-2" />
            {#if repeat === "track"}<path class="one" d="M7.2 6.9 8.3 6.2v3.6" />{/if}
          </svg>
        </RoundButton>
      </span>
    {/if}
  </div>
{/if}

<style>
  .modes {
    display: flex;
    align-items: center;
    gap: var(--gap-mark);
  }

  .mode {
    position: relative;
    display: grid;
    color: var(--ink-2);
    transition: color var(--t-quick);
  }

  .mode :global(button) {
    color: inherit;
  }

  .mode.on {
    color: var(--accent-text);
  }

  /* On: a dot under the sign, so the state never rests on the color alone. */
  .mode::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: 3px;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0;
    transform: scale(0.4);
    transition:
      opacity var(--t-quick),
      transform var(--t-quick) var(--ease-out);
    pointer-events: none;
  }

  .mode.on::after {
    opacity: 1;
    transform: none;
  }

  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .one {
    stroke-width: 1.3;
  }

  @media (prefers-reduced-motion: reduce) {
    .mode::after {
      transform: none;
      transition: opacity var(--t-quick);
    }
  }
</style>
