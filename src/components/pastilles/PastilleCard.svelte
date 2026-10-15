<!-- One pastille in full (the left card of the hub's « Pastilles » view): its Mochi large in
     its colors, standing as its state goes (asleep when the tool or the server is off, a
     worried face when something fails, watching while something runs), its name, its
     line, and what it holds: the tool's project and its agents, the models of a local
     server (a click opens the chat on one, `onopenchat`) and a download started here,
     Spotify's commands and volume, or a service's numbers and latest items. A problem is
     said calmly, with « Réessayer ». Another pastille arrives out of a light blur. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { PALETTES } from "../../lib/mochi/palettes";
  import { sessionOf } from "../../lib/media";
  import {
    type LocalModel,
    agentsIn,
    formatBytes,
    lookOf,
    pullShare,
    relativeTime,
    stanceOf,
    toolLine,
    toolProject,
  } from "../../lib/pastilles";
  import { media } from "../../lib/stores/media.svelte";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import { swap } from "../hub/motion";
  import AppVolume from "../music/AppVolume.svelte";
  import Transport from "../music/Transport.svelte";

  let {
    id,
    onopenchat,
  }: {
    id: string;
    /** Opens îlot's chat on a local model (wired by the chat, lot 3). */
    onopenchat?: (model: string, provider: "ollama" | "lmstudio") => void;
  } = $props();

  /** Models listed on the card; the rest are counted. */
  const MODEL_ROWS = 2;
  const ITEM_ROWS = 2;

  const t = $derived(i18n.t.pastilles.card);
  const view = $derived(pastilles.view(id));
  const look = $derived(lookOf(id, pastilles.config.tool, pastilles.catalog));
  const tone = $derived(view?.tone ?? "idle");
  const stance = $derived(stanceOf(tone));
  const glow = $derived(PALETTES[look.palette].tiers[2]);
  const project = $derived(id === "tool" ? toolProject(view?.tool, sessions.list) : null);
  const agents = $derived(agentsIn(project, sessions.list));
  const line = $derived(id === "tool" ? toolLine(look.name, view?.tool, project, agents) : (view?.line ?? i18n.t.pastilles.reading));
  const models = $derived(view?.models ?? []);
  const shownModels = $derived([...models].sort((a, b) => Number(b.loaded) - Number(a.loaded)).slice(0, MODEL_ROWS));
  const spotify = $derived(id === "spotify" ? sessionOf(media.sessions, "Spotify") : null);
  const pull = $derived(view?.pull);
  const server = $derived(id === "ollama" || id === "lmstudio" ? id : null);

  let asking = $state(false);
  let model = $state("");

  function modelDetail(m: LocalModel): string {
    const parts = [m.loaded ? t.inMemory : formatBytes(m.size), m.details].filter(Boolean);
    return parts.join(", ");
  }

  async function download(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (await pastilles.pull(model)) {
      asking = false;
      model = "";
    }
  }
</script>

<div class="card" style:--sc={glow}>
  <span class="mochi"><Mochi palette={look.palette} kind={look.kind} move={stance.move} face={stance.face} scale={3} label="" /></span>
  {#key id}
    <div class="body" in:swap>
      <p class="head">
        <b>{look.name}</b>
        {#if view?.checkedAt}<time>{t.checked(relativeTime(view.checkedAt, timeline.now))}</time>{/if}
      </p>
      {#if line !== view?.problem}<p class="line" title={line}>{line}</p>{/if}

      {#if view?.problem}
        <p class="problem" role="status">
          <span>{view.problem}</span>
          <button type="button" class="link" onclick={() => pastilles.retry(id)}>{i18n.t.pastilles.retry}</button>
        </p>
      {/if}

      {#if id === "tool"}
        <div class="actions">
          <button type="button" class="btn sm" disabled={!view?.tool?.open} onclick={() => pastilles.showTool()}>{t.show(look.name)}</button>
        </div>
      {:else if server}
        {#if shownModels.length > 0}
          <ul class="models" aria-label={t.models}>
            {#each shownModels as m (m.name)}
              <li>
                {#if onopenchat}
                  <button type="button" class="model" onclick={() => onopenchat?.(m.name, server)} title={t.chatWith(m.name)}>
                    <i class:loaded={m.loaded} aria-hidden="true"></i><b>{m.name}</b><span>{modelDetail(m)}</span>
                  </button>
                {:else}
                  <span class="model"><i class:loaded={m.loaded} aria-hidden="true"></i><b>{m.name}</b><span>{modelDetail(m)}</span></span>
                {/if}
              </li>
            {/each}
          </ul>
          {#if models.length > MODEL_ROWS}<p class="hint">{t.moreModels(models.length - MODEL_ROWS)}</p>{/if}
        {/if}
        {#if id === "ollama"}
          {#if pull}
            <div class="pull" class:failed={pull.error !== undefined}>
              <p class="pull-head">
                <b>{pull.model}</b>
                <span>{pull.error ?? t.pullStep(pull.step, Math.round(pullShare(pull) * 100))}</span>
                <button type="button" class="link" onclick={() => pastilles.cancelPull()}>{pull.done ? t.close : i18n.t.pastilles.cancel}</button>
              </p>
              <span
                class="meter"
                role="progressbar"
                aria-label={t.pulling(pull.model)}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(pullShare(pull) * 100)}
              >
                <i style:transform="scaleX({pullShare(pull).toFixed(4)})"></i>
              </span>
            </div>
          {:else if asking}
            <form class="ask" onsubmit={download}>
              <label class="visually-hidden" for="pull-model">{t.pullModel}</label>
              <!-- svelte-ignore a11y_autofocus -->
              <input id="pull-model" bind:value={model} placeholder="gemma4, llama3.2:3b…" autocomplete="off" spellcheck="false" autofocus />
              <button type="submit" class="btn sm primary" disabled={!model.trim()}>{t.download}</button>
              <button type="button" class="link" onclick={() => (asking = false)}>{i18n.t.pastilles.cancel}</button>
            </form>
          {:else if tone !== "off"}
            <div class="actions">
              <button type="button" class="btn sm" onclick={() => (asking = true)}>{t.downloadModel}</button>
            </div>
          {/if}
        {/if}
        {#if tone === "off"}
          <p class="hint">{t.start(look.name)}</p>
        {/if}
      {:else if id === "spotify"}
        {#if spotify}
          <div class="actions">
            <Transport track={spotify} />
            <AppVolume />
          </div>
        {:else}
          <p class="hint">{t.startSpotify}</p>
        {/if}
      {:else if view}
        {#if view.metrics.length > 0}
          <dl class="metrics">
            {#each view.metrics as metric (metric.label)}
              <div class="metric {metric.tone ?? ''}">
                <dt>{metric.label}</dt>
                <dd>{metric.value}</dd>
              </div>
            {/each}
          </dl>
        {/if}
        {#if view.items.length > 0}
          <ul class="items">
            {#each view.items.slice(0, ITEM_ROWS) as item, index (index)}
              <li class={item.tone ?? ""}>
                <i aria-hidden="true"></i>
                <b>{item.title}</b>
                <span>{item.detail}</span>
                {#if item.at}<time>{relativeTime(item.at, timeline.now)}</time>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
      {#if pastilles.notice}<p class="notice" role="alert">{pastilles.notice}</p>{/if}
    </div>
  {/key}
</div>

<style>
  .card {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
    height: 100%;
    padding: var(--s3) var(--s4) var(--s3) var(--s2);
    overflow: hidden;
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  /* The pastille's color, as a soft light behind its Mochi (identity, never a block). */
  .card::before {
    content: "";
    position: absolute;
    left: -40px;
    top: 20px;
    width: 200px;
    height: 160px;
    border-radius: 50%;
    background: radial-gradient(closest-side, var(--sc), transparent);
    opacity: 0.14;
    pointer-events: none;
  }

  .mochi {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 80px;
    height: 80px;
  }

  .body {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(--s1) + var(--gap-mark));
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: var(--s2);
    min-width: 0;
  }

  .head b {
    font-size: var(--fs-4);
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .head time,
  .hint {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .head time {
    margin-left: auto;
    white-space: nowrap;
  }

  .line {
    color: var(--ink-2);
    font-size: var(--fs-3);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }

  .problem {
    display: flex;
    align-items: baseline;
    gap: var(--s2);
    color: var(--err);
    font-size: var(--fs-2);
  }

  .notice {
    color: var(--err);
    font-size: var(--fs-2);
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

  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s2) var(--s3);
  }

  /* Numbers: values in ink, labels quiet; a state color only on its value's mark. */
  .metrics {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--s2);
    margin: var(--gap-mark) 0;
  }

  .metric {
    min-width: 0;
    padding-left: var(--s2);
    border-left: 1px solid var(--line-2);
  }

  .metric dt {
    color: var(--ink-3);
    font-size: var(--fs-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .metric dd {
    margin: 0;
    font-size: var(--fs-title);
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .metric.err {
    border-left-color: var(--err);
  }

  .metric.warn {
    border-left-color: var(--warn);
  }

  .metric.busy {
    border-left-color: var(--accent);
  }

  .items,
  .models {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--gap-mark);
  }

  .items li,
  .model {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
    font-size: var(--fs-2);
    white-space: nowrap;
  }

  /* The title keeps its room; the detail gives way first. */
  .items b,
  .model b {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 64%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 600;
  }

  .items span,
  .model span {
    flex: 1 1 0;
    min-width: 0;
    color: var(--ink-3);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .items time {
    margin-left: auto;
    flex: none;
    color: var(--ink-3);
  }

  .items i,
  .model i {
    flex: none;
    width: var(--dot);
    height: var(--dot);
    border-radius: 50%;
    background: var(--ink-3);
  }

  .items .ok i {
    background: var(--ok);
  }

  .items .err i {
    background: var(--err);
  }

  .items .warn i {
    background: var(--warn);
  }

  .items .busy i {
    background: var(--accent);
  }

  /* A model in memory has a full dot; one on disk only, a ring. */
  .model i {
    background: none;
    box-shadow: inset 0 0 0 1.5px var(--ink-3);
  }

  .model i.loaded {
    background: var(--ok);
    box-shadow: none;
  }

  button.model {
    width: 100%;
    padding: 3px var(--s1);
    margin-left: calc(-1 * var(--s1));
    border-radius: var(--r-chip);
    text-align: left;
    transition: background var(--t-quick);
  }

  button.model:hover {
    background: var(--surface-2);
  }

  .pull {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
  }

  .pull-head {
    display: flex;
    align-items: baseline;
    gap: var(--s2);
    min-width: 0;
    font-size: var(--fs-2);
  }

  .pull-head b {
    font-weight: 600;
  }

  .pull-head span {
    flex: 1;
    min-width: 0;
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .failed .pull-head span {
    color: var(--err);
  }

  /* A meter: the fill in the accent over a lighter step of the same color. */
  .meter {
    height: var(--bar);
    border-radius: var(--r-pill);
    background: var(--accent-soft);
    overflow: hidden;
  }

  .meter i {
    display: block;
    height: 100%;
    border-radius: var(--r-pill);
    background: var(--accent);
    transform-origin: left;
    transition: transform 0.25s linear;
  }

  .failed .meter i {
    background: var(--err);
  }

  .ask {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .ask input {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 var(--s2);
    border-radius: var(--r-ctl);
    border: 1px solid var(--line-2);
    background: var(--panel);
    font-family: var(--font-mono);
    font-size: var(--fs-2);
    user-select: text;
  }

  .ask input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .meter i,
    button.model {
      transition: none;
    }
  }
</style>
