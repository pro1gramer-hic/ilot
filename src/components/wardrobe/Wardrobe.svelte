<!-- Mochi's wardrobe (prompt 12, lot 1): one row per slot (Tête, Yeux, Cou, Oreilles),
     each piece shown on Mochi himself, cropped on the part it dresses; then the season
     outfits switch. Used in the island (WardrobeView) and in Réglages › Mochi. Keyboard:
     each row is one Tab stop, the arrows move between pieces and rows, Enter or Space
     puts a piece on (or takes it off), Escape closes the island's wardrobe (startup.ts). -->
<script lang="ts">
  import { onMount } from "svelte";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { PIECES, type PieceId, SEASONS, SLOTS, type Slot, piecesOf } from "../../lib/mochi/outfits";
  import { settings } from "../../lib/stores/settings.svelte";
  import { sound } from "../../lib/stores/sound.svelte";
  import { wardrobe } from "../../lib/stores/wardrobe.svelte";
  import Switch from "../settings/Switch.svelte";

  let { onpick, autofocus = false }: { onpick?: (piece: PieceId | null) => void; autofocus?: boolean } = $props();

  const uid = $props.id();
  /** CSS pixels per unit of the tiles' Mochi (48 px), cropped to a 34 px tile. */
  const TILE_SCALE = 2;
  /** How far each slot's crop moves up, in px: the part of Mochi the slot dresses. */
  const CROP: Readonly<Record<Slot, number>> = { head: 0, eyes: 10, neck: 14, ears: 4 };

  const rows = SLOTS.map((slot) => ({ ...slot, choices: [null, ...piecesOf(slot.id)] as (PieceId | null)[] }));
  const buttons = $state<HTMLButtonElement[][]>(rows.map(() => []));
  const t = $derived(i18n.t.wardrobe);
  const mochi = $derived(settings.value.mochi);
  const season = $derived(wardrobe.season);
  const seasonal = $derived(settings.value.wardrobe.seasonal);

  const worn = (slot: Slot): PieceId | null => wardrobe.worn[slot] ?? null;
  const fromSeason = (slot: Slot, piece: PieceId | null): boolean =>
    piece !== null && seasonal && season !== null && SEASONS[season].outfit[slot] === piece && !(slot in wardrobe.chosen);

  const seasonHint = $derived.by(() => {
    if (!season) return t.noSeason;
    return t.seasonOutfit(
      SEASONS[season].label,
      Object.values(SEASONS[season].outfit).map((piece) => PIECES[piece].label),
    );
  });

  function pick(slot: Slot, piece: PieceId | null): void {
    void wardrobe.choose(slot, piece);
    sound.play("wardrobe");
    onpick?.(piece);
  }

  /** Arrows move between pieces (left, right) and rows (up, down); Home and End too. */
  function onkeydown(event: KeyboardEvent, row: number, col: number): void {
    const width = rows[row].choices.length;
    let next: [number, number] | undefined;
    if (event.key === "ArrowRight") next = [row, (col + 1) % width];
    else if (event.key === "ArrowLeft") next = [row, (col - 1 + width) % width];
    else if (event.key === "Home") next = [row, 0];
    else if (event.key === "End") next = [row, width - 1];
    else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      const to = (row + (event.key === "ArrowDown" ? 1 : -1) + rows.length) % rows.length;
      next = [to, Math.min(col, rows[to].choices.length - 1)];
    }
    if (!next) return;
    event.preventDefault();
    buttons[next[0]][next[1]]?.focus();
  }

  // In the island, the keyboard starts on the first row's worn piece (once, when it opens).
  onMount(() => {
    if (!autofocus) return;
    const col = rows[0].choices.indexOf(worn("head"));
    buttons[0][Math.max(0, col)]?.focus({ preventScroll: true });
  });
</script>

<div class="wardrobe">
  {#each rows as row, r (row.id)}
    {@const on = worn(row.id)}
    <div class="slot">
      <span class="name" id="{uid}-{row.id}">
        {row.label}
        <small>{on ? PIECES[on].label : t.nothing}</small>
      </span>
      <div class="pieces" role="group" aria-labelledby="{uid}-{row.id}">
        {#each row.choices as piece, c (piece ?? "rien")}
          {@const label = piece ? PIECES[piece].label : t.nothing}
          <button
            type="button"
            class="piece"
            data-piece={piece ?? "rien"}
            aria-pressed={on === piece}
            aria-label={fromSeason(row.id, piece) ? t.seasonal(label) : label}
            title={label}
            tabindex={on === piece ? 0 : -1}
            bind:this={buttons[r][c]}
            onclick={() => pick(row.id, piece)}
            onkeydown={(event) => onkeydown(event, r, c)}
          >
            <span class="crop" style:--crop="-{CROP[row.id]}px">
              <Mochi
                kind={mochi.kind}
                palette={mochi.palette}
                outfit={piece ? { [row.id]: piece } : {}}
                scale={TILE_SCALE}
                shadow={false}
                animated={false}
                label=""
              />
            </span>
            {#if fromSeason(row.id, piece)}<i class="season" aria-hidden="true"></i>{/if}
          </button>
        {/each}
      </div>
    </div>
  {/each}
  <label class="seasons">
    <span class="text">
      <span>{t.seasons}</span>
      <small>{seasonHint}</small>
    </span>
    <Switch checked={seasonal} onchange={(on) => void wardrobe.setSeasonal(on)} />
  </label>
</div>

<style>
  .wardrobe {
    display: flex;
    flex-direction: column;
    gap: calc(var(--s1) + var(--gap-mark));
    min-width: 0;
  }

  .slot {
    display: grid;
    grid-template-columns: calc(var(--s8) * 2 + var(--s6)) minmax(0, 1fr);
    align-items: center;
    gap: var(--s2);
  }

  .name {
    display: flex;
    flex-direction: column;
    min-width: 0;
    color: var(--ink);
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .name small {
    overflow: hidden;
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pieces {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--s1) - 1px);
  }

  .piece {
    position: relative;
    width: 34px;
    height: 34px;
    flex: none;
    overflow: hidden;
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--surface);
    transition:
      background-color var(--t-quick) var(--ease-out),
      border-color var(--t-quick) var(--ease-out),
      transform var(--t-quick) var(--ease-out);
  }

  .piece:hover {
    border-color: var(--line-2);
    background: var(--surface-2);
  }

  .piece:active {
    transform: scale(var(--press));
  }

  .piece[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  /* Mochi at 48 px, cropped on what the slot dresses. */
  .crop {
    position: absolute;
    left: 50%;
    top: 0;
    translate: -50% var(--crop);
    pointer-events: none;
  }

  /* A piece the season puts on: a small dot in the corner. */
  .season {
    position: absolute;
    right: 3px;
    top: 3px;
    width: var(--dot);
    height: var(--dot);
    border-radius: 50%;
    background: var(--ink-2);
    box-shadow: 0 0 0 2px var(--surface);
  }

  .seasons {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    margin-top: var(--s1);
    padding-top: var(--s2);
    border-top: 1px solid var(--line);
    cursor: pointer;
  }

  .seasons .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-size: var(--fs-3);
  }

  .seasons small {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .piece,
    .piece:active {
      transition: none;
      transform: none;
    }
  }
</style>
