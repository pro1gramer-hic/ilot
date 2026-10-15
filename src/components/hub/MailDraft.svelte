<!-- An e-mail draft in the hub (prompt 12, lot 3): recipient, subject, text and the file
     Mochi swallowed. Nothing leaves before « Envoyer »; then the mail app opens with the
     message ready, and the user sends it from there. The island says beforehand which app
     will open. -->
<script lang="ts">
  import { fileNote } from "../../lib/chat";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { mail } from "../../lib/stores/mail.svelte";
  import HubIcon from "./HubIcon.svelte";

  let to = $state<HTMLInputElement>();

  const t = $derived(i18n.t.hub.mail);
  const appName = $derived(mail.app?.name ?? t.yourApp);
  const ahead = $derived(mail.app?.mapi === false ? t.noAttach : t.opens(mail.app?.name));

  $effect(() => {
    if (to) requestAnimationFrame(() => to?.focus({ preventScroll: true }));
  });

  function cancel(): void {
    mail.discard();
    island.showHub("home");
  }

</script>

<!-- Not a <form>: Entrée in a field must never send; only the button does. -->
<div class="mail" role="form" aria-label={t.label}>
  <div class="bar">
    <button type="button" class="back" aria-label={i18n.t.hub.back} title={i18n.t.hub.back} onclick={() => island.showHub("home")}>
      <HubIcon name="back" />
    </button>
    <h2>{t.title}</h2>
    <span class="app"><HubIcon name="mail" size={14} />{appName}</span>
  </div>

  <div class="fields">
    <label class="line">
      <span>{t.to}</span>
      <input bind:this={to} bind:value={mail.to} inputmode="email" autocomplete="email" placeholder={t.toHint} spellcheck="false" />
    </label>
    <label class="line">
      <span>{t.subject}</span>
      <input bind:value={mail.subject} maxlength="255" placeholder={t.subject} />
    </label>
    <textarea bind:value={mail.body} aria-label={t.body} placeholder={t.bodyHint}></textarea>
    {#if mail.attachment}
      <span class="chip" title={fileNote(mail.attachment)}>
        <HubIcon name="clip" size={12} />
        <span class="name">{mail.attachment.name}</span>
        <button type="button" aria-label={i18n.t.hub.remove(mail.attachment.name)} onclick={() => (mail.attachment = null)}>
          <HubIcon name="cross" size={12} />
        </button>
      </span>
    {/if}
  </div>

  <div class="foot">
    <p class:err={mail.error} role={mail.error ? "alert" : undefined}>{mail.error ?? ahead}</p>
    <button type="button" class="btn sm" onclick={cancel}>{i18n.t.hub.cancel}</button>
    <button type="button" class="btn sm primary" disabled={mail.sending} onclick={() => void mail.send()}>
      {mail.sending ? t.opening : t.send}
    </button>
  </div>
</div>

<style>
  .mail {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--s1);
    height: 34px;
    padding: 0 var(--gap-mark);
  }

  .back {
    width: 32px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    transition:
      background-color var(--t-quick),
      color var(--t-quick);
  }

  .back:hover {
    background: var(--surface);
    color: var(--ink);
  }

  h2 {
    flex: 1;
    font-size: var(--fs-4);
    font-weight: 600;
  }

  .app {
    display: inline-flex;
    align-items: center;
    gap: var(--s1);
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .fields {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    overflow: hidden;
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--s3);
    height: 36px;
    padding: 0 var(--s4);
    border-bottom: 1px solid var(--line);
  }

  .line span {
    width: 44px;
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  input,
  textarea {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--ink);
    font: 400 var(--fs-3) / 1.45 var(--font-ui);
    user-select: text;
  }

  input::placeholder,
  textarea::placeholder {
    color: var(--ink-3);
  }

  .line:focus-within {
    background: var(--hover);
  }

  textarea {
    min-height: 0;
    padding: var(--s2) var(--s4);
    resize: none;
  }

  textarea:focus-visible {
    background: var(--hover);
  }

  .chip {
    align-self: flex-start;
    max-width: 60%;
    display: inline-flex;
    align-items: center;
    gap: var(--s1);
    height: 26px;
    margin: 0 var(--s3) var(--s3);
    padding: 0 var(--s1) 0 var(--s2);
    border-radius: var(--r-pill);
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: var(--fs-2);
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink);
  }

  .chip button {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--ink-2);
  }

  .chip button:hover {
    background: var(--hover);
    color: var(--ink);
  }

  .foot {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .foot p {
    flex: 1;
    min-width: 0;
    color: var(--ink-3);
    font-size: var(--fs-2);
    line-height: 1.35;
  }

  .foot p.err {
    color: var(--err);
  }
</style>
