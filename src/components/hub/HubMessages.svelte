<!-- The hub's messages (09, section 8 bis): the latest threads, each with two quick replies
     in one click. Windows lets no app answer another app's notification: as everywhere in
     îlot (ReplyBox), the reply is copied and the app comes to the front, and the row says
     so. It never types into the other app. Masked content stays masked, and only a line
     shown in full may be translated (DeepL, prompt 12). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { type Conversation, lastLine, quickReplies } from "../../lib/notifications";
  import { banners } from "../../lib/stores/banners.svelte";
  import { conversations, replyTarget } from "../../lib/stores/conversations.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Avatar from "../messages/Avatar.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import TranslateButton from "../translate/TranslateButton.svelte";
  import { swap } from "../translate/swap";

  /** How long a row says what happened to its reply. */
  const OUTCOME_MS = 4000;

  const t = $derived(i18n.t.hub.messages);
  const hidden = $derived(settings.value.notifications.hideContent);
  /** Threads someone wrote in: the user's own replies are not news. */
  const threads = $derived(conversations.list.filter((c) => c.messages.some((m) => !m.mine)));

  let outcomes = $state<Record<string, { ok: boolean; message: string }>>({});
  let busy = $state<string | null>(null);

  async function answer(conversation: Conversation, text: string): Promise<void> {
    if (busy) return;
    busy = conversation.key;
    try {
      const outcome = await conversations.reply(replyTarget(conversation), text);
      outcomes[conversation.key] = outcome;
      // The app now in front may fold the hub: the island says it too, as for a banner.
      if (outcome.ok) banners.toast("ok", outcome.message);
      setTimeout(() => delete outcomes[conversation.key], OUTCOME_MS);
    } finally {
      busy = null;
    }
  }
</script>

<div class="box">
  {#if threads.length > 0}
    <ul aria-label={t.list}>
      {#each threads as conversation (conversation.key)}
        {@const outcome = outcomes[conversation.key]}
        {@const last = conversation.messages.at(-1)}
        {@const theirs = last && !last.mine ? last : null}
        {@const veiled = hidden || !theirs || theirs.masked || !theirs.text}
        {@const line = theirs && !veiled ? translations.shown(theirs.id, lastLine(conversation, hidden)) : lastLine(conversation, hidden)}
        <li class="translate-host" class:sent={outcome?.ok}>
          <Avatar sender={conversation.sender} app={conversation.app} size={30} />
          <div class="text">
            <span class="name">
              <b>{conversation.sender}</b>
              {#if theirs && !outcome}
                <TranslateButton id={theirs.id} text={theirs.text} hidden={veiled} reveal />
              {/if}
            </span>
            <p class:err={outcome && !outcome.ok} role="status">
              {#if outcome}{outcome.message}{:else}{#key line}<span in:swap>{line}</span>{/key}{/if}
            </p>
          </div>
          <div class="quick" role="group" aria-label={t.quick(conversation.sender, conversation.appLabel)}>
            {#each quickReplies(conversation.app).slice(0, 2) as text (text)}
              <button type="button" class="chip" disabled={busy !== null} onclick={() => answer(conversation, text)}>{text}</button>
            {/each}
          </div>
        </li>
      {/each}
    </ul>
  {:else}
    <div class="empty">
      <b>{t.empty}</b>
      <p>{t.emptyHint}</p>
      <button type="button" class="btn sm" onclick={() => island.openPanel("messages")}>{t.choose}</button>
    </div>
  {/if}
</div>

<style>
  .box {
    height: 100%;
    /* A long line (a translation is often longer) is cut here, never widens the hub. */
    contain: inline-size;
    overflow-y: auto;
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    /* The app's dot on each avatar is cut out of this card. */
    --avatar-ring: var(--surface);
  }

  ul {
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
    transition: background-color 0.7s var(--ease-out);
  }

  li + li {
    border-top: 1px solid var(--line);
  }

  li.sent {
    background: var(--ok-soft);
    transition-duration: var(--t-quick);
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  b {
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--s1);
    min-width: 0;
  }

  .name b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text p {
    color: var(--ink-2);
    font-size: var(--fs-sub);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .text p.err {
    color: var(--err);
  }

  .quick {
    flex: none;
    display: flex;
    gap: var(--s1);
  }

  .chip {
    height: 32px;
    padding: 0 var(--s3);
    border-radius: var(--r-pill);
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: var(--fs-sub);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color var(--t-quick),
      box-shadow var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .chip:hover:not(:disabled) {
    box-shadow: inset 0 0 0 1px var(--line-2);
    background: var(--hover);
  }

  .chip:active:not(:disabled) {
    transform: scale(var(--press));
  }

  .chip:disabled {
    color: var(--ink-3);
    cursor: progress;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s2);
    padding: var(--s4);
    text-align: center;
  }

  .empty p {
    max-width: 44ch;
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .chip:active:not(:disabled) {
      transform: none;
    }
  }
</style>
