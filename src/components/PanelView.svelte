<!-- The large panel (docs/09-DEMO-FINALE.md, section 8). It fills the box the island gives it
     (up to 920 × 620): a rail on the left (Mochi and his mood, the tabs, a small
     player), and on the right a header (the tab's title, one sentence on what goes on,
     « Tout suspendre », close) above the tab's content, which arrives out of a light blur
     and leaves faster than it came. Escape closes it (startup.ts). -->
<script lang="ts">
  import { sessionTool } from "../lib/agents";
  import { i18n } from "../lib/i18n/i18n.svelte";
  import { island } from "../lib/island/controller.svelte";
  import type { Tab } from "../lib/island/model";
  import { sourceLabel } from "../lib/media";
  import { decisions } from "../lib/stores/decisions.svelte";
  import { media } from "../lib/stores/media.svelte";
  import { notifications } from "../lib/stores/notifications.svelte";
  import { pastilles } from "../lib/stores/pastilles.svelte";
  import { permissions } from "../lib/stores/permissions.svelte";
  import { sessions } from "../lib/stores/sessions.svelte";
  import { settings } from "../lib/stores/settings.svelte";
  import { voice } from "../lib/stores/voice.svelte";
  import { connectors } from "./panel/connectors.svelte";
  import { inbox, unreadCount } from "./panel/inbox";
  import { arrive, leave } from "./panel/motion";
  import PanelIcon from "./panel/PanelIcon.svelte";
  import Rail from "./panel/Rail.svelte";
  import Button from "./settings/Button.svelte";
  import AgentsTab from "./tabs/AgentsTab.svelte";
  import ChatTab from "./tabs/ChatTab.svelte";
  import MessagesTab from "./tabs/MessagesTab.svelte";
  import MochiTab from "./tabs/MochiTab.svelte";
  import ModesTab from "./tabs/ModesTab.svelte";
  import GeneralTab from "./tabs/GeneralTab.svelte";
  import MusicTab from "./tabs/MusicTab.svelte";
  import SoundTab from "./tabs/SoundTab.svelte";
  import ShortcutsTab from "./tabs/ShortcutsTab.svelte";
  import PastillesTab from "./tabs/PastillesTab.svelte";
  import ThemesTab from "./tabs/ThemesTab.svelte";
  import TodayTab from "./tabs/TodayTab.svelte";

  const t = $derived(i18n.t.panel);

  function greeting(): string {
    const hour = new Date().getHours();
    return hour < 6 ? t.greeting.night : hour < 18 ? t.greeting.day : t.greeting.evening;
  }

  const working = $derived(sessions.list.filter((s) => s.status === "active").length);
  const asking = $derived(permissions.current);

  /** The tab's title and one sentence on what goes on. */
  const header: Record<Tab, () => [string, string]> = {
    today: () => [
      greeting(),
      decisions.suspended
        ? t.sentence.suspended
        : asking
          ? t.sentence.asking(sessionTool(asking))
          : working > 0
            ? t.sentence.working(working)
            : t.sentence.idle,
    ],
    agents: () => [t.tabs.agents, t.sentence.agents(working, connectors.connected)],
    messages: () => {
      const unread = unreadCount(inbox(notifications.queue, notifications.postponed));
      return [t.tabs.messages, unread > 0 ? t.sentence.unread(unread) : t.sentence.noUnread];
    },
    music: () => {
      const track = media.shown;
      if (!track) return [t.tabs.music, t.sentence.nothingPlaying];
      return [t.tabs.music, t.sentence.playing(track.playing, sourceLabel(track))];
    },
    chat: () => [t.tabs.chat, t.sentence.chat],
    pastilles: () => [t.tabs.pastilles, t.sentence.pastilles(pastilles.ids.length)],
    mochi: () => [settings.value.mochi.name, t.sentence.mochi],
    themes: () => [t.tabs.themes, t.sentence.themes],
    modes: () => [t.tabs.modes, t.sentence.modes],
    sound: () => [t.tabs.sound, settings.value.sound.muted ? t.sentence.muted : t.sentence.sound],
    general: () => [t.tabs.general, t.sentence.general],
    shortcuts: () => [t.tabs.shortcuts, voice.listening ? t.sentence.listening : t.sentence.shortcuts],
  };

  const [title, sentence] = $derived(header[island.tab]());
</script>

<div class="panel" role="dialog" aria-modal="false" aria-labelledby="panel-title" aria-describedby="panel-sentence">
  <Rail />
  <div class="main">
    <header>
      <div class="titles">
        <h2 id="panel-title">{title}</h2>
        <p id="panel-sentence">{sentence}</p>
      </div>
      <div class="actions">
        <Button variant={decisions.suspended ? "primary" : "plain"} onclick={() => void decisions.toggleSuspended()}>
          {decisions.suspended ? t.resumeAll : t.suspendAll}
        </Button>
        <Button variant="quiet" square aria-label={t.close} title={t.closeTip} onclick={() => island.closePanel()}>
          <PanelIcon name="close" />
        </Button>
      </div>
    </header>
    {#if decisions.error}
      <p class="error" role="alert">{decisions.error}</p>
    {/if}
    <div class="body">
      {#key island.tab}
        <div class="pane" role="tabpanel" id="pane-{island.tab}" aria-labelledby="tab-{island.tab}" tabindex="0" in:arrive out:leave>
          {#if island.tab === "today"}
            <TodayTab />
          {:else if island.tab === "agents"}
            <AgentsTab />
          {:else if island.tab === "messages"}
            <MessagesTab />
          {:else if island.tab === "music"}
            <MusicTab ondetach={() => island.closePanel()} />
          {:else if island.tab === "chat"}
            <ChatTab />
          {:else if island.tab === "pastilles"}
            <PastillesTab />
          {:else if island.tab === "mochi"}
            <MochiTab />
          {:else if island.tab === "themes"}
            <ThemesTab />
          {:else if island.tab === "sound"}
            <SoundTab />
          {:else if island.tab === "general"}
            <GeneralTab />
          {:else if island.tab === "shortcuts"}
            <ShortcutsTab />
          {:else}
            <ModesTab />
          {/if}
        </div>
      {/key}
    </div>
  </div>
</div>

<style>
  .panel {
    /* The rail's width (docs/09-DEMO-FINALE.md, section 8). */
    --rail: calc(6 * var(--s8) + var(--s6));
    container: panel / inline-size;
    display: grid;
    grid-template-columns: var(--rail) minmax(0, 1fr);
    width: 100%;
    height: 100%;
    overflow: hidden;
    border-radius: var(--r-panel);
    background: var(--panel);
    border: 1px solid var(--line);
    box-shadow: var(--shadow), var(--hi);
    color: var(--ink);
  }

  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s5) var(--s6) 0;
  }

  .titles {
    min-width: 0;
  }

  h2 {
    font-size: var(--fs-title);
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.3;
  }

  .titles p {
    margin-top: calc(var(--s1) / 2);
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .error {
    padding: var(--s2) var(--s6) 0;
    color: var(--err);
    font-size: var(--fs-3);
  }

  /* Leaving and arriving content share one cell, so nothing jumps. */
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
  }

  .pane {
    grid-area: 1 / 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--s4) var(--s6) var(--s6);
    transform-origin: 50% 0;
    /* What scrolls away fades out instead of being cut. */
    mask-image: linear-gradient(to bottom, transparent, var(--opaque) var(--s3), var(--opaque) calc(100% - var(--s5)), transparent);
  }

  .pane:focus-visible {
    outline-offset: calc(-1 * var(--s1));
    border-radius: var(--r-card);
  }
</style>
