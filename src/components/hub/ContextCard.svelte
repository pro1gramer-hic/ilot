<!-- A window Mochi landed on (prompt 12, lot 3), offered as context for the chat. Three
     steps, each one the user's click: « Lire la fenêtre » reads its text on this PC and shows
     it here, « Joindre au chat » puts it with the next question, which alone leaves for the
     provider. A protected window (password manager, bank, sign-in page, administrator) is
     never read: the card says why. No screenshot is ever taken. -->
<script lang="ts">
  import { providerInfo } from "../../lib/chat";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { chat } from "../../lib/stores/chat.svelte";
  import HubIcon from "./HubIcon.svelte";

  const proposal = $derived(chat.proposal);
  const preview = $derived(chat.preview);
  const label = $derived(providerInfo(chat.provider).label);
  const t = $derived(i18n.t.hub.context);
</script>

{#if proposal}
  <section class="card" class:blocked={proposal.blocked} aria-label={t.label}>
    <span class="icon" aria-hidden="true"><HubIcon name={proposal.blocked ? "alert" : "window"} /></span>
    {#if proposal.blocked}
      <div class="text" role="status">
        <b>{t.blocked(proposal.app)}</b>
        <p>{proposal.blocked}</p>
      </div>
      <div class="actions">
        <button type="button" class="btn sm" onclick={() => chat.dismissContext()}>{t.gotIt}</button>
      </div>
    {:else if preview}
      <div class="text">
        <b title={preview.title}>{preview.title}</b>
        <p>{t.read(preview.text.length, preview.truncated, label)}</p>
        <!-- A scrolling region must take the keyboard to be read to the end (WCAG 2.1.1). -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <pre class="preview" role="region" tabindex="0" aria-label={t.preview}>{preview.text}</pre>
      </div>
      <div class="actions">
        <button type="button" class="btn sm primary" onclick={() => void chat.joinContext()}>{t.join}</button>
        <button type="button" class="btn sm" onclick={() => chat.dismissContext()}>{i18n.t.hub.cancel}</button>
      </div>
    {:else}
      <div class="text" role="status">
        <b title={proposal.title}>{proposal.title || t.untitled}</b>
        <p>
          {#if chat.contextError}
            <span class="err">{chat.contextError}</span>
          {:else if chat.reading}
            {t.reading}
          {:else}
            {t.offer(proposal.app)}
          {/if}
        </p>
      </div>
      <div class="actions">
        {#if chat.contextError}
          <button type="button" class="btn sm" onclick={() => chat.dismissContext()}>{t.gotIt}</button>
        {:else}
          <button type="button" class="btn sm primary" disabled={chat.reading} onclick={() => void chat.readContext()}>
            {t.readWindow}
          </button>
          <button type="button" class="btn sm" onclick={() => chat.dismissContext()}>{t.noThanks}</button>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .card {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: start;
    gap: var(--s3);
    padding: var(--s3);
    border-radius: var(--r-card);
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
    animation: arrive var(--t-enter) var(--ease-out);
  }

  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
  }

  .icon {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--r-ctl);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .blocked .icon {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .text {
    min-width: 0;
    line-height: 1.35;
  }

  b {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-3);
    font-weight: 600;
  }

  p {
    margin-top: 2px;
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .err {
    color: var(--err);
  }

  .preview {
    max-height: 132px;
    margin-top: var(--s2);
    padding: var(--s2) var(--s3);
    overflow: auto;
    border-radius: var(--r-ctl);
    background: var(--island);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink-2);
    font: 400 var(--fs-2) / 1.45 var(--font-ui);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
  }

  @media (prefers-reduced-motion: reduce) {
    .card {
      animation-name: fade;
    }

    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
