<!-- Réglages › Général (prompt 12, lot 1): which screen holds the island, or the island
     that follows the cursor from one screen to another; the island that hides while
     nothing goes on, back when the cursor touches the screen's edge; and the language of
     the interface (lot 5), which changes at once. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { LOCALES, type Locale } from "../../lib/i18n/locale";
  import { type Language, type ScreenView, islandToScreen, listScreens } from "../../lib/ipc";
  import { settings } from "../../lib/stores/settings.svelte";
  import Segmented from "../panel/Segmented.svelte";
  import Row from "../settings/Row.svelte";
  import Switch from "../settings/Switch.svelte";

  const uid = $props.id();
  const FOLLOW = "suit";
  const LANGUAGES: readonly Language[] = ["auto", ...LOCALES];

  /** A language's name in that language: « Français », « English ». */
  function autonym(locale: Locale): string {
    const name = new Intl.DisplayNames([locale], { type: "language" }).of(locale) ?? locale;
    return name.charAt(0).toLocaleUpperCase(locale) + name.slice(1);
  }

  const t = $derived(i18n.t.settings.general);
  let screens = $state<ScreenView[] | null>(null);
  let failed = $state(false);
  const display = $derived(settings.value.display);
  const options = $derived<[string, string][]>([...(screens ?? []).map((s): [string, string] => [s.id, s.label]), [FOLLOW, t.followMouse]]);
  const value = $derived(display.followMouse ? FOLLOW : (screens?.find((s) => s.current)?.id ?? FOLLOW));
  const details = $derived((screens ?? []).map((s) => t.screenDetail(s.label, s.detail, s.primary, s.current)).join(" "));
  const languages = $derived<[string, string][]>([["auto", t.languageAuto], ...LOCALES.map((l): [string, string] => [l, autonym(l)])]);

  function chooseLanguage(id: string): void {
    const language = LANGUAGES.find((l) => l === id);
    if (language) void settings.update({ language });
  }

  async function load(): Promise<void> {
    try {
      screens = await listScreens();
      failed = false;
    } catch {
      failed = true;
    }
  }

  onMount(() => void load());

  async function choose(id: string): Promise<void> {
    if (id === FOLLOW) {
      await settings.update({ display: { ...display, followMouse: true } });
      return;
    }
    await settings.update({ display: { ...display, followMouse: false } });
    await islandToScreen(id);
    await load();
  }
</script>

<div class="general">
  <section aria-labelledby="{uid}-screen">
    <div class="block">
      <span class="text">
        <b id="{uid}-screen">{t.screen}</b>
        <small>
          {#if failed}
            {t.screensFailed}
          {:else if screens === null}
            {t.findingScreens}
          {:else if screens.length < 2}
            {t.oneScreen} {details}
          {:else}
            {details}
          {/if}
        </small>
      </span>
      {#if screens}
        <Segmented {options} {value} labelledby="{uid}-screen" onchange={(id) => void choose(id)} />
      {/if}
    </div>
    <p class="hint">{display.followMouse ? t.followsMouse : t.staysPut}</p>
  </section>

  <div class="rows">
    <Row label title={t.startup} hint={t.startupHint}>
      <Switch checked={settings.value.launchAtStartup} onchange={(on) => settings.update({ launchAtStartup: on })} />
    </Row>
    <Row label title={t.fadeWhenIdle} hint={t.fadeWhenIdleHint} index={1}>
      <Switch checked={display.fadeWhenIdle} onchange={(on) => settings.update({ display: { ...display, fadeWhenIdle: on } })} />
    </Row>
    <Row title={t.language} titleId="{uid}-language" hint={t.languageHint} index={2}>
      <Segmented options={languages} value={settings.value.language} labelledby="{uid}-language" onchange={chooseLanguage} />
    </Row>
  </div>
  {#if settings.error}
    <p class="error" role="alert">{settings.error}</p>
  {/if}
</div>

<style>
  .general {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
  }

  .block {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s3) var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
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

  .hint {
    margin-top: var(--s2);
    padding: 0 var(--s4);
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .rows {
    padding: 0 var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .rows > :global(:first-child) {
    border-top: 0;
  }

  .error {
    color: var(--err);
    font-size: var(--fs-3);
  }

  @container panel (max-width: 820px) {
    .block {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
