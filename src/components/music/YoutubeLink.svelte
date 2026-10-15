<!-- A YouTube link to play in îlot's isolated video window. Only the video id leaves the
     field (youtube.ts); the Rust core checks it again and builds the embed itself. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { media } from "../../lib/stores/media.svelte";
  import { youtubeStart, youtubeVideoId } from "../../lib/youtube";

  let link = $state("");
  /** The last link given leads to no YouTube video. */
  let invalid = $state(false);

  function play(event: SubmitEvent): void {
    event.preventDefault();
    const id = youtubeVideoId(link);
    if (!id) {
      invalid = true;
      return;
    }
    invalid = false;
    void media.playVideo(id, youtubeStart(link));
    link = "";
  }
</script>

<form class="link" onsubmit={play}>
  <label class="visually-hidden" for="youtube-link">{i18n.t.music.youtube.label}</label>
  <input
    id="youtube-link"
    type="url"
    inputmode="url"
    bind:value={link}
    placeholder={i18n.t.music.youtube.placeholder}
    autocomplete="off"
    spellcheck="false"
    aria-invalid={invalid}
    aria-describedby={invalid ? "youtube-link-error" : undefined}
    oninput={() => (invalid = false)}
  />
  <button type="submit" disabled={link.trim() === ""}>{i18n.t.music.youtube.play}</button>
</form>
{#if invalid}
  <p id="youtube-link-error" class="error" role="alert">{i18n.t.music.youtube.invalid}</p>
{/if}

<style>
  .link {
    display: flex;
    gap: var(--s2);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 var(--s3);
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--fs-3);
    user-select: text;
    transition: border-color var(--t-quick);
  }

  input::placeholder {
    color: var(--ink-3);
  }

  input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  input[aria-invalid="true"] {
    border-color: var(--err);
  }

  button {
    height: 36px;
    padding: 0 var(--s4);
    border-radius: var(--r-ctl);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--fs-3);
    font-weight: 600;
    box-shadow: var(--btn-shadow);
    transition:
      transform var(--t-quick) var(--ease-out),
      filter var(--t-quick);
  }

  button:hover:enabled {
    filter: brightness(1.08);
  }

  button:active:enabled {
    transform: scale(var(--press));
  }

  button:disabled {
    background: var(--surface-2);
    color: var(--ink-3);
    box-shadow: none;
    cursor: default;
  }

  .error {
    margin-top: var(--s2);
    color: var(--err);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    button:active:enabled {
      transform: none;
    }
  }
</style>
