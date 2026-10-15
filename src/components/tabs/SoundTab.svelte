<!-- Réglages › Son (prompt 12, lot 1): the volume, a switch that cuts every sound (also in
     one click from the hub's bar), the twenty sounds made for îlot with Écouter, and the
     folder where a file of the same name replaces one (finish.wav, approval.mp3…). -->
<script lang="ts">
  import { onMount } from "svelte";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { SOUND_IDS, SOUNDS, type SoundId } from "../../lib/sound/catalog";
  import { settings } from "../../lib/stores/settings.svelte";
  import { sound } from "../../lib/stores/sound.svelte";
  import Button from "../settings/Button.svelte";
  import Row from "../settings/Row.svelte";
  import Switch from "../settings/Switch.svelte";

  const uid = $props.id();
  const t = $derived(i18n.t.settings.sound);
  const current = $derived(settings.value.sound);
  /** The volume while the slider moves, saved when it is let go. */
  let sliding = $state<number>();
  const percent = $derived(Math.round((sliding ?? current.volume) * 100));

  onMount(() => void sound.refresh());

  function save(patch: Partial<typeof current>): void {
    void settings.update({ sound: { ...current, ...patch } });
  }

  function onvolume(event: Event & { currentTarget: HTMLInputElement }): void {
    sliding = Number(event.currentTarget.value) / 100;
  }

  async function onvolumeset(event: Event & { currentTarget: HTMLInputElement }): Promise<void> {
    const volume = Number(event.currentTarget.value) / 100;
    sliding = undefined;
    await settings.update({ sound: { ...current, volume } });
    sound.play("toast", { preview: true });
  }

  /** What the row says about a sound's file. */
  function fileNote(id: SoundId): { text: string; tone: "own" | "replaced" | "off" } {
    const file = sound.fileOf(id);
    if (!file) return { text: `${id}.wav`, tone: "own" };
    if (!file.usable) return { text: t.tooHeavy(file.file), tone: "off" };
    if (sound.broken.includes(id)) return { text: t.unreadable(file.file), tone: "off" };
    return { text: t.replacedBy(file.file), tone: "replaced" };
  }
</script>

<div class="sound">
  <div class="rows">
    <Row title={t.volume} titleId="{uid}-volume" hint={t.volumeHint}>
      <input
        class="volume"
        type="range"
        min="0"
        max="100"
        step="5"
        value={Math.round(current.volume * 100)}
        aria-labelledby="{uid}-volume"
        aria-valuetext={t.percent(percent)}
        style:--fill="{percent}%"
        oninput={onvolume}
        onchange={onvolumeset}
      />
      <output class="percent">{t.percent(percent)}</output>
    </Row>
    <Row label title={t.mute} hint={t.muteHint}>
      <Switch checked={current.muted} onchange={(muted) => save({ muted })} />
    </Row>
  </div>
  <p class="rules">{t.rules}</p>

  <section aria-labelledby="{uid}-list">
    <h3 class="label" id="{uid}-list">{t.list}</h3>
    <ul class="list">
      {#each SOUND_IDS as id (id)}
        {@const note = fileNote(id)}
        <li>
          <Button variant="quiet" square aria-label={t.listenTo(SOUNDS[id].label)} title={t.listen} onclick={() => sound.play(id, { preview: true })}>
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5v9l7-4.5z" /></svg>
          </Button>
          <span class="name">
            <b>{SOUNDS[id].label}</b>
            <small class={note.tone}>{note.text}</small>
          </span>
        </li>
      {/each}
    </ul>
  </section>

  <section class="folder" aria-labelledby="{uid}-folder">
    <h3 class="label" id="{uid}-folder">{t.replace}</h3>
    <p>{t.folder.before} <code>finish.wav</code>, <code>approval.mp3</code>{t.folder.after}</p>
    <div class="actions">
      <Button variant="plain" onclick={() => void sound.openFolder()}>{t.openFolder}</Button>
      <Button variant="quiet" onclick={() => void sound.refresh()}>{t.rereadFolder}</Button>
    </div>
    {#if sound.error}
      <p class="error" role="alert">{sound.error}</p>
    {/if}
  </section>
</div>

<style>
  .sound {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
  }

  .rows {
    padding: 0 var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .rows > :global(:first-child) {
    border-top: 0;
  }

  /* A thin slider: the played part in the accent, a round knob. */
  .volume {
    appearance: none;
    width: calc(var(--s8) * 5);
    height: var(--switch-h);
    background: transparent;
    cursor: pointer;
  }

  .volume::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--r-pill);
    background: linear-gradient(90deg, var(--accent) var(--fill), var(--well-2) var(--fill));
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .volume::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: var(--ink);
    box-shadow: var(--shadow-soft);
    transition: transform var(--t-quick) var(--ease-out);
  }

  .volume:active::-webkit-slider-thumb {
    transform: scale(1.15);
  }

  .percent {
    min-width: calc(var(--s8) + var(--s2));
    color: var(--ink-2);
    font-size: var(--fs-2);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .rules {
    margin-top: calc(-1 * var(--s2));
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .label {
    margin-bottom: var(--s2);
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .list {
    list-style: none;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s1) var(--s4);
  }

  .list li {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
  }

  .list svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }

  .name {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name b {
    font-size: var(--fs-3);
    font-weight: 500;
  }

  .name small {
    overflow: hidden;
    color: var(--ink-3);
    font-family: var(--font-mono);
    font-size: var(--fs-1);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name small.replaced {
    color: var(--ink);
  }

  .name small.off {
    color: var(--err);
  }

  .folder p {
    color: var(--ink-2);
    font-size: var(--fs-3);
    line-height: 1.5;
  }

  code {
    font-family: var(--font-mono);
    font-size: var(--fs-2);
    color: var(--ink);
  }

  .actions {
    display: flex;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  .error {
    margin-top: var(--s2);
    color: var(--err);
    font-size: var(--fs-3);
  }

  @container panel (max-width: 820px) {
    .list {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .volume:active::-webkit-slider-thumb {
      transform: none;
    }
  }
</style>
