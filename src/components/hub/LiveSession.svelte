<!-- A session followed live (09, section 8 bis), the hub grown to 720 × 420. On the left,
     Mochi, the session and its four steps (Lire, Éditer, Commande, Terminé: a turning ring
     on the current one, a green check on those done). On the right, what the agent does,
     in an editor's frame: îlot never reads the agent's files, so the app shows the file
     it works on and its latest actions, and a terminal slides up while it runs a command.
     The browser preview stages the demo instead (fake code read line by line, a line
     struck in red, the new one typed in green, the tests passing). At the end, Mochi
     turns green and celebrates with pixel confetti, then the hub goes back home. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { AgentSession } from "../../lib/agents";
  import {
    type HubSession,
    LIVE_MS,
    LIVE_STEPS,
    type StepState,
    frameSteps,
    liveFrame,
    liveSteps,
    stepCount,
    stepOfActivity,
  } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type FileDiff, sessionDiff } from "../../lib/ipc";
  import { island } from "../../lib/island/controller.svelte";
  import { reduced, stillness } from "../../lib/motion.svelte";
  import { changes } from "../../lib/stores/changes.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import { goToTerminal } from "../../lib/terminal";
  import { clockLabel, elapsedLabel, laneOf } from "../../lib/timeline";
  import Subagents from "../agents/Subagents.svelte";
  import Code from "./Code.svelte";
  import HubIcon from "./HubIcon.svelte";
  import HubMochi from "./HubMochi.svelte";

  let { followed, onburst }: { followed: HubSession & { kind: "agent" }; onburst: (x: number, y: number) => void } = $props();

  const words = $derived(i18n.t.hub.live);
  const session = $derived<AgentSession>(followed.session);
  const stage = $derived(island.hubStage?.sessionId === session.id ? island.hubStage : null);

  // ---------- the preview's staged run: a clock that plays it once ----------
  let t = $state(0);
  $effect(() => {
    if (!stage || stillness.current) return;
    let frame = 0;
    let last = performance.now();
    const tick = (now: number): void => {
      t += now - last;
      last = now;
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
  const staged = $derived(stage ? liveFrame(stage, t, reduced()) : null);

  // ---------- the real session ----------
  /** Its latest change, from its hooks' payload: fetched again each time it changes,
   * never stored. */
  let diff = $state<FileDiff | null>(null);
  $effect(() => {
    const id = session.id;
    void changes.versions[id];
    let current = true;
    void sessionDiff(id).then((latest) => {
      if (current) diff = latest;
    });
    return () => {
      current = false;
    };
  });
  const showsDiff = $derived(!stage && diff !== null && island.liveTab === "diff");

  const lane = $derived(laneOf(timeline.value, session.source));
  /** Oldest first, the current action last. */
  const journal = $derived([...lane.steps].reverse());
  const step = $derived(session.status === "active" ? stepOfActivity(session.activity) : null);
  const target = $derived(session.activity?.target);
  const elapsed = $derived(timeline.now - (timeline.value.since[session.id] ?? timeline.now));
  const failure = $derived(lane.marks.filter((mark) => mark.kind === "failed").at(-1)?.label ?? session.task);

  const states = $derived<StepState[]>(staged ? frameSteps(staged) : liveSteps(session, lane.steps.map((s) => s.text)));
  const done = $derived(staged ? staged.done : session.status === "done");
  const running = $derived(staged ? staged.terminalOpen : step === 2 || session.status === "done" || session.status === "failed");

  // The end: a green Mochi, confetti, then back home.
  let side = $state<HTMLElement>();
  let partying = $state(false);
  let seenDone: boolean | undefined;
  $effect(() => {
    const finished = done;
    return untrack(() => {
      const before = seenDone;
      seenDone = finished;
      if (before !== false || !finished) return;
      partying = true;
      const box = side?.getBoundingClientRect();
      if (box) onburst(box.left + box.width / 2, box.top + 40);
      if (stage) return;
      const timer = setTimeout(() => island.showHub("home"), LIVE_MS.party);
      return () => clearTimeout(timer);
    });
  });
  let over = false;
  $effect(() => {
    if (!staged?.over || over) return;
    over = true;
    untrack(() => {
      stage?.finish?.();
      island.showHub("home");
    });
  });

  const move = $derived(
    partying ? "party" : staged ? (["watch", "breathe", "rock", "party"] as const)[staged.step] : step === 0 ? "watch" : undefined,
  );
  const status = $derived(partying ? "ok" : staged ? "work" : followed.status);
</script>

<div class="live">
  <div class="bar">
    <button type="button" class="back" aria-label={i18n.t.hub.back} title={i18n.t.hub.back} onclick={() => island.showHub("home")}>
      <HubIcon name="back" />
    </button>
    <h2>{session.project}, {followed.tool}</h2>
    <button type="button" class="go" title={words.toTerminal} onclick={() => void goToTerminal(session.id)}>
      <HubIcon name="term" size={14} />{i18n.t.hub.goToTerminal}
    </button>
    <span class="count">{stepCount(states)}</span>
  </div>
  <div class="grid">
    <aside class="side" bind:this={side}>
      <HubMochi palette={partying ? "menthe" : followed.palette} {status} {move} face={partying ? "star" : undefined} />
      <b>{session.project}</b>
      <small>{followed.tool}</small>
      {#if session.subagents}<Subagents subagents={session.subagents} />{/if}
      <ol class="steps" aria-label={words.steps}>
        {#each LIVE_STEPS as name, index (name)}
          <li class={states[index]} aria-current={states[index] === "now" ? "step" : undefined}>
            <span class="ring" aria-hidden="true">
              {#if states[index] === "done"}<HubIcon name="check" size={10} />{/if}
            </span>
            {words.stepNames[index]}{#if states[index] !== "todo"}<span class="visually-hidden">, {states[index] === "done" ? words.stepDone : words.stepNow}</span>{/if}
          </li>
        {/each}
      </ol>
    </aside>

    <figure class="editor" aria-label={stage ? words.file(stage.file) : words.journalOf(followed.tool)}>
      {#if stage && staged}
        <div class="tab">
          <span class="lang">{stage.lang}</span>
          <span>{stage.file}</span>
          <i class="modified" class:on={staged.edits.some((edit) => edit.struck)} aria-hidden="true"></i>
          <span class="path">{stage.path}</span>
        </div>
        <div class="code">
          {#each stage.lines as line (line.n)}
            {@const edit = staged.edits.find((e) => e.line === line.n)}
            <div class="line" class:reading={staged.reading === line.n} class:del={edit?.struck}>
              <span class="n">{line.n}</span><span class="t"><Code tokens={line.tokens} /></span>
            </div>
            {#if edit && edit.typed !== null}
              <div class="line add"><span class="n">{line.n}</span><span class="t">{edit.typed}</span></div>
            {/if}
          {/each}
        </div>
        <div class="term" class:on={staged.terminalOpen} role="log" aria-label={words.terminal}>
          {#each stage.terminal.slice(0, staged.terminalLines) as line, index (index)}
            <div class="tone-{line.tone ?? 'plain'}">
              {#if line.tone === "prompt"}<span class="p">$</span>
              {:else if line.tone === "pass"}<span class="pass">PASS</span>{/if}
              {line.text}
            </div>
          {/each}
        </div>
      {:else}
        {#if diff}
          <!-- The agent's journal, or its latest change. -->
          <div class="tab views" role="tablist" aria-label={words.views}>
            <button type="button" role="tab" aria-selected={!showsDiff} class:on={!showsDiff} onclick={() => (island.liveTab = "journal")}>
              <HubIcon name="read" size={13} />{words.journal}
            </button>
            <button type="button" role="tab" aria-selected={showsDiff} class:on={showsDiff} onclick={() => (island.liveTab = "diff")}>
              <HubIcon name="edit" size={13} />{diff.file}
              <span class="plus">+{diff.added}</span><span class="minus">−{diff.removed}</span>
            </button>
            <span class="path">{session.project}</span>
          </div>
        {:else}
          <div class="tab">
            {#if target && step !== null && step < 2}
              <HubIcon name={step === 1 ? "edit" : "read"} size={13} />
              <span>{target}</span>
              <i class="modified" class:on={step === 1} aria-hidden="true"></i>
            {:else}
              <HubIcon name="read" size={13} />
              <span>{words.journal}</span>
            {/if}
            <span class="path">{session.project}</span>
          </div>
        {/if}
        {#if showsDiff && diff}
          <!-- What the change removed (red) and added (green), as its hook described it. -->
          <ol class="code diff" aria-label={words.change(diff.file)}>
            {#each diff.lines as line, index (index)}
              <li class="line" class:del={line.kind === "del"} class:add={line.kind === "add"} class:gap={line.kind === "gap"}>
                <span class="n"></span><span class="t">{line.kind === "gap" ? "⋯" : line.text}</span>
                {#if line.kind === "del" || line.kind === "add"}<span class="visually-hidden">, {line.kind === "del" ? words.removed : words.added}</span>{/if}
              </li>
            {/each}
            {#if diff.truncated}
              <li class="line quiet"><span class="t">{words.cut}</span></li>
            {/if}
          </ol>
        {:else}
          <ol class="code journal" aria-label={i18n.t.hub.recent}>
            {#each journal as entry, index (entry.at + entry.text)}
              <li class="line" class:reading={index === journal.length - 1 && session.status === "active"}>
                <time class="n">{clockLabel(entry.at)}</time><span class="t">{entry.text}</span>
              </li>
            {:else}
              <li class="line quiet"><span class="t">{words.noActions}</span></li>
            {/each}
          </ol>
          <div class="term" class:on={running} role="log" aria-label={words.terminal}>
            {#if session.status === "done"}
              <div><span class="pass">{words.finished}</span> {session.task}</div>
            {:else if session.status === "failed"}
              <div><span class="fail">{words.failed}</span> {failure}</div>
            {:else if step === 2}
              <div><span class="p">$</span> {target ?? ""}</div>
              <div class="dim">{words.runningFor(elapsedLabel(elapsed))}</div>
            {/if}
          </div>
        {/if}
      {/if}
    </figure>
  </div>
</div>

<style>
  .live {
    display: flex;
    flex-direction: column;
    height: 100%;
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

  h2,
  .count {
    margin: 0 calc(var(--s1) + var(--gap-mark));
    color: var(--ink-3);
    font-size: var(--fs-2);
    font-weight: 400;
    white-space: nowrap;
  }

  h2 {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .count {
    font-family: var(--font-mono);
  }

  /* « Aller au terminal »: a quiet button of the bar, like the back arrow. */
  .go {
    display: flex;
    align-items: center;
    gap: var(--s1);
    height: 30px;
    padding: 0 var(--s2);
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    font-size: var(--fs-2);
    white-space: nowrap;
    transition:
      background-color var(--t-quick),
      color var(--t-quick);
  }

  .go:hover {
    background: var(--surface);
    color: var(--ink);
  }

  .go:focus-visible,
  .views button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* The journal or the latest change: two tabs in the editor's tab row. */
  .views {
    gap: var(--s1);
    padding-left: var(--s1);
  }

  .views button {
    display: flex;
    align-items: center;
    gap: var(--s1);
    height: 26px;
    padding: 0 var(--s2);
    border-radius: var(--r-chip);
    color: var(--ink-3);
    font-size: var(--fs-sub);
    transition:
      background-color var(--t-quick),
      color var(--t-quick);
  }

  .views button:hover {
    color: var(--ink-2);
  }

  .views button.on {
    background: var(--surface-2);
    color: var(--ink);
  }

  .plus,
  .minus {
    font: 600 var(--fs-2) var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .plus {
    color: var(--ok);
  }

  .minus {
    color: var(--err);
  }

  /* The change scrolls when it is longer than the editor. */
  .code.diff {
    overflow: auto;
  }

  .line.gap {
    color: var(--ink-3);
  }

  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    gap: var(--s3);
    margin-top: calc(var(--s1) + var(--gap-mark));
  }

  .side {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--gap-mark);
    padding: var(--s3);
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    --badge-ring: var(--surface);
  }

  .side b {
    margin-top: var(--s2);
    font-size: var(--fs-body);
    font-weight: 600;
  }

  .side small {
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .steps {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    margin-top: var(--s3);
    font-size: var(--fs-sub);
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--ink-3);
    transition: color 0.25s;
  }

  .ring {
    width: 16px;
    height: 16px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1.5px solid var(--line-2);
    color: var(--risk-ink);
    transition:
      background-color 0.25s,
      border-color 0.25s;
  }

  .steps li.now {
    color: var(--ink);
  }

  .steps li.now .ring {
    border-color: var(--accent);
    border-top-color: transparent;
    animation: spin var(--t-spin) linear infinite;
  }

  .steps li.done {
    color: var(--ink-2);
  }

  .steps li.done .ring {
    border-color: var(--ok);
    background: var(--ok);
  }

  .steps li.done .ring :global(svg) {
    stroke-width: 2.4;
  }

  .editor {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    border-radius: var(--r-card);
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .tab {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 36px;
    flex: none;
    padding: 0 var(--s3);
    border-bottom: 1px solid var(--line);
    color: var(--ink-2);
    font-size: var(--fs-sub);
  }

  .tab > span:not(.path, .lang) {
    color: var(--ink);
  }

  .lang {
    padding: 0 var(--s1);
    border-radius: var(--r-chip);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--ink-2);
    font: 600 var(--fs-0) var(--font-mono);
  }

  .modified {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn);
    opacity: 0;
    transition: opacity 0.3s;
  }

  .modified.on {
    opacity: 1;
  }

  .path {
    margin-left: auto;
    color: var(--ink-3);
    font: var(--fs-2) var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .code {
    flex: 1;
    min-height: 0;
    padding: var(--s2) 0;
    overflow: hidden;
    list-style: none;
    font: var(--fs-2) / 1.75 var(--font-mono);
  }

  .line {
    display: flex;
    padding: 0 var(--s3);
    white-space: pre;
    transition: background-color 0.25s;
  }

  .journal .line {
    font-family: var(--font-ui);
    font-size: var(--fs-sub);
    line-height: 2;
  }

  .line .n {
    width: 28px;
    flex: none;
    color: var(--ink-3);
    user-select: none;
    font-family: var(--font-mono);
  }

  .journal .line .n {
    width: 52px;
    font-size: var(--fs-2);
  }

  .line .t {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .line.reading {
    background: var(--hover);
  }

  .line.quiet {
    color: var(--ink-3);
  }

  .line.del {
    background: var(--err-soft);
    color: var(--err);
  }

  .line.del .t {
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--err) 60%, transparent);
  }

  .line.del .t :global(span) {
    color: inherit;
  }

  .line.add {
    background: var(--ok-soft);
    color: var(--ok);
  }

  .line.del .n::after {
    content: " −";
  }

  .line.add .n::after {
    content: " +";
  }

  /* The terminal slides up over the bottom of the file, then stays. */
  .term {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 150px;
    padding: var(--s2) var(--s3);
    overflow: hidden;
    border-top: 1px solid var(--line);
    background: var(--bg);
    color: var(--ink-2);
    font: var(--fs-2) / 1.7 var(--font-mono);
    transform: translateY(100%);
    transition: transform 0.45s var(--ease-out);
  }

  .term.on {
    transform: none;
  }

  .term .p {
    color: var(--accent-text);
  }

  .tone-ok {
    color: var(--ok);
  }

  .tone-dim,
  .term .dim {
    color: var(--ink-3);
  }

  .pass,
  .fail {
    padding: 0 var(--s1);
    border-radius: var(--r-mark);
    background: var(--ok);
    color: var(--risk-ink);
    font-weight: 700;
  }

  .fail {
    background: var(--err);
    color: var(--ink);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .steps li.now .ring {
      animation: none;
      border-top-color: var(--accent);
    }

    .term {
      transform: none;
      opacity: 0;
      transition: opacity var(--t-enter);
    }

    .term.on {
      opacity: 1;
    }
  }
</style>
