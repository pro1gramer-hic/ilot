<!-- « Modes »: how each feature shows itself (docs/09-DEMO-FINALE.md, section 7), one
     segmented control per feature, saved at once (settings.modes). Below, the general
     settings that have no other place: full screen, do not disturb, end of tasks, start
     with Windows and the shortcut. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { MODES, type ModeKey, type Modes } from "../../lib/modes";
  import { settings } from "../../lib/stores/settings.svelte";
  import Segmented from "../panel/Segmented.svelte";
  import Row from "../settings/Row.svelte";
  import Switch from "../settings/Switch.svelte";

  const t = $derived(i18n.t.settings.modes);
  const keys = Object.keys(MODES) as ModeKey[];
  const shortcut = $derived(settings.value.shortcut.split("+").map((k) => (k === "Space" ? t.space : k)));

  function choose(key: ModeKey, value: string): void {
    void settings.update({ modes: { ...settings.value.modes, [key]: value } as Modes });
  }
</script>

<div class="modes">
  <p class="intro">{t.intro}</p>
  <ul class="list">
    {#each keys as key (key)}
      <li>
        <span class="text">
          <b id="mode-{key}">{MODES[key].label}</b>
          <small>{t.hints[key]}</small>
        </span>
        <Segmented
          options={MODES[key].options}
          value={settings.value.modes[key]}
          labelledby="mode-{key}"
          onchange={(value) => choose(key, value)}
        />
      </li>
    {/each}
  </ul>

  <section class="general" aria-labelledby="general-title">
    <h3 class="label" id="general-title">{t.general}</h3>
    <div class="rows">
      <Row label title={t.fullscreen} hint={t.fullscreenHint}>
        <Switch checked={settings.value.hideInFullscreen} onchange={(on) => settings.update({ hideInFullscreen: on })} />
      </Row>
      <Row label title={t.doNotDisturb} hint={t.doNotDisturbHint}>
        <Switch checked={settings.value.doNotDisturb} onchange={(on) => settings.update({ doNotDisturb: on })} />
      </Row>
      <Row label title={t.announceFinished} hint={t.announceFinishedHint}>
        <Switch checked={settings.value.announceFinished} onchange={(on) => settings.update({ announceFinished: on })} />
      </Row>
      <Row title={t.panelShortcut}>
        <span class="keys">
          {#each shortcut as key (key)}<kbd>{key}</kbd>{/each}
        </span>
      </Row>
    </div>
    {#if settings.error}
      <p class="error" role="alert">{settings.error}</p>
    {/if}
  </section>
</div>

<style>
  .modes {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }

  .intro {
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .list {
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s3) var(--s4);
  }

  .list li + li {
    border-top: 1px solid var(--line);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  small {
    margin-top: calc(var(--s1) / 2);
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  .general {
    margin-top: var(--s3);
  }

  .label {
    margin-bottom: var(--s1);
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .rows {
    padding: 0 var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  /* The rows draw a line above each other, never above the first. */
  .rows > :global(:first-child) {
    border-top: 0;
  }

  .keys {
    display: flex;
    gap: var(--s1);
  }

  kbd {
    padding: calc(var(--s1) / 2) var(--s2);
    border-radius: var(--r-chip);
    background: var(--surface);
    border: 1px solid var(--line);
    box-shadow: var(--hi);
    color: var(--ink-2);
    font: 500 var(--fs-1) var(--font-ui);
  }

  .error {
    margin-top: var(--s2);
    color: var(--err);
    font-size: var(--fs-3);
  }

  /* A narrow panel puts each control under its text. */
  @container panel (max-width: 820px) {
    .list li {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
