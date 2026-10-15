<!-- One setting on one compact row: its name, an optional line of help, and its control at
     the end. `label` makes the whole row the control's label (switches, check boxes). -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { rise } from "../../lib/motion.svelte";

  let {
    title,
    hint,
    label = false,
    titleId,
    index = 0,
    children,
  }: {
    title: string;
    /** Help under the name: plain text, or a snippet when it holds marks or code. */
    hint?: string | Snippet;
    label?: boolean;
    /** Id of the name, for a control labelled by it (aria-labelledby). */
    titleId?: string;
    index?: number;
    children: Snippet;
  } = $props();
</script>

<svelte:element this={label ? "label" : "div"} class="row" class:label in:rise|global={{ index }}>
  <span class="text">
    <span id={titleId}>{title}</span>
    {#if typeof hint === "string"}
      <small>{hint}</small>
    {:else if hint}
      <small>{@render hint()}</small>
    {/if}
  </span>
  <span class="end">{@render children()}</span>
</svelte:element>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    min-height: var(--h-row);
    padding: calc(var(--s2) - var(--gap-mark)) 0;
    border-top: 1px solid var(--line);
  }

  .label {
    cursor: pointer;
  }

  .text {
    min-width: 0;
    color: var(--ink);
    font-size: var(--fs-body);
  }

  small {
    display: block;
    margin-top: calc(var(--s1) / 2);
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .end {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
</style>
