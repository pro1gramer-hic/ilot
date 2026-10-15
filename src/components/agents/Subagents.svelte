<!-- How many subagents work for a session or an AI, in a small chip: « 3 sous-agents »
     (« 3 » alone where room is short), their kinds in the tooltip and for screen readers
     (« Explore ×2, Plan »). Nothing when none works. -->
<script lang="ts">
  import { type Subagent, subagentKinds, subagentsLabel, subagentsLine } from "../../lib/agents";

  let { subagents, compact = false }: { subagents: readonly Subagent[]; compact?: boolean } = $props();

  const kinds = $derived(subagentKinds(subagents));
  const tip = $derived(compact ? subagentsLine(subagents) : kinds || undefined);
</script>

{#if subagents.length > 0}
  <span class="subs" title={tip} aria-label={subagentsLine(subagents)} data-subagents={subagents.length}>
    <i aria-hidden="true"></i><i aria-hidden="true"></i>
    <span aria-hidden="true">{compact ? subagents.length : subagentsLabel(subagents.length)}</span>
  </span>
{/if}

<style>
  .subs {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px var(--s2) 1px 6px;
    border-radius: var(--r-chip);
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink-2);
    font-size: var(--fs-2);
    font-weight: 500;
    white-space: nowrap;
    flex: none;
  }

  /* Two small stacked dots: the agent's helpers. */
  i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--ink-3);
  }

  i + i {
    margin-left: -5px;
    transform: translate(3px, -2px);
    background: var(--ink-2);
  }

  span[aria-hidden] {
    margin-left: 3px;
    font-variant-numeric: tabular-nums;
  }
</style>
