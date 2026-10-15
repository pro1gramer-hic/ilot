<!-- Volume of Spotify alone (prompt 12, lot 4), under the PC's: the level of Spotify's
     audio session, the one Windows' volume mixer sets (src-tauri/src/app_volume.rs). Read
     when it appears; while it is dragged, one request at a time, the last level wins.
     Hidden while Spotify plays no sound (Windows has no session for it then). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { appVolume, setAppVolume } from "../../lib/ipc";

  /** Level shown, in percent; null while Spotify has no audio session. */
  let level = $state<number | null>(null);
  let wanted: number | null = null;
  let busy = false;

  $effect(() => {
    void appVolume("spotify")
      .then((value) => (level = value === null ? null : Math.round(value * 100)))
      .catch(() => (level = null));
  });

  async function follow(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      while (wanted !== null) {
        const next = wanted;
        wanted = null;
        await setAppVolume("spotify", next / 100);
      }
    } catch {
      // Spotify stopped meanwhile: the slider goes away until it plays again.
      level = null;
    } finally {
      busy = false;
    }
  }

  function input(event: Event & { currentTarget: HTMLInputElement }): void {
    level = Number(event.currentTarget.value);
    wanted = level;
    void follow();
  }
</script>

{#if level !== null}
  <label class="volume" title={i18n.t.music.volume.appTitle("Spotify")}>
    <span class="name">Spotify</span>
    <input
      type="range"
      min="0"
      max="100"
      step="1"
      value={level}
      style:--v="{level}%"
      aria-label={i18n.t.music.volume.app("Spotify")}
      aria-valuetext={i18n.t.music.volume.level(level)}
      oninput={input}
    />
  </label>
{/if}

<style>
  /* The same track as the PC's volume above it (Volume.svelte: 140 px minus its icon), so
     that the two line up on the right. */
  .volume {
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .name {
    flex: none;
  }

  input {
    flex: none;
    width: calc(140px - 14px - var(--s2));
    height: 20px;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--r-pill);
    background: linear-gradient(90deg, var(--ink) var(--v), var(--line-2) var(--v));
  }

  input::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: var(--ink);
    box-shadow: var(--shadow-soft);
    transition: transform var(--t-quick) var(--ease-out);
  }

  input:hover::-webkit-slider-thumb {
    transform: scale(1.15);
  }

  input:focus-visible {
    outline: none;
  }

  input:focus-visible::-webkit-slider-thumb {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    input::-webkit-slider-thumb {
      transition: none;
    }

    input:hover::-webkit-slider-thumb {
      transform: none;
    }
  }
</style>
