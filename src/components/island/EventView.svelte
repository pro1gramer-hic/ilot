<!-- Short openings (docs/09-DEMO-FINALE.md, section 4): an agent finished (440 × 64, its
     color in a thin bar, « Voir »), an agent failed (540 × 64, red edge, « Voir
     l'erreur »), or a toast answering what the user just did (440 × 52). Mochi sits in
     the slot on the left. -->
<script lang="ts">
  import { sessionColor, sessionTool } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { formatDuration } from "../../lib/island/banners";
  import { island } from "../../lib/island/controller.svelte";
  import { rise } from "../../lib/motion.svelte";
  import { banners } from "../../lib/stores/banners.svelte";

  const t = $derived(i18n.t.island.event);
  const banner = $derived(banners.current);

  const finished = $derived.by(() => {
    if (banner?.kind !== "finished") return null;
    const { items } = banner;
    const [first] = items;
    const added = items.reduce((sum, i) => sum + i.added, 0);
    const removed = items.reduce((sum, i) => sum + i.removed, 0);
    const diff = added || removed ? `, +${added} −${removed}` : "";
    // One agent whose latest change îlot kept: its file and lines open that change.
    const change = items.length === 1 && first.file && first.sessionId ? { file: first.file, session: first.sessionId } : null;
    return items.length === 1
      ? {
          color: sessionColor(first),
          label: sessionTool(first),
          title: t.finished(sessionTool(first)),
          sub: `${first.project}, ${formatDuration(first.durationMs)}${change ? "," : diff}`,
          change,
          added,
          removed,
        }
      : {
          color: "var(--ok)",
          label: t.agents,
          title: t.finishedMany(items.length),
          sub: `${[...new Set(items.map((i) => i.project))].join(", ")}${diff}`,
          change: null,
          added,
          removed,
        };
  });

  function openChange(session: string): void {
    banners.queue.dismissCurrent();
    island.openChange(session);
  }

  function view(): void {
    banners.queue.dismissCurrent();
    island.openPanel("agents");
  }
</script>

{#if finished}
  <div class="row" style:height="64px">
    <span class="slot" aria-hidden="true"></span>
    <span class="bar" style:--c={finished.color}></span>
    <div class="text" role="status">
      <span class="caps" in:rise={{ index: 1 }}>{finished.label}</span>
      <b in:rise={{ index: 2 }}>{finished.title}</b>
      <span class="sub" in:rise={{ index: 3 }}>
        {finished.sub}
        {#if finished.change}
          {@const change = finished.change}
          <button
            type="button"
            class="counter"
            aria-label={i18n.t.island.change.label(change.file, finished.added, finished.removed)}
            title={i18n.t.island.change.title}
            onclick={() => openChange(change.session)}
          >
            {change.file} <span class="add">+{finished.added}</span> <span class="del">−{finished.removed}</span>
          </button>
        {/if}
      </span>
    </div>
    <button type="button" class="btn sm primary" onclick={view} in:rise={{ index: 4 }}>{t.view}</button>
  </div>
{:else if banner?.kind === "failed"}
  {@const tool = sessionTool(banner.failed)}
  <div class="row" style:height="64px">
    <span class="slot" aria-hidden="true"></span>
    <span class="bar" style:--c="var(--err)"></span>
    <div class="text" role="alert">
      <span class="caps" in:rise={{ index: 1 }}>{tool}</span>
      <b in:rise={{ index: 2 }}>{t.failed(tool)}</b>
      <span class="sub" in:rise={{ index: 3 }}>{t.failure(banner.failed.project, banner.failed.detail)}</span>
    </div>
    <button type="button" class="btn sm" onclick={view} in:rise={{ index: 4 }}>{t.viewError}</button>
  </div>
{:else if banner?.kind === "toast"}
  <div class="row" style:height="52px">
    <span class="slot" aria-hidden="true"></span>
    <span class="icon {banner.tone}" aria-hidden="true">
      <svg viewBox="0 0 16 16">
        {#if banner.tone === "err"}
          <path class="draw" d="M4.5 4.5l7 7M11.5 4.5l-7 7" />
        {:else if banner.tone === "acc"}
          <path d="M4 2.5h5l3 3v8H4z M9 2.5v3h3" />
        {:else}
          <path class="draw" d="M3.5 8.5l3 3 6-7" />
        {/if}
      </svg>
    </span>
    <div class="text" role="status">
      <b in:rise={{ index: 1 }}>{banner.title}</b>
      {#if banner.sub}
        <span class="sub" in:rise={{ index: 2 }}>{banner.sub}</span>
      {/if}
    </div>
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 0 var(--s3);
  }

  .slot {
    width: 36px;
    height: 36px;
    flex: none;
  }

  /* The AI's color, as a thin bar only. */
  .bar {
    width: 3px;
    height: 32px;
    flex: none;
    border-radius: 2px;
    background: var(--c);
  }

  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }

  .caps {
    margin-bottom: 1px;
    font-size: var(--fs-0);
  }

  b,
  .sub {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
    letter-spacing: -0.005em;
  }

  .sub {
    color: var(--ink-2);
    font-size: var(--fs-sub);
  }

  /* The file and its lines open the change in the hub. */
  .counter {
    margin: -2px -4px;
    padding: 2px 4px;
    border-radius: var(--r-chip);
    color: var(--ink);
    font-size: var(--fs-sub);
    transition: background-color var(--t-quick);
  }

  .counter:hover {
    background: var(--hover);
  }

  .counter:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .counter .add,
  .counter .del {
    font: 600 var(--fs-2) var(--font-mono);
  }

  .counter .add {
    color: var(--ok);
  }

  .counter .del {
    color: var(--err);
  }

  .icon {
    width: 28px;
    height: 28px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
  }

  .icon svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon.ok {
    background: var(--ok-soft);
    color: var(--ok);
  }

  .icon.err {
    background: var(--err-soft);
    color: var(--err);
  }

  .icon.acc {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  /* The check (or the cross) draws itself as the toast arrives. */
  .draw {
    stroke-dasharray: 22;
    stroke-dashoffset: 22;
    animation: draw 0.45s var(--ease-out) 0.22s forwards;
  }

  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .draw {
      animation: none;
      stroke-dashoffset: 0;
    }
  }
</style>
