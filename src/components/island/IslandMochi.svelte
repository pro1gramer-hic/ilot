<!-- Mochi in the island (docs/09-DEMO-FINALE.md, section 6): one Mochi that glides to the
     slot each opening keeps for him. He breathes and follows the cursor with his eyes,
     opens big eyes while the island is hovered, dances to the music, shakes on a
     permission. A click squashes him, three make him dizzy; pulled down out of the pill,
     the island stretches, then he leaves for his own window (« Se balade », « Te suit »,
     « Sur le bureau »). A right click opens his wardrobe. At launch he waves (unless the
     hub's intro did), and at rest he has a small emotion now and then (prompt 12). -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { type BuddyMode, buddyHome, buddyShow, inTauri } from "../../lib/ipc";
  import { island } from "../../lib/island/controller.svelte";
  import { PULL_OUT_PX, mochiPlace, stretchOf } from "../../lib/island/model";
  import { canEmote, emotionMs, nextEmotionDelay, pickEmotion } from "../../lib/mochi/idle";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import { type OwnMove, OWN_MOVE_S } from "../../lib/mochi/moves";
  import { reduced } from "../../lib/motion.svelte";
  import type { MochiMove } from "../../lib/mochi/poses";
  import { Spring } from "../../lib/spring.svelte";
  import { bubble } from "../../lib/stores/bubble.svelte";
  import { chat } from "../../lib/stores/chat.svelte";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { media } from "../../lib/stores/media.svelte";
  import { mood } from "../../lib/stores/mood.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { sound } from "../../lib/stores/sound.svelte";

  /** The island's size now: Mochi's slot in a right-hand corner follows it. */
  let { width, height }: { width: number; height: number } = $props();

  /** CSS pixels per unit of his 24-unit drawing: 48 px. */
  const DRAWN = 2;
  /** Three clicks within this time make him dizzy. */
  const CLICKS_MS = 650;
  const SQUISH_MS = 450;
  const DIZZY_MS = 2600;
  /** With nothing to watch for this long, his frame freezes (no CPU at rest). */
  const IDLE_MS = 60_000;

  const place = $derived(mochiPlace(island.mode, { width, height, radius: 0 }));
  const target = $derived(mochiPlace(island.mode, island.size));
  const X = new Spring(2);
  const Y = new Spring(-6);
  $effect(() => {
    if (!target) return;
    X.set(target.x);
    Y.set(target.y);
  });
  /** Openings whose slot sits on the right follow the island's growing width. */
  const x = $derived(island.mode === "message" && place ? place.x : X.value);

  let clicks: number[] = [];
  let squishedUntil = $state(0);
  let dizzyUntil = $state(0);
  let now = $state(performance.now());

  function poke(): void {
    const at = performance.now();
    clicks = [...clicks.filter((t) => at - t < CLICKS_MS), at];
    if (clicks.length >= 3) {
      clicks = [];
      dizzyUntil = at + DIZZY_MS;
      island.kick(-120);
      bubble.say(i18n.t.island.mochi.dizzy(settings.value.mochi.name));
      sound.play("dizzy");
    } else {
      squishedUntil = at + SQUISH_MS;
      mood.flash("annoyed", 800);
      island.kick(-70);
      sound.play("click");
    }
    now = at;
    setTimeout(() => (now = performance.now()), Math.max(SQUISH_MS, dizzyUntil - at) + 10);
  }

  // Pulling him out: the island stretches with resistance; past 72 px, he is out.
  let pull: { y: number } | null = null;

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0 || island.mode !== "pill" || island.mochiOut) return;
    event.stopPropagation();
    poke();
    pull = { y: event.clientY };
  }

  function onwindowmove(event: PointerEvent): void {
    if (!pull) return;
    const dy = Math.max(0, (event.clientY - pull.y) * (island.placement.opensDown ? 1 : -1));
    island.pull = dy;
    const mode = settings.value.modes.mochi;
    if (dy <= PULL_OUT_PX || mode === "reste" || !inTauri) return;
    pull = null;
    island.pull = 0;
    island.kick(-160);
    island.mochiOut = true;
    // He leaves from his own place: the core knows where the island is on the screen.
    const out: BuddyMode = mode === "suit" || mode === "bureau" ? mode : "balade";
    island.mochiDesk = out === "bureau";
    void buddyHome().then((home) => buddyShow(home.x, home.y, out));
    bubble.say(i18n.t.island.mochi.out(settings.value.mochi.name));
    sound.play("leave");
  }

  /** A right click on him: his wardrobe (anywhere else on the island, the quick menu). */
  function oncontextmenu(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    island.openWardrobe();
  }

  function onwindowup(): void {
    pull = null;
    island.pull = 0;
  }

  const playing = $derived(media.shown?.playing === true);
  const working = $derived(sessions.liveCount > 0);
  const dizzy = $derived(now < dizzyUntil);
  const squished = $derived(now < squishedUntil);

  /** The greeting or an emotion playing now (îlot's own moves, once each). */
  let own = $state<OwnMove | null>(null);
  let ownTimer: ReturnType<typeof setTimeout> | undefined;

  function playOwn(move: OwnMove, ms: number): void {
    clearTimeout(ownTimer);
    own = move;
    ownTimer = setTimeout(() => (own = null), ms);
  }

  const base: MochiMove = $derived.by(() => {
    if (island.mode === "permission") return "shake";
    if (dizzy) return "dizzy";
    if (squished) return "squish";
    // A file dropped: he turns into a box that swallows it (prompt 12, lot 3).
    if (island.mode === "file") return "box";
    if (mood.expressive) return mood.expressive;
    if (island.mode === "drop") return "swallow";
    // A model answers in the chat: he thinks.
    if (chat.thinking) return "think";
    if (decisions.suspended) return "sleep";
    if (playing) return "rock";
    return working ? "breathe" : "watch";
  });
  // The greeting and the emotions only play over his calm moves.
  const move: MochiMove = $derived(own && (base === "watch" || base === "breathe") ? own : base);
  const face = $derived(mood.face ?? (island.mode === "permission" ? "worry" : island.mode === "drop" ? "eat" : undefined));
  const follows = $derived(move === "breathe" || move === "rock" || move === "watch");

  // Nothing moves around him for a minute: he stops redrawing until something happens.
  let frozen = $state(false);
  $effect(() => {
    void base;
    void island.mode;
    void island.pointer.near;
    frozen = false;
    if (working || playing || chat.thinking || island.pointer.near) return;
    const timer = setTimeout(() => (frozen = true), IDLE_MS);
    return () => clearTimeout(timer);
  });

  // The launch greeting, here when the hub's intro did not play it (a permission waited).
  $effect(() => {
    if (!island.greeting || island.mode !== "pill" || island.resting) return;
    island.greeting = false;
    playOwn("hello", OWN_MOVE_S.hello * 1000);
    sound.play("hello");
  });

  // Small emotions at rest: one every 2 to 5 minutes, drawn at random, only when all is
  // calm. Frozen, he wakes up just for it (`animated` below), then freezes again.
  const animations = $derived(settings.value.mochi.animated && !reduced());
  const calm = $derived(
    canEmote({
      folded: island.mode === "pill" && !island.hubOpen && !island.panelOpen,
      working,
      playing,
      busy: island.pointer.near || island.hovered || island.dragging || island.pull > 0 || decisions.suspended,
      away: island.mochiOut || island.resting || !place,
      animated: animations,
    }),
  );
  let lastEmotion: OwnMove | undefined;
  /** Counts the emotions played: each one starts the wait for the next. */
  let emoted = $state(0);
  $effect(() => {
    void emoted;
    if (!calm) return;
    const timer = setTimeout(() => {
      const emotion = pickEmotion(Math.random, lastEmotion);
      lastEmotion = emotion;
      playOwn(emotion, emotionMs(emotion));
      if (emotion === "yawn") sound.play("yawn");
      emoted += 1;
    }, nextEmotionDelay(Math.random));
    return () => clearTimeout(timer);
  });
  $effect(() => () => clearTimeout(ownTimer));

  const stretch = $derived(stretchOf(island.pull) * 0.65 * (island.placement.opensDown ? 1 : -1));
</script>

<svelte:window onpointermove={onwindowmove} onpointerup={onwindowup} />

<span
  class="mochi"
  class:away={!place || island.mochiOut}
  class:grab={island.mode === "pill"}
  data-mochi
  style:transform="translate({x}px, {Y.value + stretch}px)"
  title={island.mode === "pill" ? i18n.t.island.mochi.tip(settings.value.mochi.name) : undefined}
  {onpointerdown}
  {oncontextmenu}
  role="presentation"
>
  <Mochi
    {move}
    face={own ? undefined : face}
    big={island.hovered && !mood.face && !own}
    look={follows && !own ? { x: island.pointer.x } : undefined}
    animated={(!frozen || own !== null) && !island.resting && settings.value.mochi.animated}
    scale={DRAWN}
    label={settings.value.mochi.name}
    dressed
  />
</span>

<style>
  .mochi {
    position: absolute;
    left: 0;
    top: 0;
    width: 48px;
    height: 48px;
    transition: opacity var(--t-out);
    will-change: transform;
  }

  .mochi.grab {
    cursor: grab;
  }

  /* Out of the island, or stepping aside for the music: his slot stays empty. */
  .mochi.away {
    opacity: 0;
    pointer-events: none;
  }
</style>
