<!-- Thin on/off switch. Put it inside a <label> (see Row): the whole row is the click target
     and gives the switch its name. The knob slides; the accent fill only fades in. -->
<script lang="ts">
  let {
    checked = $bindable(false),
    disabled = false,
    onchange,
  }: { checked?: boolean; disabled?: boolean; onchange?: (checked: boolean) => void } = $props();
</script>

<input type="checkbox" role="switch" bind:checked {disabled} onchange={() => onchange?.(checked)} />

<style>
  input {
    /* knob and the space around it, from the track height */
    --knob: calc(var(--switch-h) - 3 * var(--gap-mark));
    --inset: calc((var(--switch-h) - var(--knob)) / 2);
    appearance: none;
    position: relative;
    flex: none;
    width: var(--switch-w);
    height: var(--switch-h);
    border-radius: var(--r-pill);
    background: var(--well-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
    cursor: pointer;
  }

  input::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--accent);
    opacity: 0;
    transition: opacity var(--t-quick) var(--ease-out);
  }

  input::after {
    content: "";
    position: absolute;
    top: var(--inset);
    left: var(--inset);
    width: var(--knob);
    height: var(--knob);
    border-radius: var(--r-pill);
    background: var(--ink-2);
    transition: transform var(--t-enter) var(--ease-out);
  }

  input:checked::before {
    opacity: 1;
  }

  input:checked::after {
    background: var(--on-accent);
    transform: translateX(calc(var(--switch-w) - var(--knob) - 2 * var(--inset)));
  }

  input:disabled {
    opacity: var(--dim);
    cursor: default;
  }

  @media (prefers-reduced-motion: reduce) {
    input::after {
      transition: none;
    }
  }
</style>
