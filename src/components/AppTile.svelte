<!-- Small square for a messaging app: its Windows logo when known, otherwise its letters.
     `ring` draws a thin ring in the app's color around it (appColor), set apart by a gap;
     `lit` fades that ring in and out. Decorative: the app name is always written next to it. -->
<script lang="ts">
  let {
    icon,
    monogram,
    size = 20,
    ring,
    lit = true,
  }: { icon?: string; monogram: string; size?: number; ring?: string; lit?: boolean } = $props();
</script>

<span class="tile" class:lit={ring !== undefined && lit} style:--size="{size}px" style:--ring={ring} aria-hidden="true">
  <span class="face">{#if icon}<img src={icon} alt="" />{:else}{monogram}{/if}</span>
</span>

<style>
  .tile {
    --radius: calc(var(--size) * 0.3);
    position: relative;
    flex: none;
    display: block;
    width: var(--size);
    height: var(--size);
  }

  .face {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: var(--radius);
    background: var(--card-2);
    box-shadow: inset 0 0 0 1px var(--line), var(--hi);
    color: var(--ink);
    font-size: max(var(--fs-1), calc(var(--size) * 0.46));
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1;
  }

  /* The ring keeps the radius of the tile, one gap further out. */
  .tile::after {
    content: "";
    position: absolute;
    inset: calc(-2 * var(--ring-w));
    border: var(--ring-w) solid var(--ring, transparent);
    border-radius: calc(var(--radius) + 2 * var(--ring-w));
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--t-quick) var(--ease-out);
  }

  .tile.lit::after {
    opacity: 1;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
</style>
