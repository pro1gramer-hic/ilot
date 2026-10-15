<!-- « OK Mochi » listens: a small dot by Mochi's head, in the action color, for as long as
     the microphone is open. One click closes the microphone (and turns « OK Mochi » off).
     It answers the keyword with a single ring, nothing that loops. -->
<script lang="ts">
  import { untrack } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { mochiPlace } from "../../lib/island/model";
  import { ENTER_MS, EXIT_MS, reduced } from "../../lib/motion.svelte";
  import { Spring } from "../../lib/spring.svelte";
  import { voice } from "../../lib/stores/voice.svelte";

  /** Right of Mochi's head, inside the island (its edge cuts what sticks out); in the
   * top-right corner while Mochi steps aside. */
  const target = $derived.by(() => {
    const place = mochiPlace(island.mode, island.size);
    return place ? { x: place.x + 30, y: Math.max(4, place.y + 4) } : { x: island.size.width - 26, y: 6 };
  });
  const X = new Spring(untrack(() => target.x));
  const Y = new Spring(untrack(() => target.y));
  $effect(() => {
    X.set(target.x);
    Y.set(target.y);
  });

  function stop(event: MouseEvent): void {
    event.stopPropagation();
    voice.stop();
  }
</script>

{#if voice.listening}
  <span class="spot" style:transform="translate({X.value}px, {Y.value}px)">
    <button
      type="button"
      class="dot"
      aria-label={i18n.t.island.voice.dot}
      title={i18n.t.island.voice.dotTip}
      onclick={stop}
      in:scale={{ start: 0.6, duration: reduced() ? 0 : ENTER_MS, opacity: 0 }}
      out:fade={{ duration: EXIT_MS }}
    >
      {#key voice.wakes}
        <i class:heard={voice.wakes > 0}></i>
      {/key}
    </button>
  </span>
{/if}

<style>
  .spot {
    position: absolute;
    left: 0;
    top: 0;
  }

  /* An 18 px target around an 8 px mark. */
  .dot {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    transition: background var(--t-quick) var(--ease-out);
  }

  .dot:hover {
    background: var(--accent-soft);
  }

  .dot:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: 1px;
  }

  /* The mark, ringed in the island's own black so it stays apart from Mochi. */
  i {
    width: var(--mark);
    height: var(--mark);
    border-radius: var(--r-pill);
    background: var(--accent-text);
    box-shadow: 0 0 0 2px var(--island);
  }

  /* The keyword heard: one ring opens and fades. */
  .heard {
    animation: heard 520ms var(--ease-out);
  }

  @keyframes heard {
    from {
      box-shadow:
        0 0 0 2px var(--island),
        0 0 0 2px var(--accent-text);
    }
    to {
      box-shadow:
        0 0 0 2px var(--island),
        0 0 0 9px transparent;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .heard {
      animation: none;
    }
  }
</style>
