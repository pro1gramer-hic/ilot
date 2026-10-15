<!-- The form that adds a service pastille: its token (straight to the Windows Credential
     Manager, never shown again), its instance's address when it has one, the identifier it
     needs, the least rights to give the token, and a warning when no key can be read-only.
     The link to create the token is copied, not opened (îlot opens no web page). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { copyText } from "../../lib/ipc";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { reduced } from "../../lib/motion.svelte";
  import type { ServiceInfo } from "../../lib/pastilles";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import Icon from "../Icon.svelte";

  let { service, ondone }: { service: ServiceInfo; ondone: () => void } = $props();

  const t = $derived(i18n.t.pastilles.form);
  let token = $state("");
  let baseUrl = $state("");
  let account = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let copied = $state(false);
  let root = $state<HTMLFormElement>();

  // The form opens under the catalog: it comes into view, its token field ready.
  $effect(() => {
    if (!root) return;
    root.scrollIntoView({ block: "nearest", behavior: reduced() ? "auto" : "smooth" });
    root.querySelector<HTMLInputElement>("input")?.focus({ preventScroll: true });
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    busy = true;
    error = await pastilles.addService(service, { token, baseUrl, account });
    busy = false;
    // The token leaves the page as soon as it is in the vault.
    token = "";
    if (!error) ondone();
  }

  async function copyLink(): Promise<void> {
    await copyText(service.help);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<form class="form" aria-labelledby="form-{service.id}" onsubmit={submit} bind:this={root}>
  <header>
    <Mochi palette={service.palette} kind={service.kind} move="breathe" face="happy" scale={1.5} shadow={false} label="" />
    <h4 id="form-{service.id}">{t.title(service.name)}</h4>
    <span class="pace">{t.pace(service.refreshMinutes)}</span>
  </header>

  <p class="rights"><b>{t.rights}</b> {service.rights}</p>
  {#if service.warning}
    <p class="warning"><Icon name="warn" size={14} /><span>{service.warning}</span></p>
  {/if}

  <div class="fields">
    <label class="field">
      <span>{service.tokenLabel}</span>
      <input
        type="password"
        bind:value={token}
        autocomplete="off"
        spellcheck="false"
        placeholder={service.hasToken ? t.keepToken : t.pasteToken}
      />
    </label>
    {#if service.customUrl !== undefined}
      <label class="field">
        <span>{t.instance(Boolean(service.urlRequired))}</span>
        <input type="url" bind:value={baseUrl} autocomplete="off" spellcheck="false" placeholder={service.customUrl} />
      </label>
    {/if}
    {#if service.account}
      <label class="field">
        <span>{service.account.label}</span>
        <input bind:value={account} autocomplete="off" spellcheck="false" />
        <small>{service.account.hint}</small>
      </label>
    {/if}
  </div>

  <p class="help">
    {t.createToken} <span class="mono">{service.help}</span>
    <button type="button" class="link" onclick={copyLink}>{copied ? t.copied : t.copy}</button>
  </p>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="actions">
    <button type="submit" class="btn sm primary" disabled={busy}>{t.submit}</button>
    <button type="button" class="btn sm" onclick={ondone}>{i18n.t.pastilles.cancel}</button>
    <span class="vault">{t.vault}</span>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: var(--s4);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  h4 {
    font-size: var(--fs-4);
    font-weight: 600;
  }

  .pace {
    margin-left: auto;
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .rights,
  .help,
  .vault {
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  .rights b {
    color: var(--ink);
    font-weight: 600;
  }

  .warning {
    display: flex;
    gap: var(--s2);
    padding: var(--s2) var(--s3);
    border-radius: var(--r-ctl);
    background: var(--warn-soft);
    box-shadow: inset 0 0 0 1px var(--warn-line);
    color: var(--ink);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  .warning :global(svg) {
    margin-top: 1px;
    color: var(--warn);
  }

  .fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--s3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    min-width: 0;
    font-size: var(--fs-2);
    color: var(--ink-2);
  }

  .field small {
    color: var(--ink-3);
    font-size: var(--fs-1);
    line-height: 1.4;
  }

  input {
    height: 32px;
    padding: 0 var(--s2);
    border-radius: var(--r-ctl);
    border: 1px solid var(--line-2);
    background: var(--panel);
    color: var(--ink);
    font-size: var(--fs-3);
    user-select: text;
  }

  input::placeholder {
    color: var(--ink-3);
  }

  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }

  .help .mono {
    color: var(--ink);
    font-size: var(--fs-1);
    word-break: break-all;
    user-select: text;
  }

  .link {
    padding: 0 var(--gap-mark);
    border-radius: var(--r-chip);
    color: var(--accent-text);
    font-size: var(--fs-2);
    font-weight: 600;
    white-space: nowrap;
  }

  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .error {
    color: var(--err);
    font-size: var(--fs-3);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s2) var(--s3);
  }

  .vault {
    color: var(--ink-3);
  }
</style>
