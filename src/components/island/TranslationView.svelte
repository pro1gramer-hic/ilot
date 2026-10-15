<!-- « Traduire le presse-papiers » (prompt 12, DeepL; 500 × auto, laid out like a message).
     First what the core just read from the clipboard, exactly as it would leave, with
     « Traduire » and « Annuler »: nothing reaches DeepL before that click. Then the
     translation, with « Copier » and « Voir l'original ». Without a key the clipboard is not
     even read: the island offers to add one. Échap closes, and the text leaves memory with
     the opening. Mochi sits in the slot on the right. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { rise } from "../../lib/motion.svelte";
  import { banners } from "../../lib/stores/banners.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import { failureOf, failureText, languageName } from "../../lib/translate";
  import PanelIcon from "../panel/PanelIcon.svelte";
  import { swap } from "../translate/swap";

  /** How long « Copié » stays on the button. */
  const COPIED_MS = 1600;

  const t = $derived(i18n.t.translate.clipboard);
  const ask = $derived(translations.clipboard);
  /** The language chosen in Réglages by its full name, the interface's by its plain one. */
  const into = $derived(
    ask ? t.into(languageName(settings.value.translate.target ? ask.target : i18n.locale.toUpperCase(), i18n.locale)) : "",
  );
  const count = $derived(ask ? [...ask.text].length : 0);
  const caps = $derived(
    ask?.status === "done" && ask.detected ? `${t.title}, ${languageName(ask.detected, i18n.locale)}` : t.title,
  );
  const shown = $derived(ask?.status === "done" && !ask.original ? ask.translation : (ask?.text ?? ""));
  const problem = $derived.by(() => {
    if (!ask) return "";
    if (ask.status === "noKey") return t.noKey;
    if (ask.status === "unreadable") return ask.failure === "tooLong" ? t.tooLong : ask.failure === "busy" ? t.busy : t.empty;
    if (ask.status === "error") return failureText(failureOf(ask.failure));
    return "";
  });

  let copied = $state(false);
  let first = $state<HTMLButtonElement>();

  // The shortcut gave îlot the keyboard: the next step is one Enter away.
  $effect(() => {
    if (first) requestAnimationFrame(() => first?.focus({ preventScroll: true }));
  });

  // An opening whose text already went (closed, replaced) has nothing to show: it leaves.
  $effect(() => {
    if (!translations.clipboard && banners.current?.kind === "translation") banners.queue.dismissCurrent();
  });

  // The opening went away (time out, a click elsewhere): its text leaves memory, unless
  // the same opening came straight back with the next step.
  $effect(() => {
    const mine = translations.clipboard;
    return () =>
      queueMicrotask(() => {
        if (translations.clipboard === mine && banners.current?.kind !== "translation") translations.forgetClipboard();
      });
  });

  async function copy(): Promise<void> {
    await translations.copyClipboard();
    copied = true;
    setTimeout(() => (copied = false), COPIED_MS);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      translations.closeClipboard();
    }
  }
</script>

{#if ask}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="clipboard" {onkeydown}>
    <div class="row">
      <span class="tile" aria-hidden="true"><PanelIcon name="clipboard" /></span>
      <div class="text">
        <span class="caps" in:rise={{ index: 1 }}>{caps}</span>
        <b in:rise={{ index: 2 }}>{into}</b>
      </div>
      <span class="slot" aria-hidden="true"></span>
    </div>

    {#if ask.status === "confirm" || ask.status === "sending"}
      <p class="lead" in:rise={{ index: 3 }}>{ask.status === "sending" ? t.sending : t.lead}</p>
      <blockquote class="quote" class:waiting={ask.status === "sending"} in:rise={{ index: 3 }}>{ask.text}</blockquote>
      <div class="actions" in:rise={{ index: 4 }}>
        {#if ask.status === "confirm"}
          <button type="button" class="btn sm primary" bind:this={first} onclick={() => void translations.sendClipboard()}>{t.send}</button>
        {/if}
        <button type="button" class="btn sm" onclick={() => translations.closeClipboard()}>{t.cancel}</button>
        <span class="count">{t.chars(count.toLocaleString(i18n.locale), count)}</span>
      </div>
    {:else if ask.status === "done"}
      <blockquote class="quote result" in:rise={{ index: 3 }} aria-live="polite">
        {#key ask.original}<span in:swap>{shown}</span>{/key}
      </blockquote>
      <div class="actions" in:rise={{ index: 4 }}>
        <button type="button" class="btn sm primary" bind:this={first} onclick={() => void copy()}>{copied ? t.copied : t.copy}</button>
        <button type="button" class="btn sm" aria-pressed={ask.original} onclick={() => translations.toggleClipboardOriginal()}>
          {ask.original ? i18n.t.translate.showTranslation : i18n.t.translate.showOriginal}
        </button>
        <button type="button" class="btn sm" onclick={() => translations.closeClipboard()}>{t.close}</button>
      </div>
    {:else}
      <p class="lead" class:failed={ask.status === "error"} role={ask.status === "error" ? "alert" : "status"} in:rise={{ index: 3 }}>
        {problem}
      </p>
      <div class="actions" in:rise={{ index: 4 }}>
        {#if ask.status === "noKey"}
          <button type="button" class="btn sm primary" bind:this={first} onclick={() => translations.addKey()}>{t.addKey}</button>
        {:else if ask.status === "error"}
          <button type="button" class="btn sm primary" bind:this={first} onclick={() => void translations.retryClipboard()}>
            {i18n.t.translate.retry}
          </button>
        {/if}
        <button type="button" class="btn sm" onclick={() => translations.closeClipboard()}>{t.close}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .clipboard {
    padding: var(--s3) var(--s4);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }

  /* The clipboard on a surface, like an app's icon on a message. */
  .tile {
    width: 36px;
    height: 36px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 11px;
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--accent-text);
  }

  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }

  .caps {
    margin-bottom: 1px;
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Mochi floats over this slot (IslandMochi). */
  .slot {
    width: 36px;
    height: 36px;
    flex: none;
  }

  .lead {
    margin-top: var(--s3);
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .lead.failed {
    color: var(--err);
  }

  /* Exactly what leaves the PC, or what came back: set apart like a quote of data. */
  .quote {
    margin-top: var(--s2);
    max-height: 132px;
    overflow-y: auto;
    padding: var(--s2) var(--s3);
    border-radius: var(--r-chip);
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--ink);
    font-size: var(--fs-3);
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .quote.waiting {
    color: var(--ink-2);
  }

  /* The translation is what the user came for: body size, more room. */
  .quote.result {
    margin-top: var(--s3);
    max-height: 168px;
    font-size: var(--fs-body);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  .count {
    margin-left: auto;
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-variant-numeric: tabular-nums;
  }
</style>
