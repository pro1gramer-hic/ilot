<!-- The hub (docs/09-DEMO-FINALE.md, section 8 bis): what a click on the pill brings out.
     A top bar, then its home (the followed session on the left, every session as an
     icon on the right), its messages, its pastilles (prompt 12, lot 4), the chat or an
     e-mail draft (prompt 12, lot 3), a session followed live, or Mochi out of breath;
     the intro plays over the home at launch and on demand. Everything comes from the
     stores: agents, their journal, the music, the conversations. Escape (startup.ts), a
     click elsewhere or the pill folds it; the gear opens the large panel (« Réglages »). -->
<script lang="ts">
  import { untrack } from "svelte";
  import { LAUNCH_LINGER_MS, hubSessions, pickFollowed, visibleTiles } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { stillness, wilt } from "../../lib/motion.svelte";
  import { media } from "../../lib/stores/media.svelte";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import ChatView from "./ChatView.svelte";
  import Dizzy from "./Dizzy.svelte";
  import FocusCard from "./FocusCard.svelte";
  import HubBar from "./HubBar.svelte";
  import HubMessages from "./HubMessages.svelte";
  import HubPastilles from "./HubPastilles.svelte";
  import Intro from "./Intro.svelte";
  import LiveSession from "./LiveSession.svelte";
  import MailDraft from "./MailDraft.svelte";
  import { swap } from "./motion";
  import { play } from "./play.svelte";
  import SessionTiles from "./SessionTiles.svelte";

  let {
    onburst,
    onopenchat,
  }: {
    onburst: (x: number, y: number) => void;
    /** Opens îlot's chat on a local model from the pastilles (wired with the chat, lot 3). */
    onopenchat?: (model: string, provider: "ollama" | "lmstudio") => void;
  } = $props();

  const list = $derived(hubSessions(sessions.list, media.shown));
  const followed = $derived(pickFollowed(list, play.chosen));
  const tiles = $derived(visibleTiles(list, followed?.key));
  const view = $derived(island.hubView);
  const live = $derived(followed?.kind === "agent" ? followed : undefined);

  // A session asked for from outside (the counter of the activity line) goes on the
  // left card.
  $effect(() => {
    const asked = island.hubFollow;
    if (asked === null) return;
    untrack(() => {
      play.chosen = asked;
      island.hubFollow = null;
    });
  });

  // Only an agent can be followed live: anything else goes back home; so does the
  // pastilles view once the last pastille is gone.
  $effect(() => {
    if (view === "live" && !live && island.hubFollow === null) island.showHub("home");
    if (view === "pastilles" && pastilles.ids.length === 0) island.showHub("home");
  });

  // Running times move on once a second while the hub can be seen.
  $effect(() => {
    if (stillness.current) return;
    timeline.tick();
    const timer = setInterval(() => timeline.tick(), 1000);
    return () => clearInterval(timer);
  });

  // After the launch intro, the hub folds back on its own, unless the user touched it.
  let used = false;
  let linger: ReturnType<typeof setTimeout> | undefined;
  $effect(() => () => clearTimeout(linger));

  function introDone(): void {
    const launch = island.hubIntro === "launch";
    island.hubIntro = null;
    if (!launch) return;
    linger = setTimeout(() => {
      if (!used && island.hubOpen && island.hubView === "home") island.closeHub();
    }, LAUNCH_LINGER_MS);
  }

  function choose(key: string): void {
    used = true;
    play.chosen = key;
  }

  /** A press anywhere else folds the hub (the pill toggles it by itself). */
  function onwindowpointerdown(event: PointerEvent): void {
    if (event.target instanceof Element && event.target.closest(".hub-card, .island, .bubble")) return;
    island.closeHub();
  }

  // The keyboard starts on the bar's current view, unless îlot just started.
  let bar = $state<HubBar>();
  $effect(() => {
    if (bar && untrack(() => island.hubIntro) !== "launch") bar.focus();
  });
</script>

<svelte:window onpointerdown={onwindowpointerdown} />

<!-- Any touch counts as use: the launch intro then leaves the hub open. -->
<div
  class="hub"
  role="dialog"
  aria-modal="false"
  aria-label={i18n.t.hub.label}
  tabindex="-1"
  onpointerdown={() => (used = true)}
  onpointerenter={() => (used = true)}
  onkeydown={() => (used = true)}
>
  {#if view === "dizzy"}
    <div class="fill" in:swap>
      <Dizzy palette={followed?.palette ?? "lavande"} />
    </div>
  {:else if view === "live" && live}
    <div class="fill" in:swap>
      <LiveSession followed={live} {onburst} />
    </div>
  {:else if view === "mail"}
    <div class="fill" in:swap>
      <MailDraft />
    </div>
  {:else}
    <HubBar bind:this={bar} />
    <div class="body" id="hub-view" role="tabpanel" aria-labelledby="hub-tab-{view}">
      {#if view === "messages"}
        <div class="fill" in:swap>
          <HubMessages />
        </div>
      {:else if view === "chat"}
        <div class="fill" in:swap>
          <ChatView />
        </div>
      {:else if view === "pastilles"}
        <div class="fill" in:swap>
          <HubPastilles {onopenchat} />
        </div>
      {:else}
        <div class="home" class:veiled={island.hubIntro !== null} in:swap>
          <FocusCard {followed} />
          <SessionTiles {tiles} followed={followed?.key} onchoose={choose} />
        </div>
      {/if}
      {#if island.hubIntro}
        <div class="intro" out:wilt={{ duration: 320 }}>
          <Intro ondone={introDone} greet={island.hubIntro === "launch"} />
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .hub {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--s2) var(--s3) var(--s3);
    color: var(--ink);
    outline: none;
    /* Badges and dots are cut out of the cards they sit on. */
    --badge-ring: var(--surface);
  }

  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    margin-top: calc(var(--s1) + var(--gap-mark));
    display: grid;
  }

  .fill {
    height: 100%;
    min-height: 0;
  }

  .home {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: var(--s3);
    min-height: 0;
  }

  /* The home waits under the intro, already laid out. */
  .home.veiled {
    visibility: hidden;
  }

  .intro {
    position: absolute;
    inset: 0;
  }
</style>
