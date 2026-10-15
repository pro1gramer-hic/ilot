<!-- « Traduire » by a received message or an answer of the chat (prompt 12, DeepL). It
     never shows for a hidden text (« Masquer le contenu », « Expéditeur seul »). A click
     sends the text to DeepL through the core, with the user's key; the parent then shows
     `translations.shown(id, text)`, and the button becomes « Voir l'original » and back.
     Without a key, the button turns into « Ajouter une clé DeepL », which opens Réglages ›
     Chat: no error. A refusal (quota, network) is said beside it, and a click tries again.
     `reveal`: in a list, it shows when its row (`.translate-host`) is hovered or focused,
     and stays once used, like « Copier » under an answer. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import { failureText, languageName, translatable } from "../../lib/translate";

  let { id, text, hidden = false, reveal = false }: { id: string; text: string; hidden?: boolean; reveal?: boolean } = $props();

  const t = $derived(i18n.t.translate);
  const current = $derived(translations.state(id));
  const label = $derived.by(() => {
    switch (current?.status) {
      case "pending":
        return t.pending;
      case "done":
        return current.original ? t.showTranslation : t.showOriginal;
      case "offer":
        return t.addKey;
      case "error":
        return t.retry;
      default:
        return t.button;
    }
  });
  /** Read by screen readers when it changes: the language found, or the refusal. */
  const note = $derived.by(() => {
    if (current?.status === "error") return failureText(current.failure);
    if (current?.status === "done" && !current.original && current.detected) return t.detected(languageName(current.detected, i18n.locale));
    return "";
  });
</script>

{#if translatable(text, hidden)}
  <span class="translate" class:reveal={reveal && !current}>
    <button
      type="button"
      class:offer={current?.status === "offer"}
      aria-busy={current?.status === "pending"}
      disabled={current?.status === "pending"}
      title={current?.status === "offer" ? t.addKeyHint : undefined}
      onclick={() => void translations.press(id, text, hidden)}
    >
      {label}
    </button>
    <span class="note" class:visually-hidden={current?.status !== "error"} role="status">{note}</span>
  </span>
{/if}

<style>
  .translate {
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
  }

  button {
    flex: none;
    display: inline-flex;
    align-items: center;
    border-radius: var(--r-chip);
    color: var(--ink-2);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color var(--t-quick),
      color var(--t-quick);
  }

  button:hover:not(:disabled) {
    background: var(--hover);
    color: var(--ink);
  }

  button:disabled {
    color: var(--ink-3);
    cursor: progress;
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* Without a key: the one place the accent shows, it is the next step. */
  button.offer {
    color: var(--accent-text);
  }

  /* In a line of text: small, it does not compete with the words. */
  button {
    height: 20px;
    padding: 0 var(--s2);
    font-size: var(--fs-1);
  }

  .reveal {
    opacity: 0;
    transition: opacity var(--t-quick);
  }

  :global(.translate-host:hover) .reveal,
  .reveal:focus-within {
    opacity: 1;
  }

  @media (hover: none) {
    .reveal {
      opacity: 1;
    }
  }

  .note {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--err);
    font-size: var(--fs-1);
  }
</style>
