<!-- « OK Mochi » asks before anything leaves the PC (500 × auto, laid out like a message):
     what Mochi heard, exactly what Claude would receive, « Envoyer à Claude » and
     « Annuler ». Then Claude's answer, which Mochi also says aloud, or why there is none
     (no key: nothing was sent). Mochi sits in the slot on the right. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { rise } from "../../lib/motion.svelte";
  import { voice } from "../../lib/stores/voice.svelte";
  import PanelIcon from "../panel/PanelIcon.svelte";

  const t = $derived(i18n.t.island.voice);
  const ask = $derived(voice.ask);
  const lines = $derived(ask ? ask.context.split("\n") : []);
</script>

{#if ask}
  <div class="voice">
    <div class="row">
      <span class="mic" aria-hidden="true"><PanelIcon name="mic" /></span>
      <div class="text">
        <span class="caps" in:rise={{ index: 1 }}>{t.now}</span>
        <b in:rise={{ index: 2 }}>{t.quote(ask.question.replace(/\.$/, ""))}</b>
      </div>
      <span class="slot" aria-hidden="true"></span>
    </div>

    {#if ask.status === "confirm"}
      <p class="lead" in:rise={{ index: 3 }}>{t.lead}</p>
      <ul class="sent" in:rise={{ index: 3 }} aria-label={t.sent}>
        {#each lines as line, index (index)}
          <li>{line}</li>
        {/each}
      </ul>
      <div class="actions" in:rise={{ index: 4 }}>
        <button type="button" class="btn sm primary" onclick={() => void voice.confirm()}>{t.send}</button>
        <button type="button" class="btn sm" onclick={() => voice.close()}>{t.cancel}</button>
      </div>
    {:else if ask.status === "sending"}
      <p class="lead waiting" role="status">{t.thinking}</p>
      <div class="actions">
        <button type="button" class="btn sm" onclick={() => voice.close()}>{t.cancel}</button>
      </div>
    {:else}
      <p class="answer" class:failed={ask.status === "error"} role={ask.status === "error" ? "alert" : "status"} in:rise={{ index: 3 }}>
        {ask.answer}
      </p>
      <div class="actions" in:rise={{ index: 4 }}>
        {#if ask.status === "answer"}
          <button type="button" class="btn sm" onclick={() => voice.repeat()}>{t.repeat}</button>
        {/if}
        <button type="button" class="btn sm" onclick={() => voice.close()}>{t.close}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .voice {
    padding: var(--s3) var(--s4);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }

  /* The microphone on a surface, like an app's icon on a message. */
  .mic {
    width: 36px;
    height: 36px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 11px;
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--accent-text);
  }

  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }

  .caps {
    margin-bottom: 1px;
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Mochi floats over this slot (IslandMochi). */
  .slot {
    width: 36px;
    height: 36px;
    flex: none;
  }

  .lead {
    margin-top: var(--s3);
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .waiting {
    color: var(--ink);
  }

  /* Exactly what leaves the PC, set apart like a quote of data. */
  .sent {
    list-style: none;
    margin-top: var(--s2);
    padding: var(--s2) var(--s3);
    border-radius: var(--r-chip);
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--ink);
    font-size: var(--fs-2);
    line-height: 1.5;
  }

  .answer {
    margin-top: var(--s3);
    max-height: 132px;
    overflow-y: auto;
    color: var(--ink);
    font-size: var(--fs-body);
    line-height: 1.5;
  }

  .answer.failed {
    color: var(--ink-2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    margin-top: var(--s3);
  }
</style>
