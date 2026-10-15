<!-- Volume of Windows (not of one app): a speaker and a thin slider, the set part in ink.
     The core moves the volume by notches of 2 %, the slider asks for as many as it takes,
     one request at a time, and shows the level the core answers. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { changeVolume, getVolume } from "../../lib/ipc";
  import Icon from "../Icon.svelte";

  /** One notch of `change_volume`, in percent. */
  const NOTCH = 2;

  /** Level shown, in percent. */
  let level = $state(0);
  /** Level the core reported last, in percent. */
  let reported = 0;
  let wanted: number | null = null;
  let busy = false;

  $effect(() => {
    void getVolume().then((value) => {
      reported = Math.round(value * 100);
      level = reported;
    });
  });

  async function follow(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      while (wanted !== null) {
        const notches = Math.round((wanted - reported) / NOTCH);
        wanted = null;
        if (notches !== 0) reported = Math.round((await changeVolume(notches)) * 100);
      }
    } finally {
      busy = false;
    }
  }

  function input(event: Event & { currentTarget: HTMLInputElement }): void {
    level = Number(event.currentTarget.value);
    wanted = level;
    void follow();
  }
</script>

<label class="volume">
  <span class="speaker"><Icon name="volume" size={14} /></span>
  <span class="visually-hidden">{i18n.t.music.volume.label}</span>
  <input type="range" min="0" max="100" step={NOTCH} value={level} style:--v="{level}%" aria-valuetext={i18n.t.music.volume.level(level)} oninput={input} />
</label>

<style>
  .volume {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 140px;
    color: var(--ink-2);
  }

  .speaker {
    flex: none;
  }

  input {
    flex: 1;
    min-width: 0;
    height: 20px;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  /* The track: the set part in ink, the rest in a faint line. */
  input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--r-pill);
    background: linear-gradient(90deg, var(--ink) var(--v), var(--line-2) var(--v));
  }

  input::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: var(--ink);
    box-shadow: var(--shadow-soft);
    transition: transform var(--t-quick) var(--ease-out);
  }

  input:hover::-webkit-slider-thumb {
    transform: scale(1.15);
  }

  input:focus-visible {
    outline: none;
  }

  input:focus-visible::-webkit-slider-thumb {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    input::-webkit-slider-thumb {
      transition: none;
    }

    input:hover::-webkit-slider-thumb {
      transform: none;
    }
  }
</style>
