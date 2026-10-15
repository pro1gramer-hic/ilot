<!-- The full permission request (docs/09-DEMO-FINALE.md, section 4), opened in the island
     or in the Agents tab: the agent and its folder, the exact command in mono with hidden
     characters made visible, what a file tool will write, the risk and its warnings, the
     reason, and the answers (« Autoriser » is the main one, never « Toujours »). Allowing
     needs the whole request read: a long one must be scrolled to its end, a cut one is
     answered in the terminal. Once answered, the card plays its answer for a moment (the
     chosen button lights up, or the card shakes), then the island folds. -->
<script lang="ts">
  import { AI } from "../lib/agents";
  import { i18n } from "../lib/i18n/i18n.svelte";
  import type { PermissionAnswer, Surface } from "../lib/island/guard";
  import { rise } from "../lib/motion.svelte";
  import { looksSpoofed, reveal } from "../lib/reveal";
  import { type PermissionRequest, permissions } from "../lib/stores/permissions.svelte";
  import Icon from "./Icon.svelte";

  let {
    request,
    waiting,
    surface = "panel",
    answer = null,
  }: { request: PermissionRequest; waiting: number; surface?: Surface; answer?: PermissionAnswer | null } = $props();

  const t = $derived(i18n.t.island.permission);
  const shared = $derived(i18n.t.island.request);
  const segments = $derived(reveal(request.detail));
  const lines = $derived(
    (request.content ?? "").split("\n").filter((line, index, all) => line !== "" || index < all.length - 1),
  );
  const flagged = $derived(
    segments.some((s) => s.kind !== "text") ||
      lines.some((line) => reveal(line.slice(1)).some((s) => s.kind === "hidden")),
  );

  let body = $state<HTMLElement>();
  let readToEnd = $state(false);
  $effect(() => {
    const element = body;
    if (!element) return;
    const measure = (): void => {
      if (element.clientHeight > 0 && element.scrollTop + element.clientHeight >= element.scrollHeight - 2) {
        readToEnd = true;
      }
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
  const spoofed = $derived(looksSpoofed(request.detail) || looksSpoofed(request.content ?? ""));
  const late = $derived(permissions.isLate(request.id));
  const risky = $derived(request.risk === "high");
  const canAllow = $derived(readToEnd && !request.truncated && !late);
  $effect(() => permissions.guard.setWhole(surface, request.id, canAllow));

  /** "à l'instant", "il y a 12 s", "il y a 1 min". */
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });
  const age = $derived.by(() => {
    const seconds = Math.max(0, Math.floor((now - request.receivedAt) / 1000));
    if (seconds < 2) return t.askedNow;
    return seconds < 60 ? t.askedSeconds(seconds) : t.askedMinutes(Math.floor(seconds / 60));
  });
</script>

<article
  class="card {surface}"
  class:risky
  class:denied={answer === "deny"}
  aria-labelledby="permission-title-{surface}"
  aria-describedby="permission-command-{surface}"
>
  <header>
    {#if surface === "island"}
      <span class="slot" aria-hidden="true"></span>
    {/if}
    <div class="titles">
      <span class="caps" in:rise|global={{ index: 0 }}>
        <i class="dot" style:--c={AI[request.source].color}></i>{shared.where(AI[request.source].tool, request.folder ?? request.project)}
      </span>
      <h2 id="permission-title-{surface}" in:rise|global={{ index: 1 }}>{request.title}</h2>
    </div>
    <span class="risk" class:high={risky} in:rise|global={{ index: 1 }}>{risky ? t.riskHigh : t.riskLow}</span>
  </header>
  <!-- The request scrolls: the keyboard must reach it to read it to the end. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="body" bind:this={body} tabindex="0" aria-label={t.body} in:rise|global={{ index: 2 }}>
    <pre class="command mono" id="permission-command-{surface}">{#each segments as part, index (index)}{#if part.kind === "text"}{part.value}{:else if part.kind === "newline"}<span class="hidden">↵</span>
{:else}<span class="hidden">{part.label}</span>{/if}{/each}</pre>
    {#if lines.length > 0}
      <div class="content mono">
        {#each lines as line, row (row)}
          <div class:del={line.startsWith("-")} class:add={line.startsWith("+")}>{#each reveal(line.slice(1)) as part, index (index)}{#if part.kind === "text"}{part.value}{:else if part.kind === "hidden"}<span class="hidden">{part.label}</span>{/if}{/each}</div>
        {/each}
      </div>
    {/if}
    {#if request.extras.length > 0}
      <ul class="extras mono" aria-label={t.extras}>
        {#each request.extras as extra (extra)}
          <li>{extra}</li>
        {/each}
      </ul>
    {/if}
  </div>
  {#each request.warnings ?? [] as warning (warning)}
    <p class="warning" in:rise|global={{ index: 3 }}><Icon name="warn" size={13} />{warning}</p>
  {/each}
  <div class="notes" in:rise|global={{ index: 3 }}>
    {#if request.extras.length > 0}
      <p class="warn">{t.extrasNote}</p>
    {/if}
    {#if spoofed}
      <p class="warn">{t.spoofed}</p>
    {/if}
    {#if flagged}
      <p class="warn">{t.flagged}</p>
    {/if}
    {#if late}
      <p class="warn">{shared.late}</p>
    {:else if request.truncated}
      <p class="warn">{t.truncated}</p>
    {:else if !readToEnd}
      <p class="warn">{t.readToEnd}</p>
    {/if}
    {#if request.reason}
      <p class="why">{t.reason(AI[request.source].name, request.reason)}</p>
    {/if}
  </div>
  <div class="buttons" inert={answer !== null} in:rise|global={{ index: 4 }}>
    {#if request.truncated}
      <button type="button" class="btn" disabled={late} onclick={() => permissions.respond(request.id, "deny")}>
        {t.deny}{#if surface === "island"}<kbd>{i18n.t.island.keyEscape}</kbd>{/if}
      </button>
      <button
        type="button"
        class="btn primary"
        class:chosen={answer === "terminal"}
        disabled={late}
        onclick={() => permissions.respond(request.id, "terminal")}
      >
        {shared.terminal}
      </button>
    {:else}
      <button type="button" class="btn" disabled={late} onclick={() => permissions.respond(request.id, "deny")}>
        {t.deny}{#if surface === "island"}<kbd>{i18n.t.island.keyEscape}</kbd>{/if}
      </button>
      <button
        type="button"
        class="btn"
        class:chosen={answer === "always"}
        disabled={!request.canAlways || !canAllow}
        title={request.canAlways ? t.alwaysTip : request.source === "codex" ? t.alwaysCodex : t.alwaysNever}
        onclick={() => permissions.respond(request.id, "always")}
      >
        {t.always}
      </button>
      <button
        type="button"
        class="btn primary yes"
        class:chosen={answer === "allow"}
        disabled={!canAllow && answer !== "allow"}
        onclick={() => permissions.respond(request.id, "allow")}
      >
        {#if answer === "allow"}
          <span class="done"><Icon name="check" size={12} /></span>{t.allowed}
        {:else}
          {t.allow}{#if surface === "island"}<kbd>{i18n.t.island.keyEnter}</kbd>{/if}
        {/if}
      </button>
    {/if}
  </div>
  <p class="scope">
    {t.scope(age, waiting)}
  </p>
</article>

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

  .card.denied {
    animation: shake var(--t-shake) var(--ease-out);
  }

  @keyframes shake {
    20% {
      transform: translateX(-5px);
    }
    40% {
      transform: translateX(4px);
    }
    60% {
      transform: translateX(-3px);
    }
    80% {
      transform: translateX(2px);
    }
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

  .risk {
    flex: none;
    padding: 4px 10px;
    border: 1px solid color-mix(in srgb, var(--ok) 30%, transparent);
    border-radius: var(--r-pill);
    color: var(--ok);
    font-size: var(--fs-1);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .risk.high {
    border-color: transparent;
    background: var(--warn);
    color: var(--risk-ink);
  }

  /* The whole request scrolls in one box; allowing waits until its end was on screen. */
  .body {
    max-height: 156px;
    margin-top: var(--s3);
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
  }

  .body:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .command,
  .content {
    padding: 10px 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .command {
    color: var(--ink);
    font-size: var(--fs-sub);
    font-weight: 500;
    line-height: 1.55;
  }

  .content {
    border-top: 1px solid var(--line);
    font-size: var(--fs-2);
  }

  /* Each line keeps its mark in a gutter, wrapped text included. */
  .content > div {
    min-height: 1lh;
    padding-left: 14px;
    text-indent: -14px;
  }

  .content > div::before {
    content: "  ";
  }

  .content > .del {
    color: var(--err);
  }

  .content > .del::before {
    content: "− ";
  }

  .content > .add {
    color: var(--ok);
  }

  .content > .add::before {
    content: "+ ";
  }

  .hidden {
    padding: 0 3px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--warn) 20%, transparent);
    color: var(--warn);
  }

  .extras {
    list-style: none;
    padding: 6px 12px;
    border-top: 1px solid var(--line);
    color: var(--warn);
    font-size: var(--fs-2);
    overflow-wrap: anywhere;
  }

  /* What makes the command risky, one box per reason. */
  .warning {
    display: flex;
    align-items: flex-start;
    gap: var(--s2);
    margin-top: var(--s2);
    padding: 8px 10px;
    border: 1px solid var(--warn-line);
    border-radius: var(--r-ctl);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: var(--fs-sub);
    line-height: 1.45;
  }

  .warning :global(svg) {
    margin-top: 2px;
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

  .why {
    color: var(--ink-2);
    font-size: var(--fs-sub);
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
    .card.denied {
      animation: none;
    }

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
