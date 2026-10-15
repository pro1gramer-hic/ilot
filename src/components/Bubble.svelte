<!-- Mochi's speech bubble (3.5 s, see stores/bubble.svelte.ts): under the island, or above
     it when the island opens upward. It never covers the island. -->
<script lang="ts">
  import type { Box } from "../lib/island/model";
  import { bubble } from "../lib/stores/bubble.svelte";
  import { settings } from "../lib/stores/settings.svelte";

  let { box, opensDown }: { box: Box; opensDown: boolean } = $props();

  const GAP = 12;
</script>

{#if bubble.text}
  <div
    class="bubble"
    class:above={!opensDown}
    role="status"
    style:left="{box.x + box.width / 2}px"
    style:top={opensDown ? `${box.y + box.height + GAP}px` : undefined}
    style:bottom={opensDown ? undefined : `calc(100% - ${box.y - GAP}px)`}
  >
    <b>{settings.value.mochi.name}</b>
    <span>{bubble.text}</span>
  </div>
{/if}

<style>
  .bubble {
    position: absolute;
    transform: translateX(-50%);
    max-width: 420px;
    padding: var(--s2) var(--s3);
    border-radius: var(--r-card);
    background: var(--ink);
    color: var(--island);
    box-shadow: var(--shadow-soft);
    pointer-events: none;
    animation: pop var(--t-enter) var(--ease-pop);
  }

  .bubble::before {
    content: "";
    position: absolute;
    left: 50%;
    top: -5px;
    width: 10px;
    height: 10px;
    background: inherit;
    border-radius: 2px;
    transform: translateX(-50%) rotate(45deg);
  }

  .above::before {
    top: auto;
    bottom: -5px;
  }

  b {
    display: block;
    font-family: var(--font-pixel);
    font-size: var(--fs-1);
    font-weight: 600;
  }

  span {
    font-size: var(--fs-2);
    font-weight: 500;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(-4px) scale(0.96);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bubble {
      animation-name: appear;
    }

    @keyframes appear {
      from {
        opacity: 0;
      }
    }
  }
</style>
