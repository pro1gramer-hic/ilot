<!-- Round icon button of the music surfaces: a light shade fades in on hover, a press sinks
     it. `solid` fills it in ink (play-pause); `size` in pixels. Always give it an
     aria-label: the sign inside is decorative. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    solid = false,
    size = 36,
    children,
    ...rest
  }: HTMLButtonAttributes & { solid?: boolean; size?: number; children: Snippet } = $props();
</script>

<button type="button" class:solid style:--b="{size}px" {...rest}>{@render children()}</button>

<style>
  button {
    position: relative;
    flex: none;
    width: var(--b);
    height: var(--b);
    display: grid;
    place-items: center;
    isolation: isolate;
    border-radius: 50%;
    color: var(--ink);
    transition: transform var(--t-quick) var(--ease-out);
  }

  /* Hover shade under the sign: only its opacity moves. */
  button::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: var(--surface-2);
    opacity: 0;
    transition: opacity var(--t-quick);
  }

  button:hover:enabled::before {
    opacity: 1;
  }

  button:active:enabled {
    transform: scale(var(--press));
  }

  button:disabled {
    color: var(--ink-3);
    opacity: var(--dim);
    cursor: default;
  }

  .solid {
    background: var(--ink);
    color: var(--island);
    box-shadow: var(--shadow-soft);
  }

  .solid::before {
    background: var(--island);
  }

  .solid:hover:enabled::before {
    opacity: 0.12;
  }

  .solid:disabled {
    background: var(--surface-2);
    color: var(--ink-3);
    box-shadow: none;
    opacity: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    button:active:enabled {
      transform: none;
    }
  }
</style>
