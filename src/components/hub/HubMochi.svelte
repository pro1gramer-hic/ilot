<!-- Mochi in the hub, large, in the colors of a session, with his state badge on the head.
     He stands as the session goes (breathes at work, shakes on a failure, rocks to the
     music) unless a scene leads him (`move`, `face`). A click squashes him with big eyes,
     a few more annoy him, five in a row make him dizzy (play.svelte.ts). A right click
     opens his wardrobe in the island (prompt 12). -->
<script lang="ts">
  import { type HubStatus, mochiFor } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import type { FaceId } from "../../lib/mochi/faces";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import type { MoveId } from "../../lib/mochi/moves";
  import type { PaletteId } from "../../lib/mochi/palettes";
  import { settings } from "../../lib/stores/settings.svelte";
  import { play } from "./play.svelte";
  import StateBadge from "./StateBadge.svelte";

  let {
    palette,
    status,
    move,
    face,
    pokable = true,
  }: { palette: PaletteId; status: HubStatus; move?: MoveId; face?: FaceId; pokable?: boolean } = $props();

  const stance = $derived(mochiFor(status));
  const moveNow = $derived(pokable && play.squished ? "squish" : (move ?? stance.move));
  const faceNow = $derived<FaceId | undefined>(
    pokable && play.reaction === "annoyed" ? "annoyed" : pokable && play.reaction === "squish" ? "big" : (face ?? stance.face),
  );
  const name = $derived(settings.value.mochi.name);

  function oncontextmenu(event: MouseEvent): void {
    event.preventDefault();
    island.openWardrobe();
  }
</script>

{#if pokable}
  <button
    type="button"
    class="mochi"
    aria-label={i18n.t.hub.mochi.poke(name)}
    title={i18n.t.hub.mochi.pokeTip(name)}
    onclick={() => play.poke()}
    {oncontextmenu}
  >
    <Mochi {palette} move={moveNow} face={faceNow} scale={4} label="" look={{ x: island.pointer.x }} dressed />
    <StateBadge {status} />
  </button>
{:else}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="mochi" {oncontextmenu}>
    <Mochi {palette} move={moveNow} face={faceNow} scale={4} label={name} dressed />
    <StateBadge {status} />
  </span>
{/if}

<style>
  .mochi {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 96px;
    height: 96px;
    border-radius: var(--r-card);
  }

  button.mochi {
    cursor: pointer;
    transition: transform var(--t-quick) var(--ease-out);
  }

  button.mochi:active {
    transform: scale(var(--press));
  }

  @media (prefers-reduced-motion: reduce) {
    button.mochi:active {
      transform: none;
    }
  }
</style>
