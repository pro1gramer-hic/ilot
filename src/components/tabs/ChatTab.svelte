<!-- « Chat » in Réglages (prompt 12, lot 3): the provider and the model the chat opens on,
     the keys of Claude, Gemini and OpenAI, and the two local servers. A key goes straight
     to the Windows Credential Manager: this page never shows it again, it only says
     « Clé enregistrée », and offers to replace or erase it. Last, DeepL's translation
     (prompt 12, TranslateSettings). -->
<script lang="ts">
  import { PROVIDERS, type ProviderInfo, providerInfo } from "../../lib/chat";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import type { ChatProvider } from "../../lib/ipc";
  import { chat } from "../../lib/stores/chat.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Segmented from "../panel/Segmented.svelte";
  import Button from "../settings/Button.svelte";
  import Row from "../settings/Row.svelte";
  import TranslateSettings from "../translate/TranslateSettings.svelte";

  const remote = PROVIDERS.filter((p) => !p.local);
  const local = PROVIDERS.filter((p) => p.local);
  const options = PROVIDERS.map((p) => [p.id, p.label] as const);

  const t = $derived(i18n.t.settings.chat);
  const chosen = $derived(settings.value.chat);
  const models = $derived(chat.models[chosen.provider]);

  /** Typed keys, until saved (then emptied); which rows show their field. */
  let typed = $state<Partial<Record<ChatProvider, string>>>({});
  let editing = $state<Partial<Record<ChatProvider, boolean>>>({});
  let errors = $state<Partial<Record<ChatProvider, string | null>>>({});
  let busy = $state<ChatProvider | null>(null);

  $effect(() => {
    for (const provider of remote) void chat.refreshKey(provider.id);
  });
  $effect(() => {
    if (!providerInfo(chosen.provider).secret || chat.keys[chosen.provider]) void chat.loadModels(chosen.provider);
  });

  function choose(provider: string): void {
    const next = PROVIDERS.find((p) => p.id === provider)?.id ?? "anthropic";
    void settings.update({ chat: { provider: next, model: "" } });
  }

  function chooseModel(model: string): void {
    void settings.update({ chat: { ...chosen, model } });
  }

  async function save(provider: ProviderInfo): Promise<void> {
    const key = typed[provider.id] ?? "";
    if (!key.trim() || busy) return;
    busy = provider.id;
    const error = await chat.saveKey(provider.id, key);
    busy = null;
    errors[provider.id] = error;
    if (!error) {
      typed[provider.id] = "";
      editing[provider.id] = false;
    }
  }

  async function erase(provider: ProviderInfo): Promise<void> {
    busy = provider.id;
    errors[provider.id] = await chat.removeKey(provider.id);
    busy = null;
  }

  function onkeydown(event: KeyboardEvent, provider: ProviderInfo): void {
    if (event.key === "Enter") {
      event.preventDefault();
      void save(provider);
    }
  }
</script>

<div class="chat-tab">
  <p class="intro">{t.intro}</p>

  <section aria-labelledby="chat-default">
    <h3 class="label" id="chat-default">{t.defaults}</h3>
    <ul class="list">
      <li>
        <span class="text">
          <b id="chat-provider">{t.provider}</b>
          <small>{t.providerHint}</small>
        </span>
        <Segmented {options} value={chosen.provider} labelledby="chat-provider" onchange={choose} />
      </li>
      <li>
        <span class="text">
          <b id="chat-model">{t.model}</b>
          <small>
            {#if models?.loading}
              {t.loadingModels}
            {:else if models?.error}
              <span class="err">{models.error}</span>
            {:else if models}
              {t.models(models.list.length, providerInfo(chosen.provider).label)}
            {:else}
              {t.addKeyFirst}
            {/if}
          </small>
        </span>
        <span class="end">
          <select
            aria-labelledby="chat-model"
            value={chosen.model}
            disabled={!models?.list.length && !chosen.model}
            onchange={(event) => chooseModel(event.currentTarget.value)}
          >
            <option value="">{t.firstModel}</option>
            {#if chosen.model && !models?.list.includes(chosen.model)}
              <option value={chosen.model}>{chosen.model}</option>
            {/if}
            {#each models?.list ?? [] as model (model)}
              <option value={model}>{model}</option>
            {/each}
          </select>
          <Button variant="quiet" onclick={() => void chat.loadModels(chosen.provider, true)}>{t.refresh}</Button>
        </span>
      </li>
    </ul>
  </section>

  <section aria-labelledby="chat-keys">
    <h3 class="label" id="chat-keys">{t.keys}</h3>
    <div class="rows">
      {#each remote as provider, index (provider.id)}
        {@const saved = chat.keys[provider.id] === true}
        <Row title={provider.maker === provider.label ? provider.label : `${provider.label} (${provider.maker})`} titleId="key-{provider.id}" {index}>
          {#snippet hint()}
            {#if errors[provider.id]}
              <span class="err" role="alert">{errors[provider.id]}</span>
            {:else if saved}
              {t.keptByWindows}
            {:else}
              {t.createKey(provider.keyPage)}
            {/if}
          {/snippet}
          {#if saved && !editing[provider.id]}
            <span class="saved" role="status">{t.keySaved}</span>
            <Button onclick={() => (editing[provider.id] = true)}>{t.replace}</Button>
            <Button variant="quiet" disabled={busy === provider.id} onclick={() => void erase(provider)}>{t.erase}</Button>
          {:else}
            <input
              type="password"
              autocomplete="off"
              spellcheck="false"
              aria-labelledby="key-{provider.id}"
              placeholder={provider.keyHint}
              bind:value={typed[provider.id]}
              onkeydown={(event) => onkeydown(event, provider)}
            />
            <Button
              variant={typed[provider.id]?.trim() ? "primary" : "plain"}
              disabled={!typed[provider.id]?.trim() || busy === provider.id}
              onclick={() => void save(provider)}
            >
              {t.saveKey}
            </Button>
            {#if saved}
              <Button variant="quiet" onclick={() => (editing[provider.id] = false)}>{t.cancel}</Button>
            {/if}
          {/if}
        </Row>
      {/each}
    </div>
  </section>

  <section aria-labelledby="chat-local">
    <h3 class="label" id="chat-local">{t.local}</h3>
    <div class="rows">
      {#each local as provider, index (provider.id)}
        {@const found = chat.models[provider.id]}
        <Row title={provider.label} {index}>
          {#snippet hint()}
            {#if found?.loading}
              {t.findingServer}
            {:else if found?.error}
              <span class="err">{found.error}</span>
            {:else if found}
              {t.ready(found.list.length)}
            {:else}
              {t.port(provider.id === "ollama" ? 11434 : 1234)}
            {/if}
          {/snippet}
          <Button onclick={() => void chat.loadModels(provider.id, true)}>{t.check}</Button>
        </Row>
      {/each}
      <Row title={t.dictation} index={2}>
        {#snippet hint()}
          {#if chat.canDictate}
            {t.dictationOn}
          {:else}
            {t.dictationOff}
          {/if}
        {/snippet}
        <span class="saved" class:off={!chat.canDictate}>{chat.canDictate ? t.dictationReady : t.dictationMissing}</span>
      </Row>
    </div>
  </section>

  <TranslateSettings />
</div>

<style>
  .chat-tab {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
  }

  .intro {
    max-width: 72ch;
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .label {
    margin-bottom: var(--s1);
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .list {
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s3) var(--s4);
  }

  .list li + li {
    border-top: 1px solid var(--line);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  small {
    margin-top: calc(var(--s1) / 2);
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  .end {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
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
    font-family: var(--font-mono);
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
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  select:disabled {
    color: var(--ink-3);
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

  .saved.off {
    background: var(--surface-2);
    color: var(--ink-3);
  }

  .err {
    color: var(--err);
  }

  @container panel (max-width: 820px) {
    .list li {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
