<!-- The decisions journal (docs/09-DEMO-FINALE.md, section 9): time, AI, the exact command
     and what the user decided. The core keeps it in memory only; the latest come first. -->
<script lang="ts">
  import { DECISION, sourceColor, sourceTool } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { clockLabel } from "../../lib/timeline";
  import Button from "../settings/Button.svelte";

  /** Rows shown before « Tout afficher ». */
  const FIRST = 8;

  const t = $derived(i18n.t.agentsPanel.journal);
  let all = $state(false);
  const count = $derived(decisions.list.length);
  const rows = $derived(all ? decisions.list : decisions.list.slice(0, FIRST));
</script>

<section aria-labelledby="journal-title">
  <div class="head">
    <h3 class="label" id="journal-title">{t.title}</h3>
    <span class="label">{t.count(count)}</span>
  </div>
  {#if count > 0}
    <ul class="log">
      {#each rows as row (`${row.at}-${row.source}-${row.command}`)}
        <li>
          <time class="mono">{clockLabel(row.at)}</time>
          <span class="who"><i class="dot" style:background={sourceColor(row.source)} aria-hidden="true"></i>{sourceTool(row.source)}</span>
          <code title={row.command}>{row.command}</code>
          <span class="verdict {DECISION[row.decision].tone}"><i aria-hidden="true"></i>{DECISION[row.decision].label}</span>
        </li>
      {/each}
    </ul>
    {#if count > FIRST}
      <div class="more">
        <Button variant="quiet" onclick={() => (all = !all)}>{all ? t.less : t.all(count)}</Button>
      </div>
    {/if}
  {:else}
    <p class="empty">{t.empty}</p>
  {/if}
</section>

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    margin-bottom: var(--s2);
  }

  .label {
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .log {
    list-style: none;
    display: grid;
    grid-template-columns: max-content minmax(0, max-content) minmax(0, 1fr) max-content;
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    font-size: var(--fs-3);
  }

  .log li {
    display: grid;
    grid-column: 1 / -1;
    grid-template-columns: subgrid;
    align-items: center;
    column-gap: var(--s4);
    padding: var(--s3) var(--s4);
  }

  .log li + li {
    border-top: 1px solid var(--line);
  }

  time {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .who,
  .verdict {
    display: flex;
    align-items: center;
    gap: var(--s2);
    white-space: nowrap;
  }

  .who {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dot,
  .verdict i {
    flex: none;
    width: var(--dot);
    height: var(--dot);
    border-radius: var(--r-pill);
  }

  code {
    min-width: 0;
    font: var(--fs-2) var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .verdict {
    font-size: var(--fs-2);
    font-weight: 600;
  }

  .verdict i {
    background: var(--ink-3);
  }

  .verdict.ok i {
    background: var(--ok);
  }

  .verdict.err i {
    background: var(--err);
  }

  .more {
    display: flex;
    justify-content: center;
    margin-top: var(--s2);
  }

  .empty {
    padding: var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    color: var(--ink-3);
    font-size: var(--fs-3);
    text-align: center;
  }
</style>
