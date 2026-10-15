<!-- Réglages › Chat › Traduction (prompt 12, DeepL). The user's DeepL key goes straight to
     the Windows Credential Manager: this page never shows it again, it only says « Clé
     enregistrée », and offers to replace or erase it. The page where keys are made is a
     link to copy: îlot opens no page by itself. What DeepL keeps of the texts is said under
     the key. Then the language texts go into, the interface's by default. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { copyText } from "../../lib/ipc";
  import { reduced } from "../../lib/motion.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import { DEEPL_KEY_PAGE, languageName, targetChoices } from "../../lib/translate";
  import Button from "../settings/Button.svelte";
  import Row from "../settings/Row.svelte";

  /** How long « Lien copié » stays. */
  const COPIED_MS = 1600;

  const t = $derived(i18n.t.translate.settings);
  const keys = $derived(i18n.t.settings.chat);
  const choices = $derived(targetChoices(i18n.locale));
  const chosen = $derived(settings.value.translate.target);
  const saved = $derived(translations.hasKey === true);
  /** The interface's language by its plain name (« français », « English »). */
  const own = $derived(languageName(i18n.locale.toUpperCase(), i18n.locale));

  let typed = $state("");
  let editing = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let copied = $state(false);
  let section = $state<HTMLElement>();
  let field = $state<HTMLInputElement>();

  $effect(() => {
    void translations.refreshKey();
  });

  // Opened by a « Traduire » without a key: this section comes into view, the field takes
  // the keyboard.
  $effect(() => {
    if (!translations.wantsKey || !section) return;
    translations.wantsKey = false;
    section.scrollIntoView({ block: "start", behavior: reduced() ? "auto" : "smooth" });
    requestAnimationFrame(() => field?.focus({ preventScroll: true }));
  });

  async function save(): Promise<void> {
    if (!typed.trim() || busy) return;
    busy = true;
    error = await translations.saveKey(typed);
    busy = false;
    if (!error) {
      typed = "";
      editing = false;
    }
  }

  async function erase(): Promise<void> {
    busy = true;
    error = await translations.removeKey();
    busy = false;
  }

  async function copyLink(): Promise<void> {
    await copyText(DEEPL_KEY_PAGE);
    copied = true;
    setTimeout(() => (copied = false), COPIED_MS);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      void save();
    }
  }

  function choose(target: string): void {
    void settings.update({ translate: { target } });
  }
</script>

<section aria-labelledby="translate-title" bind:this={section}>
  <h3 class="label" id="translate-title">{t.title}</h3>
  <p class="intro">{t.intro}</p>
  <div class="rows">
    <Row title={t.key} titleId="key-deepl" index={0}>
      {#snippet hint()}
        {#if error}
          <span class="err" role="alert">{error}</span>
        {:else if saved}
          {keys.keptByWindows}
        {:else}
          {t.keyHint}
          <button type="button" class="link" onclick={() => void copyLink()}>{copied ? t.linkCopied : t.copyLink}</button>
        {/if}
      {/snippet}
      {#if saved && !editing}
        <span class="saved" role="status">{keys.keySaved}</span>
        <Button onclick={() => (editing = true)}>{keys.replace}</Button>
        <Button variant="quiet" disabled={busy} onclick={() => void erase()}>{keys.erase}</Button>
      {:else}
        <input
          type="password"
          autocomplete="off"
          spellcheck="false"
          aria-labelledby="key-deepl"
          placeholder="xxxxxxxx-…:fx"
          bind:this={field}
          bind:value={typed}
          {onkeydown}
        />
        <Button variant={typed.trim() ? "primary" : "plain"} disabled={!typed.trim() || busy} onclick={() => void save()}>
          {keys.saveKey}
        </Button>
        {#if saved}
          <Button variant="quiet" onclick={() => (editing = false)}>{keys.cancel}</Button>
        {/if}
      {/if}
    </Row>
    <Row title={t.target} titleId="translate-target" hint={t.targetHint} index={1}>
      <select aria-labelledby="translate-target" value={chosen} onchange={(event) => choose(event.currentTarget.value)}>
        <option value="">{t.followInterface(own)}</option>
        {#each choices as choice (choice.code)}
          <option value={choice.code}>{choice.name}</option>
        {/each}
      </select>
    </Row>
  </div>
  <p class="note">{t.privacy}</p>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    scroll-margin-top: var(--s4);
  }

  .label {
    margin-bottom: var(--s1);
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .intro,
  .note {
    max-width: 72ch;
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  .intro {
    margin-bottom: var(--s2);
  }

  .note {
    margin-top: var(--s2);
  }

  .rows {
    padding: 0 var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .rows > :global(:first-child) {
    border-top: 0;
  }

  select,
  input {
    height: var(--h-ctl);
    border-radius: var(--r-ctl);
    background: var(--well);
    border: 1px solid var(--line);
    color: var(--ink);
    font-size: var(--fs-2);
  }

  select {
    max-width: 230px;
    padding: 0 var(--s2);
  }

  input {
    width: 220px;
    padding: 0 var(--s3);
    font-family: var(--font-mono);
    user-select: text;
  }

  input::placeholder {
    color: var(--ink-3);
  }

  select:focus-visible,
  input:focus-visible,
  .link:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* The key page, to paste in a browser: îlot does not open it. Its words line up with
     the hint's when it wraps. */
  .link {
    margin-left: calc(-1 * var(--s1));
    padding: 0 var(--s1);
    border-radius: var(--r-chip);
    color: var(--accent-text);
    font-size: inherit;
    font-weight: 500;
    transition: background-color var(--t-quick);
  }

  .link:hover {
    background: var(--hover);
  }

  .saved {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 var(--s2);
    border-radius: var(--r-pill);
    background: var(--ok-soft);
    color: var(--ok);
    font-size: var(--fs-2);
    font-weight: 500;
  }

  .err {
    color: var(--err);
  }
</style>
