<!-- The island's wardrobe (prompt 12, lot 1), opened by a right click on Mochi in the
     pill, the hub or out on the desktop: Mochi large on the left wearing what is chosen
     (he smiles at each new piece), the pieces on the right. Escape, the cross or a click
     elsewhere closes it. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { type MoveId, OWN_MOVE_S } from "../../lib/mochi/moves";
  import { rise } from "../../lib/motion.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import PanelIcon from "../panel/PanelIcon.svelte";
  import Wardrobe from "../wardrobe/Wardrobe.svelte";

  const t = $derived(i18n.t.island.wardrobe);
  const name = $derived(settings.value.mochi.name);
  let move = $state<MoveId>("breathe");
  let timer: ReturnType<typeof setTimeout> | undefined;

  /** A new piece: he smiles at it. */
  function react(): void {
    clearTimeout(timer);
    move = "smile";
    timer = setTimeout(() => (move = "breathe"), OWN_MOVE_S.smile * 1000);
  }

  $effect(() => () => clearTimeout(timer));
</script>

<div class="view" role="dialog" aria-modal="false" aria-labelledby="wardrobe-title">
  <figure class="preview" in:rise={{ index: 0, delay: 80 }}>
    <Mochi {move} scale={4} label={t.dressed(name)} animated={settings.value.mochi.animated} dressed />
    <figcaption>{name}</figcaption>
  </figure>
  <div class="side" in:rise={{ index: 1, delay: 80 }}>
    <header>
      <h2 id="wardrobe-title">{t.title}</h2>
      <button type="button" class="close" aria-label={t.close} title={t.closeTip} onclick={() => island.closeWardrobe()}>
        <PanelIcon name="close" />
      </button>
    </header>
    <Wardrobe onpick={react} autofocus />
  </div>
</div>

<style>
  .view {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--s3);
    padding: var(--s3) var(--s4) var(--s4) var(--s3);
    color: var(--ink);
    cursor: default;
  }

  .preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s1);
    padding-top: var(--s4);
  }

  figcaption {
    max-width: calc(var(--s8) * 3);
    overflow: hidden;
    font-family: var(--font-pixel);
    font-size: var(--fs-4);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    font-size: var(--fs-4);
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .close {
    display: grid;
    place-items: center;
    width: var(--h-ctl);
    height: var(--h-ctl);
    border-radius: var(--r-ctl);
    color: var(--ink-2);
    transition:
      background-color var(--t-quick),
      color var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .close:hover {
    background: var(--hover);
    color: var(--ink);
  }

  .close:active {
    transform: scale(var(--press));
  }

  @media (prefers-reduced-motion: reduce) {
    .close:active {
      transform: none;
    }
  }
</style>
