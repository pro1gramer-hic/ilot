<!-- The badge on Mochi's head (09, section 8 bis): three dots that pulse while the agent
     works, a check once done, an exclamation for a failure or a wait, a note for the
     music. Reserved colors, always with their sign; the card's text says the state in
     words. It pops in on a spring-like curve, and changes without moving under reduced
     motion. -->
<script lang="ts">
  import type { HubStatus } from "../../lib/hub";
  import HubIcon from "./HubIcon.svelte";

  let { status }: { status: HubStatus } = $props();
</script>

{#key status}
  {#if status !== "idle"}
    <span class="badge {status}" aria-hidden="true">
      {#if status === "work"}
        <span class="dots"><i></i><i></i><i></i></span>
      {:else if status === "ok"}
        <HubIcon name="check" size={12} />
      {:else if status === "play"}
        <HubIcon name="note" size={12} />
      {:else}
        <HubIcon name="alert" size={12} />
      {/if}
    </span>
  {/if}
{/key}

<style>
  .badge {
    position: absolute;
    left: 2px;
    top: 0;
    z-index: 1;
    min-width: var(--s5);
    height: var(--s5);
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    color: var(--accent-ink);
    background: var(--accent);
    box-shadow: 0 0 0 2.5px var(--badge-ring, var(--surface));
    animation: pop var(--t-in) var(--ease-pop) both;
  }

  .badge :global(svg) {
    stroke-width: 2.2;
  }

  .ok {
    background: var(--ok);
    color: var(--risk-ink);
  }

  .err {
    background: var(--err);
    color: var(--ink);
  }

  .perm {
    background: var(--warn);
    color: var(--risk-ink);
  }

  /* The music's own color (its source), with the note. */
  .play {
    background: var(--sc, var(--src-spotify));
    color: var(--risk-ink);
  }

  .dots {
    display: flex;
    gap: var(--gap-mark);
  }

  .dots i {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: currentColor;
    animation: think 1s infinite;
  }

  .dots i:nth-child(2) {
    animation-delay: 0.15s;
  }

  .dots i:nth-child(3) {
    animation-delay: 0.3s;
  }

  @keyframes pop {
    from {
      transform: scale(0.6);
      opacity: 0;
    }
  }

  @keyframes think {
    0%,
    60%,
    100% {
      opacity: 0.35;
      transform: none;
    }

    30% {
      opacity: 1;
      transform: translateY(-1.5px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .badge {
      animation-name: fade;
    }

    .dots i {
      animation: none;
      opacity: 0.85;
    }

    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
