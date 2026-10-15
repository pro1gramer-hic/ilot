<!-- Messages tab of the panel (docs/09-DEMO-FINALE.md, section 8): one switch per app, as
     pills; the conversations of this run (with what is unread) beside the open thread, in
     bubbles, the user's own replies in the accent color; quick replies and a field under
     the thread; « Masquer le contenu des messages ». Everything here lives in memory only
     (conversations.svelte.ts); the masking hides every received text, in the list too. A
     received bubble shown in full can be translated (DeepL, prompt 12). -->
<script lang="ts">
  import { untrack } from "svelte";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import type { NotificationSettings } from "../../lib/ipc";
  import { type Conversation, appColor, lastLine, messageText, unreadOf } from "../../lib/notifications";
  import { conversations, replyTarget } from "../../lib/stores/conversations.svelte";
  import { notifications } from "../../lib/stores/notifications.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Avatar from "../messages/Avatar.svelte";
  import { translations } from "../../lib/stores/translate.svelte";
  import TranslateButton from "../translate/TranslateButton.svelte";
  import { swap } from "../translate/swap";
  import ReplyBox from "../messages/ReplyBox.svelte";
  import { arrive } from "../music/arrive";
  import Button from "../settings/Button.svelte";
  import Switch from "../settings/Switch.svelte";

  const t = $derived(i18n.t.panel.messages);
  const value = $derived(settings.value.notifications);
  const hidden = $derived(value.hideContent);
  const selected = $derived(conversations.selected);
  const access = $derived(notifications.status.access);

  /** The apps turned on (and those with a thread) are shown; the others wait behind a
   * button. An app turned off here keeps its pill until the tab closes. */
  let allApps = $state(false);
  const onAtFirst = untrack(() => new Set(value.allowed));
  const usedRules = $derived(
    value.rules.filter(
      (rule) => onAtFirst.has(rule.id) || value.allowed.includes(rule.id) || conversations.list.some((c) => c.app === rule.id),
    ),
  );
  const shownRules = $derived(allApps || usedRules.length === 0 ? value.rules : usedRules);
  const hiddenRules = $derived(value.rules.length - shownRules.length);

  let bubbles = $state<HTMLElement>();
  let list = $state<HTMLElement>();

  const clock = new Intl.DateTimeFormat("fr-FR", { hour: "2-digit", minute: "2-digit" });
  const time = (at: number): string => clock.format(at);

  function patch(change: Partial<NotificationSettings>): void {
    void settings.update({ notifications: { ...value, ...change } });
  }

  function toggleApp(id: string, on: boolean): void {
    const others = value.allowed.filter((allowed) => allowed !== id);
    patch({ allowed: on ? [...others, id] : others });
  }

  // Opening the tab shows what is new: the newest thread with something unread, else the
  // newest thread. The thread on screen is read, and stays the open one.
  conversations.selectedKey = null;
  // Pinned at once, so that reading it does not hand the screen to another thread.
  $effect(() => {
    const open = selected;
    if (open && (conversations.selectedKey !== open.key || unreadOf(open) > 0)) untrack(() => conversations.select(open.key));
  });

  // The newest message of the open thread stays in view.
  $effect(() => {
    void selected?.messages.length;
    void selected?.key;
    if (bubbles) bubbles.scrollTop = bubbles.scrollHeight;
  });

  /** Up and down move through the conversations, like a list of options. */
  function navigate(event: KeyboardEvent): void {
    const keys = conversations.list.map((c) => c.key);
    const at = keys.indexOf(selected?.key ?? "");
    const next: Record<string, number> = {
      ArrowDown: Math.min(keys.length - 1, at + 1),
      ArrowUp: Math.max(0, at - 1),
      Home: 0,
      End: keys.length - 1,
    };
    if (!(event.key in next) || keys.length === 0) return;
    event.preventDefault();
    conversations.select(keys[next[event.key]]);
    queueMicrotask(() => list?.querySelector<HTMLElement>('[aria-selected="true"]')?.focus());
  }

  function allowedIn(conversation: Conversation): boolean {
    return value.allowed.includes(conversation.app);
  }
</script>

<section class="messages" aria-label={i18n.t.panel.tabs.messages}>
  {#if !value.enabled}
    <div class="notice" in:arrive>
      <p><b>{t.off.title}</b> {t.off.text}</p>
      <Button variant="primary" onclick={() => patch({ enabled: true })}>{t.enable}</Button>
    </div>
  {:else if access === "notAsked"}
    <div class="notice" in:arrive>
      <p><b>{t.notAsked.title}</b> {t.notAsked.text}</p>
      <Button variant="primary" onclick={() => notifications.requestAccess()}>{t.allow}</Button>
    </div>
  {:else if access === "denied"}
    <div class="notice" in:arrive>
      <p><b>{t.denied.title}</b> {t.denied.text}</p>
      <Button onclick={() => notifications.openWindowsSettings()}>{t.openSettings}</Button>
    </div>
  {/if}

  {#if value.rules.length > 0}
    <ul class="apps" id="message-apps" aria-label={t.apps} in:arrive={{ index: 1 }}>
      {#each shownRules as rule (rule.id)}
        {@const on = value.allowed.includes(rule.id)}
        <li>
          <label class="app" class:on>
            <i class="dot" style:background={appColor(rule.id)}></i>
            <span aria-hidden="true">{rule.label}</span>
            <span class="visually-hidden">{t.showApp(rule.label)}</span>
            <Switch checked={on} onchange={(checked) => toggleApp(rule.id, checked)} />
          </label>
        </li>
      {/each}
      {#if hiddenRules > 0 || allApps}
        <li>
          <button type="button" class="more" aria-expanded={allApps} aria-controls="message-apps" onclick={() => (allApps = !allApps)}>
            {allApps ? t.fewerApps : t.moreApps(hiddenRules)}
          </button>
        </li>
      {/if}
    </ul>
  {/if}

  <div class="panes" in:arrive={{ index: 2 }}>
    <div class="convs">
      {#if conversations.list.length > 0}
        <!-- svelte-ignore a11y_interactive_supports_focus -->
        <div role="listbox" aria-label={t.conversations} bind:this={list} onkeydown={navigate}>
          {#each conversations.list as conversation (conversation.key)}
            {@const unread = unreadOf(conversation)}
            {@const on = conversation.key === selected?.key}
            <button
              type="button"
              class="conv"
              role="option"
              aria-selected={on}
              tabindex={on ? 0 : -1}
              onclick={() => conversations.select(conversation.key)}
            >
              <Avatar sender={conversation.sender} app={conversation.app} />
              <span class="who">
                <span class="name-line">
                  <b>{conversation.sender}</b>
                  <time datetime={new Date(conversation.updatedAt).toISOString()}>{time(conversation.updatedAt)}</time>
                </span>
                <span class="last">
                  <span class="visually-hidden">{conversation.appLabel}, </span>{lastLine(conversation, hidden)}
                </span>
              </span>
              {#if unread > 0}
                <span class="unread">{unread}<span class="visually-hidden">{t.unread(unread)}</span></span>
              {/if}
            </button>
          {/each}
        </div>
      {:else}
        <p class="empty">{t.noConversation}</p>
      {/if}
    </div>

    <div class="thread">
      {#if selected}
        {#key selected.key}
          <header class="head" in:arrive>
            <Avatar sender={selected.sender} app={selected.app} />
            <div class="who">
              <b>{selected.sender}</b>
              <small>{selected.appLabel}{allowedIn(selected) ? "" : t.hiddenInIsland}</small>
            </div>
            <Button variant="quiet" onclick={() => notifications.open(selected.notificationId)}>{t.open(selected.appLabel)}</Button>
          </header>
        {/key}
        <ol class="bubbles" role="log" aria-label={t.thread(selected.sender)} bind:this={bubbles}>
          {#each selected.messages as message (message.id)}
            {@const veiled = !message.mine && (hidden || message.masked)}
            {@const said = message.mine || veiled || !message.text ? messageText(message, hidden) : translations.shown(message.id, message.text)}
            <li class="bubble translate-host" class:mine={message.mine} class:veiled in:arrive>
              {#if message.mine}<span class="visually-hidden">{t.you}</span>{/if}{#key said}<span in:swap>{said}</span>{/key}
              <span class="meta">
                <time datetime={new Date(message.at).toISOString()}>{time(message.at)}{message.mine ? t.copied : ""}</time>
                {#if !message.mine}<TranslateButton id={message.id} text={message.text} hidden={veiled} reveal />{/if}
              </span>
            </li>
          {/each}
        </ol>
        <div class="compose">
          {#key selected.key}
            <ReplyBox target={replyTarget(selected)} autofocus={false} />
          {/key}
        </div>
      {:else}
        <div class="nothing">
          <p><b>{t.emptyTitle}</b></p>
          <p>{t.emptyText}</p>
        </div>
      {/if}
    </div>
  </div>

  {#if value.allowed.includes("whatsapp")}
    <label class="option" in:arrive={{ index: 3 }}>
      <span class="text">
        <b>{t.whatsappTabs}</b>
        <small>{t.whatsappTabsHint}</small>
      </span>
      <Switch checked={value.whatsappTabs} onchange={(whatsappTabs) => patch({ whatsappTabs })} />
    </label>
  {/if}

  <label class="option" in:arrive={{ index: 3 }}>
    <span class="text">
      <b>{t.hideContent}</b>
      <small>{t.hideContentHint}</small>
    </span>
    <Switch checked={hidden} onchange={(hideContent) => patch({ hideContent })} />
  </label>
</section>

<style>
  .messages {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-width: 0;
  }

  .notice {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    background: var(--surface);
    font-size: var(--fs-3);
    color: var(--ink-2);
  }

  .notice p {
    flex: 1;
    min-width: 0;
  }

  .notice b {
    color: var(--ink);
    font-weight: 600;
  }

  .apps {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
  }

  .app {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 34px;
    padding: 0 var(--s2) 0 var(--s3);
    border-radius: var(--r-pill);
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--ink-2);
    font-size: var(--fs-2);
    font-weight: 500;
    cursor: pointer;
    transition:
      border-color var(--t-quick),
      color var(--t-quick);
  }

  .app:hover {
    border-color: var(--line-2);
    color: var(--ink);
  }

  .app.on {
    color: var(--ink);
  }

  .app:has(:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .app :global(input:focus-visible) {
    outline: none;
  }

  .more {
    height: 34px;
    padding: 0 var(--s3);
    border-radius: var(--r-pill);
    border: 1px dashed var(--line-2);
    color: var(--ink-2);
    font-size: var(--fs-2);
    font-weight: 500;
    transition:
      border-color var(--t-quick),
      color var(--t-quick);
  }

  .more:hover {
    border-color: var(--ink-3);
    color: var(--ink);
  }

  .dot {
    width: var(--s2);
    height: var(--s2);
    border-radius: 50%;
    flex: none;
    opacity: 0.45;
    transition: opacity var(--t-quick);
  }

  .on .dot {
    opacity: 1;
  }

  .panes {
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr);
    height: 380px;
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    overflow: hidden;
  }

  .convs {
    min-height: 0;
    overflow-y: auto;
    border-right: 1px solid var(--line);
  }

  .conv {
    --avatar-ring: var(--panel);
    display: flex;
    align-items: center;
    gap: var(--s3);
    width: 100%;
    padding: var(--s3) var(--s3) var(--s3) var(--s4);
    text-align: left;
    transition: background var(--t-quick);
  }

  .conv + .conv {
    border-top: 1px solid var(--line);
  }

  .conv:hover {
    --avatar-ring: var(--surface);
    background: var(--surface);
  }

  .conv[aria-selected="true"] {
    --avatar-ring: var(--surface-2);
    background: var(--surface-2);
  }

  .conv:focus-visible {
    outline-offset: -2px;
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.35;
  }

  .name-line {
    display: flex;
    align-items: baseline;
    gap: var(--s2);
  }

  .name-line b {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .name-line time {
    flex: none;
    color: var(--ink-3);
    font-size: var(--fs-1);
  }

  .last {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .unread {
    flex: none;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--fs-1);
    font-weight: 600;
  }

  .empty {
    padding: var(--s4);
    color: var(--ink-3);
    font-size: var(--fs-3);
  }

  .thread {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .head {
    --avatar-ring: var(--panel);
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s3) var(--s3) var(--s3) var(--s4);
    border-bottom: 1px solid var(--line);
  }

  .head b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  .head small {
    display: block;
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .head .who b,
  .head .who small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bubbles {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s4);
    /* Older bubbles fade under the header instead of being cut by it. */
    mask-image: linear-gradient(transparent, var(--opaque) var(--s4));
  }

  /* Theirs on the left, on a raised surface; yours on the right, in the accent. The corner
     nearest the speaker is tighter. */
  .bubble {
    max-width: 76%;
    align-self: flex-start;
    padding: var(--s2) var(--s3);
    border-radius: var(--r-card);
    border-bottom-left-radius: var(--r-chip);
    background: var(--surface-2);
    font-size: var(--fs-3);
    line-height: 1.45;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .bubble.mine {
    align-self: flex-end;
    border-bottom-left-radius: var(--r-card);
    border-bottom-right-radius: var(--r-chip);
    background: var(--accent);
    color: var(--accent-ink);
  }

  .bubble.veiled {
    color: var(--ink-2);
    font-style: italic;
  }

  .bubble .meta {
    display: flex;
    align-items: center;
    gap: var(--s1);
    margin-top: 2px;
  }

  .bubble time {
    display: block;
    font-family: var(--font-mono);
    font-size: var(--fs-1);
    opacity: 0.7;
  }

  .compose {
    padding: var(--s3);
    border-top: 1px solid var(--line);
  }

  .nothing {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--s1);
    padding: var(--s6);
    color: var(--ink-2);
    font-size: var(--fs-3);
    max-width: 44ch;
    margin: 0 auto;
    text-align: center;
  }

  .nothing b {
    color: var(--ink);
    font-size: var(--fs-body);
    font-weight: 600;
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    cursor: pointer;
  }

  .option .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .option b {
    font-size: var(--fs-3);
    font-weight: 500;
  }

  .option small {
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .conv,
    .app,
    .dot {
      transition: none;
    }
  }
</style>
