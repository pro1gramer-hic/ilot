<!-- A file Mochi just swallowed (prompt 12, lot 3), 480 px wide: Mochi turned into a box
     in his slot, the file's name, and what to do with it. Its path is already copied for the
     agent, as before; the file itself is not read until a question about it is sent. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { rise } from "../../lib/motion.svelte";
  import { banners } from "../../lib/stores/banners.svelte";
  import { chat } from "../../lib/stores/chat.svelte";
  import { mail } from "../../lib/stores/mail.svelte";

  const t = $derived(i18n.t.island.file);
  const banner = $derived(banners.current?.kind === "file" ? banners.current : undefined);

  function ask(): void {
    if (!banner) return;
    const { attachment } = banner;
    banners.queue.dismissCurrent();
    chat.askAbout(attachment);
  }

  function mailIt(): void {
    if (!banner) return;
    const { attachment } = banner;
    banners.queue.dismissCurrent();
    mail.open(attachment);
  }
</script>

{#if banner}
  <div class="file">
    <span class="slot" aria-hidden="true"></span>
    <div class="text" role="status">
      <b in:rise={{ index: 1 }} title={banner.attachment.name}>{banner.attachment.name}</b>
      <span class="sub" in:rise={{ index: 2 }}>{t.sub}</span>
      <span class="actions" in:rise={{ index: 3 }}>
        <button type="button" class="btn sm primary" onclick={ask}>{t.ask}</button>
        <button type="button" class="btn sm" onclick={mailIt}>{t.mail}</button>
        <button type="button" class="btn sm quiet" onclick={() => banners.queue.dismissCurrent()}>{t.later}</button>
      </span>
    </div>
  </div>
{/if}

<style>
  .file {
    display: flex;
    gap: var(--s3);
    padding: var(--s3) var(--s4) var(--s3) var(--s3);
  }

  .slot {
    width: 56px;
    height: 56px;
    flex: none;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }

  b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-body);
    font-weight: 600;
  }

  .sub {
    margin-top: 2px;
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .actions {
    display: flex;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  .quiet {
    margin-left: auto;
    background: none;
    border-color: transparent;
    box-shadow: none;
    color: var(--ink-3);
  }

  .quiet:hover:not(:disabled) {
    background: var(--surface);
    color: var(--ink);
  }
</style>
