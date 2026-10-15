<!-- A state in a word, with its color on a small dot only: the text stays in ink. A finished
     or paused state gets a hollow dot; an agent that waits for the user breathes slowly. -->
<script lang="ts">
  import type { Tone } from "../../lib/agents";

  let { tone, label, chip = false }: { tone: Tone; label: string; chip?: boolean } = $props();
</script>

<span class="status {tone}" class:chip><i aria-hidden="true"></i>{label}</span>

<style>
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
    color: var(--ink-2);
    font-size: var(--fs-1);
    font-weight: 500;
    white-space: nowrap;
  }

  .chip {
    height: calc(var(--h-ctl) - var(--s1));
    padding: 0 calc(var(--s2) + var(--gap-mark));
    border-radius: var(--r-pill);
    border: 1px solid var(--line);
    font-size: var(--fs-2);
  }

  i {
    flex: none;
    width: var(--dot);
    height: var(--dot);
    border-radius: var(--r-pill);
    background: var(--ink-3);
  }

  .ok i {
    background: var(--ok);
  }

  .wait i {
    background: var(--wait);
  }

  /* An agent waiting for the user breathes; a chip in the settings stays still. */
  .wait:not(.chip) i {
    animation: breathe var(--t-breath) ease-in-out infinite;
  }

  .err i {
    background: var(--err);
  }

  .idle i {
    background: none;
    box-shadow: inset 0 0 0 1px var(--ink-3);
  }

  @keyframes breathe {
    50% {
      opacity: var(--dim);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wait:not(.chip) i {
      animation: none;
    }
  }
</style>
