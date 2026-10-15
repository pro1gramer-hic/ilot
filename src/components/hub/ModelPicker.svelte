<!-- The chat's model picker (prompt 12, lot 3): a click on the model's name opens it under
     the button. On the left the five providers (Claude, Gemini, OpenAI with the user's key,
     Ollama and LM Studio on this PC), on the right the models the chosen one offers, read
     from the provider itself. A provider without its key says so and leads to Réglages.
     Escape or a click outside closes it; the keyboard moves with the arrows. -->
<script lang="ts">
  import { PROVIDERS, providerInfo } from "../../lib/chat";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import type { ChatProvider } from "../../lib/ipc";
  import { chat } from "../../lib/stores/chat.svelte";
  import HubIcon from "./HubIcon.svelte";

  let { onclose, onkey }: { onclose: (focusBack: boolean) => void; onkey: () => void } = $props();

  const t = $derived(i18n.t.hub.picker);
  let shown = $state<ChatProvider>(chat.provider);
  const info = $derived(providerInfo(shown));
  const missingKey = $derived(!info.local && chat.keys[shown] === false);
  const list = $derived(chat.models[shown]);
  let box = $state<HTMLElement>();

  $effect(() => {
    void chat.refreshKey(shown);
  });
  $effect(() => {
    if (!missingKey) void chat.loadModels(shown);
  });

  // The keyboard lands on the model in use, else on its provider.
  $effect(() => {
    const target = box?.querySelector<HTMLElement>(".models [aria-current='true'], .providers [aria-checked='true']");
    target?.focus({ preventScroll: true });
  });

  function pick(model: string): void {
    chat.choose(shown, model);
    onclose(true);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onclose(true);
      return;
    }
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0 || !(event.target instanceof HTMLElement)) return;
    const group = event.target.closest(".providers, .models");
    if (!group) return;
    const items = [...group.querySelectorAll<HTMLButtonElement>("button")];
    const index = items.indexOf(event.target as HTMLButtonElement);
    if (index < 0) return;
    event.preventDefault();
    const next = items[(index + step + items.length) % items.length];
    next.focus();
    if (group.classList.contains("providers")) next.click();
  }

  function onwindowpointerdown(event: PointerEvent): void {
    if (event.target instanceof Element && event.target.closest(".picker, .model-button")) return;
    onclose(false);
  }
</script>

<svelte:window onpointerdown={onwindowpointerdown} />

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="picker" role="dialog" aria-label={t.label} bind:this={box} {onkeydown}>
  <div class="providers" role="radiogroup" aria-label={t.providers}>
    {#each PROVIDERS as provider (provider.id)}
      <button
        type="button"
        role="radio"
        aria-checked={shown === provider.id}
        tabindex={shown === provider.id ? 0 : -1}
        onclick={() => (shown = provider.id)}
      >
        <b>{provider.label}</b>
        <small class:warn={!provider.local && chat.keys[provider.id] === false}>
          {provider.local ? t.local : chat.keys[provider.id] === false ? t.noKey : provider.maker}
        </small>
      </button>
    {/each}
  </div>

  <div class="models" aria-live="polite">
    {#if missingKey}
      <div class="state">
        <p>{t.addKeyFor(info.label)}</p>
        <button type="button" class="btn sm primary" onclick={onkey}>{i18n.t.hub.addKey}</button>
      </div>
    {:else if !list || (list.loading && list.list.length === 0)}
      <p class="state quiet">{t.loading(info.label)}</p>
    {:else if list.error}
      <div class="state">
        <p class="err">{list.error}</p>
        <button type="button" class="btn sm" onclick={() => void chat.loadModels(shown, true)}>{t.retry}</button>
      </div>
    {:else}
      <ul aria-label={t.models(info.label)}>
        {#each list.list as model (model)}
          {@const current = chat.provider === shown && chat.model === model}
          <li>
            <button type="button" aria-current={current ? "true" : undefined} tabindex={current ? 0 : -1} onclick={() => pick(model)}>
              <span class="id">{model}</span>
              {#if current}<HubIcon name="check" size={14} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="where">{info.local ? t.viaLocalhost : t.withKey(info.maker)}</p>
  </div>
</div>

<style>
  .picker {
    position: absolute;
    z-index: 3;
    top: calc(var(--s8) + var(--s3));
    left: 0;
    width: 420px;
    height: 272px;
    display: grid;
    grid-template-columns: 136px minmax(0, 1fr);
    border-radius: var(--r-card);
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line-2), var(--shadow-soft);
    overflow: hidden;
    transform-origin: 24px 0;
    animation: open var(--t-enter) var(--ease-out);
  }

  @keyframes open {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.97);
    }
  }

  .providers {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--s2);
    border-right: 1px solid var(--line);
  }

  .providers button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: var(--s1) var(--s2);
    border-radius: var(--r-ctl);
    text-align: left;
    line-height: 1.25;
    transition: background-color var(--t-quick);
  }

  .providers button:hover {
    background: var(--hover);
  }

  .providers button[aria-checked="true"] {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .providers b {
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .providers small {
    color: var(--ink-3);
    font-size: var(--fs-1);
  }

  .providers small.warn {
    color: var(--warn);
  }

  .models {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: var(--s2);
  }

  ul {
    flex: 1;
    min-height: 0;
    list-style: none;
    overflow-y: auto;
  }

  ul button {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2);
    height: var(--h-row);
    padding: 0 var(--s2);
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    text-align: left;
    transition:
      background-color var(--t-quick),
      color var(--t-quick);
  }

  ul button:hover {
    background: var(--hover);
    color: var(--ink);
  }

  ul button[aria-current="true"] {
    color: var(--accent-text);
  }

  .id {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 500 var(--fs-2) var(--font-mono);
  }

  .state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: var(--s2);
    padding: 0 var(--s2);
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .state.quiet {
    color: var(--ink-3);
  }

  .err {
    color: var(--err);
  }

  .where {
    padding: var(--s1) var(--s2) 0;
    color: var(--ink-3);
    font-size: var(--fs-1);
  }

  @media (prefers-reduced-motion: reduce) {
    .picker {
      animation-name: fade;
    }

    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
