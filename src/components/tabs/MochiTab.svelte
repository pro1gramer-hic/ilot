<!-- Onglet Mochi of the panel (docs/09-DEMO-FINALE.md, sections 6 and 8): a big preview,
     then his silhouette, name, color, animations and heads, and his wardrobe (prompt 12).
     Each choice is saved in the settings at once and answered by a small squash of the
     preview, which wears his outfit; the animations only play in the preview. -->
<script lang="ts">
  import { onDestroy } from "svelte";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { HEAD_IDS, HEAD_LABELS, isHead, type HeadId } from "../../lib/mochi/faces";
  import { KIND_IDS, KIND_LABELS } from "../../lib/mochi/kinds";
  import { ANIMATIONS, type MoveId } from "../../lib/mochi/moves";
  import { PALETTES, PALETTE_IDS } from "../../lib/mochi/palettes";
  import { Spring } from "../../lib/spring.svelte";
  import type { MochiSettings } from "../../lib/ipc";
  import { mood } from "../../lib/stores/mood.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Wardrobe from "../wardrobe/Wardrobe.svelte";

  const MAX_NAME = 14;
  /** Kick given to the preview's squash spring: about one unit of squash at its peak. */
  const SQUASH_KICK = 44;
  const LOVE_MS = 1200;
  /** Preview sizes, in CSS pixels per unit of his grid: 168 px, 56 px tiles and 48 px heads. */
  const PREVIEW_SCALE = 7;
  const KIND_SCALE = 56 / 24;
  const HEAD_SCALE = 2;
  /** These animations keep their own face: a head tried on does not replace it. */
  const OWN_FACE: ReadonlySet<MoveId> = new Set(["squish", "dizzy", "swallow"]);

  const uid = $props.id();
  const t = $derived(i18n.t.settings.mochi);
  const mochi = $derived(settings.value.mochi);
  const head: HeadId = $derived(isHead(mochi.face) ? mochi.face : "idle");

  let move = $state<MoveId>("breathe");
  /** The head last clicked here: the preview wears it over the animation. */
  let tried = $state<HeadId>();
  /** The name being typed, shown under the preview before it is saved. */
  let typed = $state<string>();
  let pointerX = $state<number>();
  let nameField = $state<HTMLInputElement>();

  const shownName = $derived(typed?.trim() || mochi.name);
  const previewFace = $derived(mood.face ?? (tried && !OWN_FACE.has(move) ? tried : undefined));
  // « Suivre »: his eyes follow the pointer, wherever it is on the page.
  const previewLook = $derived(move === "watch" && pointerX !== undefined ? { x: pointerX } : undefined);

  const squash = new Spring(0);

  function save(patch: Partial<MochiSettings>): void {
    void settings.update({ mochi: { ...mochi, ...patch } });
    squash.push(SQUASH_KICK);
  }

  function chooseHead(id: HeadId): void {
    tried = id;
    save({ face: id });
  }

  function onNameInput(event: Event & { currentTarget: HTMLInputElement }): void {
    typed = event.currentTarget.value;
    tried = undefined;
  }

  /** Saves the typed name when the field is left (Enter, Tab, a click elsewhere). */
  function commitName(): void {
    if (typed === undefined) return;
    const name = typed.trim();
    typed = undefined;
    if (!name) {
      if (nameField) nameField.value = mochi.name;
      return;
    }
    if (name === mochi.name) return;
    save({ name });
    mood.flash("love", LOVE_MS);
  }

  // Closing the panel while typing still keeps the new name.
  onDestroy(commitName);
</script>

<svelte:window onpointermove={move === "watch" ? (event) => (pointerX = event.clientX) : undefined} />

<div class="mochi-tab">
  <div class="layout">
    <figure class="preview">
      <div class="stage">
        <div class="squash" style:--squash={squash.value}>
          <Mochi
            {move}
            kind={mochi.kind}
            palette={mochi.palette}
            rest={head}
            face={previewFace}
            look={previewLook}
            scale={PREVIEW_SCALE}
            label={shownName}
            dressed
          />
        </div>
      </div>
      <figcaption>{shownName}</figcaption>
    </figure>

    <div class="fields">
      <div class="field">
        <span class="label" id="{uid}-kind">{t.kind}</span>
        <div class="kinds" role="group" aria-labelledby="{uid}-kind">
          {#each KIND_IDS as id (id)}
            <button type="button" class="kind" aria-pressed={mochi.kind === id} onclick={() => save({ kind: id })}>
              <Mochi kind={id} palette={mochi.palette} rest={head} shadow={false} scale={KIND_SCALE} label="" />
              {KIND_LABELS[id]}
            </button>
          {/each}
        </div>
      </div>

      <label class="field">
        <span class="label">{t.name}</span>
        <input
          bind:this={nameField}
          class="name"
          type="text"
          value={mochi.name}
          maxlength={MAX_NAME}
          spellcheck="false"
          autocomplete="off"
          oninput={onNameInput}
          onchange={commitName}
        />
      </label>

      <div class="field">
        <span class="label" id="{uid}-color">{t.color}</span>
        <div class="swatches" role="group" aria-labelledby="{uid}-color">
          {#each PALETTE_IDS as id (id)}
            {@const { name, tiers } = PALETTES[id]}
            <button
              type="button"
              class="swatch"
              aria-pressed={mochi.palette === id}
              aria-label={name}
              title={name}
              style:--light={tiers[1]}
              style:--dark={tiers[3]}
              onclick={() => save({ palette: id })}
            ></button>
          {/each}
        </div>
      </div>

      <div class="field">
        <span class="label" id="{uid}-animation">{t.animations}</span>
        <div class="chips" role="group" aria-labelledby="{uid}-animation">
          {#each ANIMATIONS as animation (animation.move)}
            <button type="button" class="chip" aria-pressed={move === animation.move} onclick={() => (move = animation.move)}>
              {animation.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="field">
        <span class="label" id="{uid}-head">{t.heads}</span>
        <div class="heads" role="group" aria-labelledby="{uid}-head">
          {#each HEAD_IDS as id (id)}
            <button
              type="button"
              class="head"
              aria-pressed={head === id}
              aria-label={t.head(HEAD_LABELS[id])}
              title={HEAD_LABELS[id]}
              onclick={() => chooseHead(id)}
            >
              <Mochi kind={mochi.kind} palette={mochi.palette} face={id} shadow={false} scale={HEAD_SCALE} label="" />
            </button>
          {/each}
        </div>
      </div>

      <div class="field">
        <span class="label" id="{uid}-wardrobe">{t.wardrobe}</span>
        <div role="group" aria-labelledby="{uid}-wardrobe">
          <Wardrobe onpick={() => squash.push(SQUASH_KICK)} />
        </div>
      </div>

      {#if settings.error}
        <p class="error" role="alert">{i18n.t.settings.saveFailed}</p>
      {/if}
    </div>
  </div>
</div>

<style>
  /* The panel sets the width: below 520 px, the preview goes above the fields. */
  .mochi-tab {
    container-type: inline-size;
  }

  .layout {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: var(--s6);
  }

  .preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s3);
  }

  /* A soft floor under him: lighter where he stands, fading into the panel. */
  .stage {
    display: grid;
    place-items: center;
    padding: var(--s6);
    border-radius: var(--r-open);
    border: 1px solid var(--line);
    background: radial-gradient(circle at 50% 76%, var(--surface-2), var(--panel) 66%);
  }

  .squash {
    transform: scale(calc(1 + 0.06 * var(--squash)), calc(1 - 0.08 * var(--squash)));
    transform-origin: 50% 100%;
  }

  figcaption {
    max-width: 100%;
    overflow: hidden;
    font-family: var(--font-pixel);
    font-size: var(--fs-title);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .label {
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  button {
    transition:
      background-color var(--t-quick) var(--ease-out),
      border-color var(--t-quick) var(--ease-out),
      transform var(--t-quick) var(--ease-out);
  }

  button:active {
    transform: scale(var(--press));
  }

  .kinds {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--s2);
  }

  .kind,
  .head {
    border: 1px solid var(--line);
    background: var(--surface);
  }

  .kind {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s1);
    padding: var(--s2);
    border-radius: var(--r-card);
    font-size: var(--fs-3);
    font-weight: 500;
  }

  .kind:hover,
  .head:hover,
  .chip:hover {
    background: var(--surface-2);
  }

  .kind[aria-pressed="true"],
  .head[aria-pressed="true"],
  .chip[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .name {
    height: calc(var(--s5) * 2);
    padding: 0 var(--s3);
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--surface);
    font-size: var(--fs-body);
    caret-color: var(--accent-text);
    user-select: text;
    outline: none;
  }

  .name:hover {
    border-color: var(--line-2);
  }

  .name:focus-visible {
    border-color: var(--accent);
  }

  .swatches,
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
  }

  /* Each color as his light and shaded tiers, split down the middle. */
  .swatch {
    width: var(--s8);
    height: var(--s8);
    padding: var(--stroke);
    border-radius: var(--r-pill);
    border: var(--stroke) solid transparent;
    background: linear-gradient(90deg, var(--light) 50%, var(--dark) 50%) content-box;
  }

  .swatch:hover {
    border-color: var(--line-2);
  }

  .swatch[aria-pressed="true"] {
    border-color: var(--ink);
  }

  .chip {
    height: var(--s8);
    padding: 0 var(--s3);
    border-radius: var(--r-pill);
    border: 1px solid var(--line);
    background: var(--surface);
    box-shadow: var(--btn-shadow);
    font-size: var(--fs-3);
    font-weight: 500;
  }

  .heads {
    display: grid;
    grid-template-columns: repeat(9, minmax(0, 1fr));
    gap: calc(var(--s1) * 1.5);
  }

  .head {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--r-ctl);
  }

  /* The small heads shrink with their cell, down from 48 px. */
  .head :global(.mochi) {
    width: 100%;
    max-width: calc(var(--s5) * 2);
  }

  .head :global(canvas) {
    width: 100%;
    height: auto;
  }

  .error {
    color: var(--err);
    font-size: var(--fs-2);
  }

  @container (max-width: 520px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    button,
    button:active {
      transition: none;
      transform: none;
    }
  }
</style>
