<!-- The chat in the hub (prompt 12, lot 3), as tall as a session followed live. On top,
     Mochi (he thinks while a model answers) and the model's name: a click changes the
     provider and the model. Below, the conversation, kept in memory only, and the field:
     Entrée sends, Maj + Entrée goes to the line. A dropped file and a window Mochi landed
     on join the next question as chips, until it is sent. The microphone shows only when
     Windows has its French recognizer: dictation stays on this PC. A finished answer can be
     translated by DeepL (prompt 12): « Traduire » sits by the name of who answered. -->
<script lang="ts">
  import { tick } from "svelte";
  import { blocker, destination, fileNote, modelText, providerInfo, readyToSend } from "../../lib/chat";
  import { blocks } from "../../lib/chat-text";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { copyText } from "../../lib/ipc";
  import { island } from "../../lib/island/controller.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { chat } from "../../lib/stores/chat.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import TranslateButton from "../translate/TranslateButton.svelte";
  import { swap } from "../translate/swap";
  import ContextCard from "./ContextCard.svelte";
  import HubIcon from "./HubIcon.svelte";
  import ModelPicker from "./ModelPicker.svelte";

  /** The field grows with its text up to four lines. */
  const FIELD_MAX_PX = 92;
  const t = $derived(i18n.t.hub.chat);

  const info = $derived(providerInfo(chat.provider));
  const stop = $derived(blocker(info, chat.keys[chat.provider]));
  const joined = $derived(chat.attachment !== null || chat.window !== null);
  const ready = $derived(!stop && readyToSend(chat.draft, joined));
  const empty = $derived(chat.messages.length === 0);

  let picking = $state(false);
  let thread = $state<HTMLElement>();
  let field = $state<HTMLTextAreaElement>();
  let modelButton = $state<HTMLButtonElement>();
  let copied = $state<string | null>(null);

  // The thread follows an answer as it streams, unless the user scrolled up to read.
  let pinned = true;
  function onscroll(): void {
    if (thread) pinned = thread.scrollHeight - thread.scrollTop - thread.clientHeight < 32;
  }
  $effect(() => {
    void chat.messages.length;
    void chat.messages.at(-1)?.text;
    void chat.messages.at(-1)?.error;
    if (!thread || !pinned) return;
    void tick().then(() => thread?.scrollTo({ top: thread.scrollHeight }));
  });

  $effect(() => {
    void chat.draft;
    if (!field) return;
    field.style.height = "auto";
    field.style.height = `${Math.min(field.scrollHeight, FIELD_MAX_PX)}px`;
  });

  // The microphone never outlives the chat on screen.
  $effect(() => () => {
    if (chat.listening) void chat.toggleDictation();
  });

  // The keyboard starts in the field, unless a window waits for an answer.
  $effect(() => {
    if (field && !chat.proposal) requestAnimationFrame(() => field?.focus({ preventScroll: true }));
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      void chat.send();
    }
  }

  function suggest(text: string): void {
    chat.draft = text;
    field?.focus();
  }

  function closePicker(focusBack: boolean): void {
    picking = false;
    if (focusBack) modelButton?.focus();
  }

  function addKey(): void {
    picking = false;
    island.openPanel("chat");
  }

  async function copy(id: string, text: string): Promise<void> {
    await copyText(text);
    copied = id;
    setTimeout(() => {
      if (copied === id) copied = null;
    }, 1600);
  }
</script>

<div class="chat" class:picking>
  <header class="top">
    <span class="me" aria-hidden="true">
      <Mochi move={chat.thinking ? "think" : "breathe"} scale={1.5} shadow={false} label="" />
    </span>
    <button
      type="button"
      class="model-button"
      aria-haspopup="dialog"
      aria-expanded={picking}
      title={t.change}
      bind:this={modelButton}
      onclick={() => (picking = !picking)}
    >
      <b>{info.label}</b>
      <span class="id">{modelText(chat.model)}</span>
      <HubIcon name="chevron" size={14} />
    </button>
    <button
      type="button"
      class="icon"
      aria-label={t.newChat}
      title={t.newChat}
      disabled={empty && !joined}
      onclick={() => chat.clear()}
    >
      <HubIcon name="compose" />
    </button>
  </header>

  {#if picking}
    <ModelPicker onclose={closePicker} onkey={addKey} />
  {/if}

  <ContextCard />

  <div class="thread" bind:this={thread} {onscroll} role="log" aria-label={t.thread} tabindex="-1">
    {#if empty && chat.proposal}
      <!-- The window card says it all: nothing else competes with it. -->
    {:else if empty}
      <div class="empty">
        {#if stop}
          <b>{t.noKey(info.label)}</b>
          <p>{stop.text} {t.orLocal}</p>
          <div class="row">
            <button type="button" class="btn sm primary" onclick={addKey}>{i18n.t.hub.addKey}</button>
            <button type="button" class="btn sm" onclick={() => (picking = true)}>{t.changeModel}</button>
          </div>
        {:else}
          <b>{t.ask}</b>
          <p>{destination(info)}</p>
          <div class="row">
            {#each t.suggestions as text (text)}
              <button type="button" class="suggestion" onclick={() => suggest(text)}>{text}</button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <ol>
        {#each chat.messages as message (message.id)}
          {#if message.role === "user"}
            <li class="mine">
              <div class="bubble">
                {#if message.file || message.window}
                  <span class="joined">
                    {#if message.file}<span><HubIcon name="clip" size={12} />{message.file}</span>{/if}
                    {#if message.window}<span><HubIcon name="window" size={12} />{message.window}</span>{/if}
                  </span>
                {/if}
                {#if message.text}<p>{message.text}</p>{/if}
              </div>
            </li>
          {:else}
            {@const shown = translations.shown(message.id, message.text)}
            <li class="answer translate-host" class:pending={message.pending}>
              <span class="by"
                >{message.by}{#if !message.pending && message.text}<TranslateButton id={message.id} text={message.text} reveal />{/if}</span
              >
              {#if message.text}
                {#key shown !== message.text}
                <div class="text" in:swap>
                  {#each blocks(shown) as block, index (index)}
                    {#if block.kind === "code"}
                      <pre><code>{block.text}</code></pre>
                    {:else}
                      <p class={block.kind}>
                        {#if block.kind === "item"}<span class="mark">{block.mark}</span>{/if}<span class="line"
                          >{#each block.spans as span, at (at)}{#if span.code}<code>{span.text}</code>{:else if span.bold}<strong
                                >{span.text}</strong
                              >{:else}{span.text}{/if}{/each}</span
                        >
                      </p>
                    {/if}
                  {/each}
                </div>
                {/key}
              {:else if message.pending}
                <p class="waiting" role="status">{t.thinking}</p>
              {/if}
              {#if message.error}
                <p class="err" role="alert">{message.error}</p>
              {:else if message.stopped}
                <p class="note">{t.stopped}</p>
              {/if}
              {#if !message.pending && message.text}
                <button type="button" class="copy" onclick={() => void copy(message.id, shown)}>
                  {copied === message.id ? t.copied : t.copy}
                </button>
              {/if}
            </li>
          {/if}
        {/each}
      </ol>
    {/if}
  </div>

  <div class="composer">
    {#if chat.attachment || chat.window}
      <div class="chips">
        {#if chat.attachment}
          <span class="chip" title={fileNote(chat.attachment)}>
            <HubIcon name="clip" size={12} />
            <span class="name">{chat.attachment.name}</span>
            <button type="button" aria-label={i18n.t.hub.remove(chat.attachment.name)} onclick={() => chat.detach()}>
              <HubIcon name="cross" size={12} />
            </button>
          </span>
        {/if}
        {#if chat.window}
          <span class="chip" title={t.windowTip}>
            <HubIcon name="window" size={12} />
            <span class="name">{chat.window.title}</span>
            <button type="button" aria-label={t.removeWindow(chat.window.title)} onclick={() => chat.removeWindow()}>
              <HubIcon name="cross" size={12} />
            </button>
          </span>
        {/if}
      </div>
    {/if}
    <div class="field" class:listening={chat.listening}>
      <textarea
        rows="1"
        bind:this={field}
        bind:value={chat.draft}
        {onkeydown}
        disabled={stop !== null}
        aria-label={t.field}
        placeholder={stop ? t.needKey : chat.listening ? t.listening : t.placeholder}
      ></textarea>
      {#if chat.canDictate}
        <button
          type="button"
          class="round mic"
          aria-pressed={chat.listening}
          aria-label={chat.listening ? t.stopDictation : t.dictate}
          title={chat.listening ? t.stopDictation : t.dictateTip}
          disabled={stop !== null}
          onclick={() => void chat.toggleDictation()}
        >
          <HubIcon name="mic" />
        </button>
      {/if}
      {#if chat.thinking}
        <button type="button" class="round stop" aria-label={t.stopAnswer} title={t.stop} onclick={() => chat.stop()}>
          <HubIcon name="stop" />
        </button>
      {:else}
        <button
          type="button"
          class="round send"
          aria-label={t.send}
          title={t.sendTip}
          disabled={!ready}
          onclick={() => void chat.send()}
        >
          <HubIcon name="send" />
        </button>
      {/if}
    </div>
    {#if chat.heard}
      <p class="heard" aria-live="polite">{chat.heard}…</p>
    {:else if chat.dictationError}
      <p class="heard err" role="alert">{chat.dictationError}</p>
    {/if}
  </div>
</div>

<style>
  .chat {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-height: 0;
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: calc(var(--s8) + var(--s1));
  }

  .me {
    flex: none;
    width: 36px;
    height: 36px;
  }

  .model-button {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 32px;
    padding: 0 var(--s2) 0 var(--s3);
    border-radius: var(--r-pill);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink-2);
    transition:
      background-color var(--t-quick),
      box-shadow var(--t-quick);
  }

  .model-button:hover,
  .model-button[aria-expanded="true"] {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }

  .model-button b {
    color: var(--ink);
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .model-button .id {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 500 var(--fs-2) var(--font-mono);
  }

  .icon {
    margin-left: auto;
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

  .icon:hover:not(:disabled) {
    background: var(--surface);
    color: var(--ink);
  }

  .icon:disabled {
    opacity: var(--dim);
    cursor: default;
  }

  .thread {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 var(--s1);
    outline: none;
    transition: opacity var(--t-quick) var(--ease-out);
    /* What scrolls away fades out instead of being cut. */
    mask-image: linear-gradient(to bottom, transparent, var(--opaque) var(--s2), var(--opaque) calc(100% - var(--s2)), transparent);
  }

  /* The picker stands out: what it covers steps back. */
  .picking .thread,
  .picking > :global(.card) {
    opacity: 0.18;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--s1);
    padding: 0 var(--s2);
  }

  .empty b {
    font-size: var(--fs-title);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .empty p {
    max-width: 52ch;
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  .suggestion {
    height: 30px;
    padding: 0 var(--s3);
    border-radius: var(--r-pill);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--ink-2);
    font-size: var(--fs-2);
    transition:
      background-color var(--t-quick),
      color var(--t-quick);
  }

  .suggestion:hover {
    background: var(--surface);
    color: var(--ink);
  }

  ol {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: var(--s2) 0;
  }

  .mine {
    display: flex;
    justify-content: flex-end;
  }

  .bubble {
    max-width: 78%;
    padding: var(--s2) var(--s3);
    border-radius: var(--r-card) var(--r-card) var(--s1) var(--r-card);
    background: var(--accent-soft);
    font-size: var(--fs-body);
    line-height: 1.45;
    user-select: text;
  }

  .bubble p {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .joined {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s2);
    margin-bottom: var(--s1);
    color: var(--accent-text);
    font-size: var(--fs-2);
  }

  .joined span {
    display: inline-flex;
    align-items: center;
    gap: var(--s1);
  }

  .answer {
    position: relative;
    max-width: 92%;
    padding-right: var(--s8);
  }

  .by {
    display: flex;
    align-items: center;
    gap: var(--s1);
    margin-bottom: 2px;
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
  }

  .text {
    font-size: var(--fs-body);
    line-height: 1.55;
    user-select: text;
  }

  .text p + p,
  .text p + pre,
  .text pre + p {
    margin-top: var(--s2);
  }

  .text p.item {
    display: flex;
    gap: var(--s2);
    margin-top: var(--s1);
  }

  .line {
    min-width: 0;
  }

  .text p.title {
    font-weight: 600;
  }

  .mark {
    flex: none;
    min-width: 1.2em;
    color: var(--ink-3);
  }

  .text code {
    padding: 1px 5px;
    border-radius: var(--r-chip);
    background: var(--surface-2);
    font: 500 0.88em var(--font-mono);
  }

  .text pre {
    padding: var(--s2) var(--s3);
    overflow-x: auto;
    border-radius: var(--r-ctl);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .text pre code {
    padding: 0;
    background: none;
    font-size: var(--fs-2);
  }

  /* While the answer streams, a caret follows the last word. */
  .pending .text > :last-child::after {
    content: "";
    display: inline-block;
    width: 2px;
    height: 1em;
    margin-left: 2px;
    vertical-align: -2px;
    background: var(--accent-text);
    animation: caret 1s steps(2) infinite;
  }

  @keyframes caret {
    50% {
      opacity: 0;
    }
  }

  .waiting,
  .note {
    color: var(--ink-3);
    font-size: var(--fs-3);
  }

  .err {
    color: var(--err);
    font-size: var(--fs-3);
  }

  .copy {
    position: absolute;
    top: 0;
    right: 0;
    padding: 0 var(--s2);
    height: 22px;
    border-radius: var(--r-chip);
    color: var(--ink-3);
    font-size: var(--fs-1);
    opacity: 0;
    transition:
      opacity var(--t-quick),
      background-color var(--t-quick);
  }

  .answer:hover .copy,
  .copy:focus-visible {
    opacity: 1;
  }

  .copy:hover {
    background: var(--surface);
    color: var(--ink);
  }

  .composer {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1);
  }

  .chip {
    max-width: 48%;
    display: inline-flex;
    align-items: center;
    gap: var(--s1);
    height: 26px;
    padding: 0 var(--s1) 0 var(--s2);
    border-radius: var(--r-pill);
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: var(--fs-2);
  }

  .chip .name {
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

  .field {
    display: flex;
    align-items: flex-end;
    gap: var(--s1);
    padding: var(--s1) var(--s1) var(--s1) var(--s3);
    border-radius: var(--r-bar);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow var(--t-quick);
  }

  .field:focus-within {
    box-shadow: inset 0 0 0 1px var(--line-2), 0 0 0 3px var(--accent-soft);
  }

  textarea {
    flex: 1;
    min-height: 32px;
    padding: 6px 0;
    border: 0;
    outline: none;
    resize: none;
    background: none;
    color: var(--ink);
    font: 400 var(--fs-body) / 1.45 var(--font-ui);
    user-select: text;
  }

  textarea::placeholder {
    color: var(--ink-3);
  }

  textarea:disabled {
    cursor: not-allowed;
  }

  .round {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    transition:
      background-color var(--t-quick),
      color var(--t-quick),
      transform var(--t-quick) var(--ease-out),
      opacity var(--t-quick);
  }

  .round:active:not(:disabled) {
    transform: scale(var(--press));
  }

  .round:disabled {
    opacity: var(--dim);
    cursor: default;
  }

  .send {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .send:disabled {
    background: var(--surface-2);
    color: var(--ink-3);
  }

  .stop {
    background: var(--surface-2);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }

  .mic {
    position: relative;
    color: var(--ink-2);
  }

  .mic:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--ink);
  }

  /* Listening: the mic takes the action color and a ring breathes around it (only its
     scale and opacity move). */
  .mic[aria-pressed="true"] {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .mic[aria-pressed="true"]::after {
    content: "";
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--accent-soft);
    animation: listen 1.6s ease-in-out infinite;
    pointer-events: none;
  }

  @keyframes listen {
    50% {
      opacity: 0.2;
      transform: scale(1.12);
    }
  }

  .heard {
    padding: 0 var(--s3);
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .pending .text > :last-child::after,
    .mic[aria-pressed="true"]::after {
      animation: none;
    }

    .round:active:not(:disabled) {
      transform: none;
    }
  }
</style>
