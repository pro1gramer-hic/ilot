<!-- « Raccourcis »: one global shortcut per action îlot can run now (the others appear when
     their module arrives), changed by pressing the keys; Windows is asked first, and a
     shortcut it or another app already holds is refused with the reason, the old one kept.
     Below, « OK Mochi »: off by default, the microphone on the PC, what Mochi understands. -->
<script lang="ts">
  import { tick } from "svelte";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { ISLAND_ACTION, type ActionId, assignProblem, fromKeyboard, keyCaps, shortcutOf } from "../../lib/shortcuts";
  import { actions } from "../../lib/stores/actions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { voice } from "../../lib/stores/voice.svelte";
  import { usableCommands } from "../../lib/voice";
  import PanelIcon from "../panel/PanelIcon.svelte";
  import Button from "../settings/Button.svelte";
  import Row from "../settings/Row.svelte";
  import Switch from "../settings/Switch.svelte";

  const t = $derived(i18n.t.settings.shortcuts);
  let recording = $state<ActionId | null>(null);
  /** Modifiers held while recording, shown as they are pressed. */
  let held = $state<string[]>([]);
  let saving = $state<ActionId | null>(null);
  let problems = $state<Partial<Record<ActionId, string>>>({});
  let field = $state<HTMLButtonElement>();

  const phrases = $derived(usableCommands(actions.ready).map((c) => c.phrases[0]));

  async function record(id: ActionId): Promise<void> {
    recording = id;
    held = [];
    await tick();
    field?.focus();
  }

  function stop(): void {
    recording = null;
    held = [];
  }

  async function assign(id: ActionId, text: string): Promise<void> {
    stop();
    const problem = assignProblem(settings.value, id, text);
    if (problem) {
      problems[id] = problem;
      return;
    }
    saving = id;
    const refused = await settings.setShortcut(id, text);
    saving = null;
    if (refused) problems[id] = refused;
    else delete problems[id];
  }

  /** Keys pressed while recording: Escape gives up, Backspace or Delete alone removes the
   * shortcut, anything else with its modifiers becomes the new one. Nothing reaches the
   * panel (Escape would close it). */
  function capture(event: KeyboardEvent, id: ActionId): void {
    event.preventDefault();
    event.stopPropagation();
    const alone = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (alone && event.key === "Escape") return stop();
    if (alone && (event.key === "Backspace" || event.key === "Delete") && id !== ISLAND_ACTION) return void assign(id, "");
    const text = fromKeyboard(event);
    if (text) return void assign(id, text);
    holding(event);
  }

  function holding(event: KeyboardEvent): void {
    const keys = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && t.shift, event.metaKey && "Windows"];
    held = keys.filter((key): key is string => typeof key === "string");
  }
</script>

<div class="shortcuts">
  <p class="intro">{t.intro}</p>

  <ul class="list" aria-label={t.list}>
    {#each actions.available as action (action.id)}
      {@const text = shortcutOf(settings.value, action.id)}
      <li>
        <span class="text">
          <b id="shortcut-{action.id}">{action.label}</b>
          <small>{action.hint}</small>
          {#if problems[action.id]}
            <small class="problem" role="alert">{problems[action.id]}</small>
          {/if}
        </span>
        <span class="keys">
          {#if recording === action.id}
            <button
              type="button"
              class="field recording"
              bind:this={field}
              onkeydown={(event) => capture(event, action.id)}
              onkeyup={holding}
              onblur={stop}
              aria-label={t.recording(action.label)}
            >
              {#if held.length}
                {#each held as key (key)}<kbd>{key}</kbd>{/each}<span class="more">…</span>
              {:else}
                <span class="hint">{t.pressKeys}</span>
              {/if}
            </button>
          {:else}
            <button
              type="button"
              class="field"
              disabled={saving === action.id}
              aria-describedby="shortcut-{action.id}"
              aria-label={t.change(action.label, text ? keyCaps(text).join(" ") : null)}
              onclick={() => void record(action.id)}
            >
              {#if text}
                {#each keyCaps(text) as key (key)}<kbd>{key}</kbd>{/each}
              {:else}
                <span class="hint">{t.none}</span>
              {/if}
            </button>
          {/if}
          {#if text && action.id !== ISLAND_ACTION && recording !== action.id}
            <Button variant="quiet" square aria-label={t.remove(action.label)} title={t.removeTip} onclick={() => void assign(action.id, "")}>
              <PanelIcon name="close" />
            </Button>
          {:else}
            <!-- Keeps the keys of every row in one column. -->
            <span class="spacer" aria-hidden="true"></span>
          {/if}
        </span>
      </li>
    {/each}
  </ul>

  <section class="voice" aria-labelledby="voice-title">
    <h3 class="label" id="voice-title">{t.voice}</h3>
    <div class="rows">
      <Row label title={t.listen} hint={t.listenHint}>
        <Switch checked={settings.value.voice.listen} onchange={(on) => settings.update({ voice: { ...settings.value.voice, listen: on } })} />
      </Row>
      <Row label title={t.speak} hint={t.speakHint}>
        <Switch checked={settings.value.voice.speak} onchange={(on) => settings.update({ voice: { ...settings.value.voice, speak: on } })} />
      </Row>
    </div>
    {#if voice.error}
      <p class="problem" role="alert">{voice.error}</p>
    {/if}
    <p class="says" id="voice-phrases">{t.say}</p>
    <ul class="phrases" aria-labelledby="voice-phrases">
      {#each phrases as phrase (phrase)}
        <li>{phrase}</li>
      {/each}
    </ul>
    <p class="privacy">{t.privacy}</p>
  </section>
</div>

<style>
  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }

  .intro,
  .says,
  .privacy {
    color: var(--ink-2);
    font-size: var(--fs-3);
    line-height: 1.5;
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

  .problem {
    color: var(--err);
    font-size: var(--fs-2);
  }

  .keys {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s1);
  }

  /* The shortcut itself is the button: its keys, or « Aucun ». */
  .field {
    min-width: calc(5 * var(--s8));
    height: var(--h-ctl);
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--s1);
    padding: 0 var(--s2);
    border-radius: var(--r-ctl);
    border: 1px solid transparent;
    transition:
      background var(--t-quick) var(--ease-out),
      border-color var(--t-quick) var(--ease-out);
  }

  .field:hover:not(:disabled) {
    background: var(--hover);
  }

  .field:disabled {
    opacity: var(--dim);
    cursor: progress;
  }

  .field.recording {
    justify-content: center;
    background: var(--accent-soft);
    border-color: var(--accent-text);
  }

  kbd {
    padding: calc(var(--s1) / 2) var(--s2);
    border-radius: var(--r-chip);
    background: var(--surface);
    border: 1px solid var(--line);
    box-shadow: var(--hi);
    color: var(--ink);
    font: 500 var(--fs-1) var(--font-ui);
  }

  .hint,
  .more {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .recording .hint {
    color: var(--accent-text);
  }

  .voice {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  .label {
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

  .rows > :global(:first-child) {
    border-top: 0;
  }

  .spacer {
    width: var(--h-ctl);
  }

  /* What Mochi understands, one phrase per chip. */
  .phrases {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
  }

  .phrases li {
    padding: var(--s1) var(--s3);
    border-radius: var(--r-pill);
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--ink);
    font-size: var(--fs-3);
  }

  .privacy {
    color: var(--ink-3);
  }

  @container panel (max-width: 820px) {
    .list li {
      flex-direction: column;
      align-items: flex-start;
    }

    .field {
      justify-content: flex-start;
    }
  }
</style>
