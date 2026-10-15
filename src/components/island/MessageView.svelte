<!-- A message (500 × auto, docs/09-DEMO-FINALE.md, section 4): the app's icon, the sender,
     a preview (hidden when « Masquer le contenu » or « Expéditeur seul » asks for it), and
     Répondre, Ouvrir, Marquer comme lu, Ignorer. Replying holds the opening open: Windows
     does not let îlot answer in another app, so the reply is copied and the app opened.
     « Traduire » (DeepL, prompt 12) closes the preview's line, never on a hidden one. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { rise } from "../../lib/motion.svelte";
  import { appColor, bannerParts } from "../../lib/notifications";
  import { banners } from "../../lib/stores/banners.svelte";
  import { conversations } from "../../lib/stores/conversations.svelte";
  import { notifications } from "../../lib/stores/notifications.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import AppTile from "../AppTile.svelte";
  import ReplyBox from "../messages/ReplyBox.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import TranslateButton from "../translate/TranslateButton.svelte";
  import { swap } from "../translate/swap";

  const t = $derived(i18n.t.island.message);
  const banner = $derived(banners.current);
  const message = $derived(
    banner?.kind === "message" && notifications.current?.id === banner.id ? notifications.current : null,
  );
  const hidden = $derived(
    message !== null && (message.masked || settings.value.modes.messages === "expediteur" || !message.text),
  );
  const title = $derived(message ? (message.count > 1 ? bannerParts(message).map((p) => p.text).join("") : message.sender || message.appLabel) : "");

  let replying = $state(false);
  $effect(() => {
    banners.hold("reply", replying);
    return () => banners.hold("reply", false);
  });

  function close(): void {
    replying = false;
    banners.queue.dismissCurrent();
  }

  /** The reply is copied and the app opened: the island says so in a toast. */
  function replied(outcome: { message: string }): void {
    close();
    banners.toast("ok", outcome.message);
  }

  /** « Marquer comme lu »: the thread in the Messages tab no longer counts it. */
  function read(): void {
    if (message) conversations.readNotification(message.id);
    close();
  }

  async function open(): Promise<void> {
    if (!message) return;
    conversations.readNotification(message.id);
    await notifications.open(message.id);
    close();
  }
</script>

{#if message}
  <div class="message">
    <div class="row">
      <span class="app" style:--app={appColor(message.app)}>
        <AppTile icon={message.icon} monogram={message.monogram} size={22} />
      </span>
      <div class="text">
        <span class="caps" in:rise={{ index: 1 }}>{t.now(message.appLabel)}</span>
        <b in:rise={{ index: 2 }}>{title}</b>
      </div>
      <span class="slot" aria-hidden="true"></span>
    </div>
    {#if message.count === 1}
      {@const shown = hidden ? t.masked : translations.shown(message.id, message.text)}
      <div class="said" in:rise={{ index: 3 }}>
        <p class="preview">{#key shown}<span in:swap>{shown}</span>{/key}</p>
        <TranslateButton id={message.id} text={message.text} {hidden} />
      </div>
    {/if}
    {#if replying}
      <ReplyBox notification={message} ondone={replied} oncancel={() => (replying = false)} />
    {:else}
      <div class="actions" in:rise={{ index: 4 }}>
        <button type="button" class="btn sm primary" onclick={() => (replying = true)}>{t.reply}</button>
        <button type="button" class="btn sm" onclick={open}>{t.open}</button>
        <button type="button" class="btn sm" onclick={read}>{t.read}</button>
        <button type="button" class="btn sm" onclick={close}>{t.ignore}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .message {
    padding: var(--s3) var(--s4);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }

  /* The app's icon on a surface, its color in a small dot only. */
  .app {
    position: relative;
    width: 36px;
    height: 36px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 11px;
    background: var(--surface);
    border: 1px solid var(--line);
  }

  .app::after {
    content: "";
    position: absolute;
    right: -2px;
    top: -2px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--app);
    box-shadow: 0 0 0 2px var(--island);
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

  .preview {
    margin-top: 2px;
    color: var(--ink-2);
    font-size: var(--fs-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    margin-top: var(--s3);
  }

  /* The preview's line, and « Traduire » at its end when there is something to read. */
  .said {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-top: 2px;
  }

  .said .preview {
    flex: 1;
    min-width: 0;
    margin-top: 0;
  }
</style>
