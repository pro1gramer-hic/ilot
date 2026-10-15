<!-- A segmented control: one choice out of a few, as radio buttons in a row. The chosen
     one's block slides to it on a spring; arrow keys move the choice (WAI-ARIA radio group). -->
<script lang="ts">
  import { Spring } from "../../lib/spring.svelte";

  let {
    options,
    value,
    labelledby,
    onchange,
  }: {
    options: readonly (readonly [string, string])[];
    value: string;
    labelledby: string;
    onchange: (value: string) => void;
  } = $props();

  const buttons = $state<HTMLButtonElement[]>([]);
  const index = $derived(Math.max(0, options.findIndex(([id]) => id === value)));

  const left = new Spring(0);
  const width = new Spring(0);
  let placed = false;
  $effect(() => {
    const button = buttons[index];
    // Labels change width with the language: the block follows them.
    void options;
    if (!button) return;
    if (placed) {
      left.set(button.offsetLeft);
      width.set(button.offsetWidth);
    } else {
      left.snap(button.offsetLeft);
      width.snap(button.offsetWidth);
      placed = true;
    }
  });

  // Labels change width once the interface font is in: the block follows at once.
  $effect(() => {
    let live = true;
    void document.fonts.ready.then(() => {
      const button = buttons[index];
      if (!live || !button) return;
      left.snap(button.offsetLeft);
      width.snap(button.offsetWidth);
    });
    return () => (live = false);
  });

  function onKey(event: KeyboardEvent): void {
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const next = (index + step + options.length) % options.length;
    onchange(options[next][0]);
    buttons[next]?.focus();
  }
</script>

<div class="segmented" role="radiogroup" aria-labelledby={labelledby}>
  <i class="thumb" style:transform="translateX({left.value}px)" style:width="{width.value}px" aria-hidden="true"></i>
  {#each options as [id, label], position (id)}
    <button
      type="button"
      role="radio"
      aria-checked={id === value}
      tabindex={id === value ? 0 : -1}
      bind:this={buttons[position]}
      onclick={() => onchange(id)}
      onkeydown={onKey}
    >
      {label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: flex;
    flex-wrap: nowrap;
    padding: var(--gap-mark);
    border-radius: calc(var(--r-ctl) + var(--gap-mark));
    background: var(--panel);
    border: 1px solid var(--line);
  }

  .thumb {
    position: absolute;
    left: 0;
    top: var(--gap-mark);
    bottom: var(--gap-mark);
    border-radius: var(--r-ctl);
    background: var(--surface-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      var(--hi);
    pointer-events: none;
    will-change: transform;
  }

  button {
    position: relative;
    height: calc(var(--h-ctl) + var(--s1));
    padding: 0 var(--s3);
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    font-size: var(--fs-3);
    font-weight: 500;
    white-space: nowrap;
    transition: color var(--t-quick) var(--ease-out);
  }

  button:hover,
  button[aria-checked="true"] {
    color: var(--ink);
  }
</style>
