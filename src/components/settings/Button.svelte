<!-- Compact button of the panel tabs (28 px): `primary` in the one action color, `plain` on a
     raised surface, `quiet` without a background until hovered; `square` for an icon alone
     (always with an aria-label). Hover only fades a light layer in; a press sinks the button. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    variant = "plain",
    square = false,
    children,
    ...rest
  }: HTMLButtonAttributes & { variant?: "primary" | "plain" | "quiet"; square?: boolean; children: Snippet } = $props();
</script>

<button type="button" class={variant} class:square {...rest}>{@render children()}</button>

<style>
  button {
    position: relative;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--s1);
    height: var(--h-ctl);
    padding: 0 var(--s3);
    border-radius: var(--r-ctl);
    color: var(--ink);
    font-size: var(--fs-2);
    font-weight: 600;
    white-space: nowrap;
    transition:
      transform var(--t-quick) var(--ease-out),
      opacity var(--t-quick) var(--ease-out);
  }

  button::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--hover);
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--t-quick) var(--ease-out);
  }

  button:hover:not(:disabled)::after {
    opacity: 1;
  }

  button:active:not(:disabled) {
    transform: scale(var(--press));
  }

  .plain {
    background: var(--card-2);
    border: 1px solid var(--line);
    box-shadow: var(--hi);
  }

  .primary {
    background: var(--accent);
    color: var(--on-accent);
    box-shadow: var(--btn-shadow);
  }

  .quiet {
    padding: 0 var(--s2);
    color: var(--ink-2);
  }

  .quiet:hover {
    color: var(--ink);
  }

  .square {
    width: var(--h-ctl);
    padding: 0;
  }

  button:disabled {
    opacity: var(--dim);
    cursor: progress;
  }

  @media (prefers-reduced-motion: reduce) {
    button:active:not(:disabled) {
      transform: none;
    }
  }
</style>
