<!-- Answer to a message, in the island's message opening and under a thread of the Messages
     tab: three quick replies in the tone of the app, a field, and the button. Enter answers,
     Escape cancels. Windows lets no app answer another app's notification, so îlot copies
     the reply and brings the app to the front (conversations.svelte.ts), and says so: the
     button and the line under it never claim the message was sent.
     It answers the banner's `notification`, or a thread's `target`. `ondone` gets the
     outcome once the reply is copied (the island shows it as a toast and closes); a failed
     copy stays in the box with its reason. `oncancel` closes the box (without it, Escape
     only clears the field). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { firstName, quickReplies } from "../../lib/notifications";
  import type { NotificationItem } from "../../lib/notifications";
  import { type ReplyOutcome, type ReplyTarget, bannerTarget, conversations } from "../../lib/stores/conversations.svelte";
  import { arrive } from "../music/arrive";

  let {
    notification,
    target: given,
    ondone,
    oncancel,
    autofocus = true,
  }: {
    notification?: NotificationItem;
    target?: ReplyTarget;
    ondone?: (outcome: ReplyOutcome) => void;
    oncancel?: () => void;
    autofocus?: boolean;
  } = $props();

  const t = $derived(i18n.t.messages.reply);
  const target = $derived(given ?? (notification ? bannerTarget(notification) : null));

  /** Longest reply, as the demo's field. */
  const MAX_LENGTH = 280;

  let draft = $state("");
  let busy = $state(false);
  let outcome = $state<ReplyOutcome | null>(null);
  let field = $state<HTMLInputElement>();

  const quick = $derived(target ? quickReplies(target.app) : []);
  const name = $derived(target ? firstName(target.sender) || target.appLabel : "");

  $effect(() => {
    if (autofocus) field?.focus({ preventScroll: true });
  });

  async function answer(text: string): Promise<void> {
    if (busy || !target || !text.trim()) return;
    busy = true;
    try {
      outcome = await conversations.reply(target, text);
      if (outcome.ok) {
        draft = "";
        ondone?.(outcome);
      }
    } finally {
      busy = false;
    }
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void answer(draft);
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key !== "Escape") return;
    if (oncancel) oncancel();
    else if (draft) draft = "";
    else return;
    event.stopPropagation();
    outcome = null;
  }
</script>

{#if target}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="reply" role="group" aria-label={t.to(name)} onkeydown={keydown}>
    <div class="quick" in:arrive={{ index: 0 }}>
      {#each quick as text (text)}
        <button type="button" class="chip" disabled={busy} onclick={() => answer(text)}>{text}</button>
      {/each}
    </div>
    <form class="row" onsubmit={submit} in:arrive={{ index: 1 }}>
      <input
        bind:this={field}
        bind:value={draft}
        maxlength={MAX_LENGTH}
        placeholder={t.to(name)}
        aria-label={t.field(name)}
        aria-describedby="reply-hint-{target.notificationId}"
        autocomplete="off"
        oninput={() => (outcome = null)}
      />
      <button type="submit" class="send" disabled={busy || !draft.trim()}>{t.send}</button>
    </form>
    <p id="reply-hint-{target.notificationId}" class="hint" class:err={outcome && !outcome.ok} role="status" in:arrive={{ index: 2 }}>
      {outcome ? outcome.message : t.hint(target.appLabel)}
    </p>
  </div>
{/if}

<style>
  .reply {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }

  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--s1) + var(--gap-mark));
  }

  .chip {
    height: 32px;
    padding: 0 var(--s3);
    border-radius: var(--r-pill);
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--fs-2);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background var(--t-quick),
      border-color var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .chip:hover:enabled {
    background: var(--surface-2);
    border-color: var(--line-2);
  }

  .chip:active:enabled {
    transform: scale(var(--press));
  }

  .row {
    display: flex;
    gap: var(--s2);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 var(--s3);
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--fs-3);
    user-select: text;
    transition:
      border-color var(--t-quick),
      box-shadow var(--t-quick);
  }

  input::placeholder {
    color: var(--ink-3);
  }

  input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .send {
    flex: none;
    height: 36px;
    padding: 0 var(--s4);
    border-radius: var(--r-ctl);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--fs-3);
    font-weight: 600;
    white-space: nowrap;
    box-shadow: var(--btn-shadow);
    transition:
      transform var(--t-quick) var(--ease-out),
      filter var(--t-quick);
  }

  .send:hover:enabled {
    filter: brightness(1.08);
  }

  .send:active:enabled {
    transform: scale(var(--press));
  }

  .send:disabled {
    opacity: var(--dim);
    cursor: default;
  }

  .chip:disabled {
    opacity: var(--dim);
    cursor: progress;
  }

  .hint {
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.4;
  }

  .hint.err {
    color: var(--err);
  }

  @media (prefers-reduced-motion: reduce) {
    .chip:active:enabled,
    .send:active:enabled {
      transform: none;
    }
  }
</style>
