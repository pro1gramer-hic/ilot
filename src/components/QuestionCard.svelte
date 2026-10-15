<!-- An agent's questions (Claude Code's `AskUserQuestion`), opened in the island or in the
     Agents tab: one question at a time, its choices (one, or several), an answer of the
     user's own when the agent takes one, then « Suivant » or « Envoyer », or « Répondre
     dans le terminal ». Like a permission, each question needs a look before an answer
     counts and must be seen whole; the card is made again for each question (no choice,
     focus or scroll carries over), and nothing is ever chosen for the user. Without an
     answer in time, the agent asks in its terminal. -->
<script lang="ts">
  import { AI } from "../lib/agents";
  import { i18n } from "../lib/i18n/i18n.svelte";
  import type { PermissionAnswer, Surface } from "../lib/island/guard";
  import { rise } from "../lib/motion.svelte";
  import { type Question, answerOf, stepLabel } from "../lib/questions";
  import { reveal } from "../lib/reveal";
  import { ANSWER_MARGIN_MS, type PermissionRequest, permissions } from "../lib/stores/permissions.svelte";
  import Icon from "./Icon.svelte";

  let {
    request,
    waiting,
    surface = "panel",
    answer = null,
  }: {
    request: PermissionRequest & { questions: Question[] };
    waiting: number;
    surface?: Surface;
    answer?: PermissionAnswer | null;
  } = $props();

  const t = $derived(i18n.t.island.question);
  const shared = $derived(i18n.t.island.request);
  const draft = $derived(permissions.draft(request.id));
  const total = $derived(request.questions.length);
  const step = $derived(Math.min(draft.step, total - 1));
  const question = $derived(request.questions[step]);
  const last = $derived(step === total - 1);
  const ready = $derived(answerOf(draft, question) !== null);
  const late = $derived(permissions.isLate(request.id));
  const tool = $derived(AI[request.source].tool);

  let body = $state<HTMLElement>();
  let readToEnd = $state(false);
  $effect(() => {
    const element = body;
    if (!element) return;
    readToEnd = false;
    const measure = (): void => {
      if (element.clientHeight > 0 && element.scrollTop + element.clientHeight >= element.scrollHeight - 2) readToEnd = true;
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    element.addEventListener("scroll", measure, { passive: true });
    return () => {
      observer.disconnect();
      element.removeEventListener("scroll", measure);
    };
  });
  const whole = $derived(readToEnd && !request.truncated && !late);
  $effect(() => permissions.guard.setWhole(surface, request.id, whole));

  /** Time left before the agent asks in its terminal. */
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });
  const left = $derived(Math.max(0, Math.ceil((request.receivedAt + request.timeoutMs - ANSWER_MARGIN_MS - now) / 1000)));

  /** Enter on a choice or in the typed answer validates the question on screen. */
  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== "Enter" || event.repeat || (event.target instanceof HTMLElement && event.target.closest("button"))) return;
    event.preventDefault();
    event.stopPropagation();
    permissions.submitQuestion(request.id);
  }

  const picked = (label: string): boolean => draft.picked.includes(label);
</script>

<!-- A new card for each question. -->
{#key step}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <article class="card {surface}" class:answered={answer !== null} aria-labelledby="question-title-{surface}" {onkeydown}>
    <header>
      {#if surface === "island"}
        <span class="slot" aria-hidden="true"></span>
      {/if}
      <div class="titles">
        <span class="caps" in:rise|global={{ index: 0 }}>
          <i class="dot" style:--c={AI[request.source].color}></i>{shared.where(tool, request.folder ?? request.project)}
        </span>
        <h2 id="question-title-{surface}" in:rise|global={{ index: 1 }}>{request.title}</h2>
      </div>
      {#if total > 1}
        <span class="count" in:rise|global={{ index: 1 }}>{stepLabel(step, total)}</span>
      {/if}
    </header>

    <!-- The question scrolls: the keyboard must reach it to read it to the end. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="body" bind:this={body} tabindex="0" aria-label={t.body} in:rise|global={{ index: 2 }}>
      {#if question.header}<span class="chip">{question.header}</span>{/if}
      <p class="question" id="question-text-{surface}">{#each reveal(question.question) as part, index (index)}{#if part.kind === "text"}{part.value}{:else}<span class="hidden">{part.kind === "newline" ? "↵" : part.label}</span>{/if}{/each}</p>
      <div class="choices" role={question.multiSelect ? "group" : "radiogroup"} aria-labelledby="question-text-{surface}">
        {#each question.options as option (option.label)}
          <label class="choice" class:on={picked(option.label)}>
            <input
              type={question.multiSelect ? "checkbox" : "radio"}
              name="question-{surface}-{request.id}-{step}"
              checked={picked(option.label)}
              disabled={answer !== null}
              onchange={() => permissions.pick(request.id, option.label)}
            />
            <span class="mark" class:square={question.multiSelect} aria-hidden="true">
              {#if picked(option.label)}<Icon name="check" size={10} />{/if}
            </span>
            <span class="text">
              <b>{#each reveal(option.label) as part, index (index)}{#if part.kind === "text"}{part.value}{:else}<span class="hidden">{part.kind === "newline" ? "↵" : part.label}</span>{/if}{/each}</b>
              {#if option.description}<small>{option.description}</small>{/if}
            </span>
          </label>
        {/each}
        {#if question.freeText}
          <input
            class="other"
            class:on={draft.other.trim() !== ""}
            type="text"
            maxlength="500"
            aria-label={t.other}
            placeholder={t.otherHint}
            value={draft.other}
            disabled={answer !== null}
            oninput={(event) => permissions.type(request.id, event.currentTarget.value)}
          />
        {/if}
      </div>
    </div>

    <div class="notes" in:rise|global={{ index: 3 }}>
      {#if late}
        <p class="warn">{shared.late}</p>
      {:else if request.truncated}
        <p class="warn">{t.truncated}</p>
      {:else if !readToEnd}
        <p class="warn">{t.readToEnd}</p>
      {:else if question.multiSelect}
        <p class="hint">{t.several}</p>
      {/if}
    </div>

    <div class="buttons" inert={answer !== null} in:rise|global={{ index: 4 }}>
      <button type="button" class="btn" class:chosen={answer === "terminal"} disabled={late} onclick={() => permissions.respond(request.id, "terminal")}>
        {shared.terminal}{#if surface === "island"}<kbd>{i18n.t.island.keyEscape}</kbd>{/if}
      </button>
      <button
        type="button"
        class="btn primary yes"
        class:chosen={answer === "allow"}
        disabled={(!ready || !whole) && answer !== "allow"}
        onclick={() => permissions.submitQuestion(request.id)}
      >
        {#if answer === "allow"}
          <span class="done"><Icon name="check" size={12} /></span>{t.sent}
        {:else}
          {last ? t.send : t.next}{#if surface === "island"}<kbd>{i18n.t.island.keyEnter}</kbd>{/if}
        {/if}
      </button>
    </div>
    <p class="scope">
      {t.scope(left, AI[request.source].name, waiting)}
    </p>
  </article>
{/key}

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  /* In the island, the opening's own padding; in the panel, a card among others. */
  .card.island {
    padding: var(--s4) var(--s5);
  }

  .card.panel {
    padding: var(--s4);
    background: var(--surface);
    border: 1px solid color-mix(in srgb, var(--warn) 40%, var(--line));
    border-radius: var(--r-card);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }

  /* Mochi floats over this slot in the island (IslandMochi). */
  .slot {
    width: 44px;
    height: 44px;
    flex: none;
  }

  .titles {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }

  .caps {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 1px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dot {
    width: var(--dot);
    height: var(--dot);
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }

  h2 {
    font-size: var(--fs-4);
    font-weight: 600;
    letter-spacing: -0.005em;
  }

  .count {
    flex: none;
    color: var(--ink-2);
    font-size: var(--fs-2);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* The whole question scrolls in one box; answering waits until its end was on screen. */
  /* As tall as the opening allows (MAX_OPEN_HEIGHT): three choices and a typed answer fit
     without scrolling; a fourth choice scrolls, and must be read to the end. */
  .body {
    max-height: 272px;
    margin-top: var(--s3);
    padding: 10px var(--s3);
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
  }

  .card.panel .body {
    background: var(--surface-2);
  }

  .body:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .chip {
    display: inline-block;
    margin-bottom: var(--s1);
    padding: 1px var(--s2);
    border-radius: var(--r-chip);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--ink-2);
    font-size: var(--fs-1);
    font-weight: 600;
  }

  .question {
    color: var(--ink);
    font-size: var(--fs-body);
    font-weight: 500;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .choices {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    margin-top: var(--s2);
  }

  /* A choice: the whole row is clickable, the native input stays for the keyboard and
     screen readers, drawn over by its mark. */
  .choice {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: var(--s2);
    padding: 6px var(--s2);
    border-radius: var(--r-ctl);
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: pointer;
    transition:
      background-color var(--t-quick),
      box-shadow var(--t-quick);
  }

  .choice:hover {
    background: var(--hover);
  }

  .choice.on,
  .other.on {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 70%, transparent);
    background: var(--accent-soft);
  }

  .choice input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .choice:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .mark {
    width: 16px;
    height: 16px;
    flex: none;
    margin-top: 1px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--line-2);
    color: var(--accent-ink);
    transition: background-color var(--t-quick);
  }

  .mark.square {
    border-radius: 5px;
  }

  .choice.on .mark {
    background: var(--accent);
    box-shadow: none;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .text b {
    color: var(--ink);
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .text small {
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.4;
  }

  /* An answer of the user's own: one line, the same frame as a choice. */
  .other {
    width: 100%;
    height: 34px;
    padding: 0 var(--s2);
    border: 0;
    border-radius: var(--r-ctl);
    box-shadow: inset 0 0 0 1px var(--line);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-3);
    transition: box-shadow var(--t-quick);
  }

  .other::placeholder {
    color: var(--ink-3);
  }

  .other:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .hidden {
    padding: 0 3px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--warn) 20%, transparent);
    color: var(--warn);
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: var(--s2);
  }

  .notes:empty {
    display: none;
  }

  .warn {
    color: var(--warn);
    font-size: var(--fs-2);
  }

  .hint {
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  .buttons .btn {
    position: relative;
    flex: 1 1 auto;
  }

  .buttons .yes {
    flex: 1.2;
  }

  /* The chosen answer lights up while the island folds. */
  .btn::before {
    content: "";
    position: absolute;
    inset: -3px;
    border-radius: calc(var(--r-ctl) + 3px);
    box-shadow: 0 0 14px 2px color-mix(in srgb, var(--accent) 70%, transparent);
    opacity: 0;
    transition: opacity var(--t-quick);
    pointer-events: none;
  }

  .btn.chosen::before {
    opacity: 1;
  }

  .done {
    display: grid;
    animation: pop var(--t-enter) var(--ease-pop) both;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
  }

  kbd {
    margin-left: 2px;
    font: 500 var(--fs-1) var(--font-ui);
    opacity: 0.7;
  }

  .scope {
    margin-top: var(--s2);
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .done {
      animation-name: appear;
    }

    @keyframes appear {
      from {
        opacity: 0;
      }
    }
  }
</style>
