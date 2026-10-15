<!-- « Trop de clics » (09, section 8 bis): five clicks on Mochi within 1.6 s. The hub
     shrinks to 720 × 140, Mochi gets dizzy in a soft red light, a countdown runs, then the
     home comes back (play.svelte.ts). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import type { PaletteId } from "../../lib/mochi/palettes";
  import { play } from "./play.svelte";

  let { palette }: { palette: PaletteId } = $props();

  const t = $derived(i18n.t.hub.dizzy);
</script>

<div class="dizzy">
  <Mochi {palette} move="dizzy" scale={4} label={t.label(settings.value.mochi.name)} dressed />
  <!-- Read once: the countdown itself is for the eyes. -->
  <div class="text" role="status">
    <b>{t.title}</b>
    <span aria-hidden="true">{t.wait} <em>{play.countdown}</em> {t.seconds(play.countdown)}</span>
    <span class="visually-hidden">{t.said}</span>
  </div>
</div>

<style>
  /* Full bleed: the light reaches the hub's own edges (Hub.svelte's padding). */
  .dizzy {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s4);
    height: calc(100% + var(--s2) + var(--s3));
    margin: calc(-1 * var(--s2)) calc(-1 * var(--s3)) 0;
    padding: 0 var(--s8);
    overflow: hidden;
  }

  /* A red light low on the right, a little of the accent high on the left. */
  .dizzy::before {
    content: "";
    position: absolute;
    inset: -40%;
    background:
      radial-gradient(closest-side at 70% 80%, color-mix(in srgb, var(--err) 30%, transparent), transparent 70%),
      radial-gradient(closest-side at 30% 20%, var(--accent-soft), transparent 70%);
    pointer-events: none;
  }

  .text {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--gap-mark);
  }

  b {
    font-size: var(--fs-title);
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  span {
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  em {
    color: var(--ink);
    font-style: normal;
    font-weight: 600;
  }
</style>
