<!-- The folded island (docs/09-DEMO-FINALE.md, section 3): Mochi's slot on the left, the
     agents in the middle (a count, nothing, or what each one does, by mode; at rest, the
     chosen pastille instead of « En veille »), the unread badge of the « Pastille » mode,
     and one dot per AI on the right. The wheel sets the volume. -->
<script lang="ts">
  import { editCounter, fitDots, isLive, pillDots, sessionColor, sessionName, subagentsLabel, tickerText } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { changeVolume } from "../../lib/ipc";
  import { island } from "../../lib/island/controller.svelte";
  import { rise, vanish } from "../../lib/motion.svelte";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Icon from "../Icon.svelte";
  import PillPastille from "./PillPastille.svelte";

  /** The volume gauge stays this long after the last notch. */
  const GAUGE_MS = 1200;
  /** The activity line moves to the next agent this often. */
  const TICK_MS = 2400;

  const t = $derived(i18n.t.island.pill);
  const count = $derived(sessions.liveCount);
  const shows = $derived(settings.value.modes.agents);
  /** Dots that fit beside each mode's text; more agents than that are counted (« +2 »). */
  const MAX_DOTS: Record<typeof shows, number> = { compteur: 4, activite: 4, points: 5 };

  /** The three AIs always have their dot; any other agent or badge only while it is there. */
  const dots = $derived(pillDots(sessions.list));
  const helping = $derived(dots.reduce((sum, dot) => sum + dot.subagents, 0));
  const fitting = $derived(
    fitDots(
      dots.map((dot) => ({ ...dot, state: island.flashes[dot.key] ?? dot.state })),
      MAX_DOTS[shows],
    ),
  );

  let volume = $state<number | null>(null);
  let gaugeTimer: ReturnType<typeof setTimeout> | undefined;

  async function onwheel(event: WheelEvent): Promise<void> {
    event.preventDefault();
    if (event.deltaY === 0) return;
    const level = await changeVolume(event.deltaY < 0 ? 1 : -1);
    volume = Math.round(level * 100);
    clearTimeout(gaugeTimer);
    gaugeTimer = setTimeout(() => (volume = null), GAUGE_MS);
  }

  // « Activité »: one agent at a time, in turn.
  let turn = $state(0);
  $effect(() => {
    if (shows !== "activite") return;
    const timer = setInterval(() => (turn += 1), TICK_MS);
    return () => clearInterval(timer);
  });
  const live = $derived(sessions.list.filter(isLive));
  const ticking = $derived(live.length > 0 ? live[turn % live.length] : undefined);
  /** The file it edits and its lines, a button to its change in the hub. */
  const counter = $derived(ticking ? editCounter(ticking) : null);

  const summary = $derived(
    (decisions.suspended ? t.suspended : count > 0 ? t.working(count) : t.none) +
      (helping > 0 ? `, ${subagentsLabel(helping)}` : ""),
  );
</script>

<div class="pill" data-shows={shows} {onwheel} role="presentation">
  <span class="slot" aria-hidden="true"></span>
  {#if volume !== null}
    <span class="gauge" in:rise={{ shift: 3 }} out:vanish aria-live="polite">
      <Icon name="volume" size={12} />
      <span class="bar" aria-hidden="true"><i style:transform="scaleX({volume / 100})"></i></span>
      <span class="level">{volume}</span>
    </span>
  {:else if shows === "compteur"}
    <span class="count" aria-label={summary}>
      {#if decisions.suspended}
        <b>{t.paused}</b>
      {:else if count > 0}
        <b>{count}</b> {t.agents(count)}
      {:else if pastilles.shown}
        <PillPastille id={pastilles.shown} />
      {:else}
        {t.idle}
      {/if}
    </span>
  {:else if shows === "activite"}
    <span class="ticker" aria-label={summary}>
      {#if ticking}
        {#key `${ticking.id}:${turn}`}
          <span class="step" in:rise={{ shift: 8 }}>
            <i class="dot" style:--c={sessionColor(ticking)}></i>
            <b>{sessionName(ticking)}</b>
            {#if counter}
              {ticking.activity?.verb}
              <button
                type="button"
                class="counter"
                aria-label={i18n.t.island.change.label(counter.file, counter.added, counter.removed)}
                title={i18n.t.island.change.title}
                onclick={() => island.openChange(ticking.id)}
              >
                <span class="file">{counter.file}</span>
                <span class="add">+{counter.added}</span>
                <span class="del">−{counter.removed}</span>
              </button>
            {:else}
              {tickerText(ticking)}
            {/if}
          </span>
        {/key}
      {:else if !decisions.suspended && pastilles.shown}
        <span class="step"><PillPastille id={pastilles.shown} /></span>
      {:else}
        <span class="step">{decisions.suspended ? t.paused : t.idle}</span>
      {/if}
    </span>
  {:else}
    <span class="visually-hidden">{summary}</span>
  {/if}
  {#if island.unread > 0}
    <span class="badge" aria-label={t.unread(island.unread)}>
      {island.unread}
    </span>
  {/if}
  <span class="dots" aria-hidden="true">
    {#each fitting.shown as dot (dot.key)}
      <span class="ai" data-dot={dot.key} title={dot.name}>
        <i class={dot.state} style:--c={dot.color}></i>
        {#if dot.subagents > 0}<b data-subagents={dot.key}>{dot.subagents}</b>{/if}
      </span>
    {/each}
    {#if fitting.hidden > 0}<b class="more" data-more>+{fitting.hidden}</b>{/if}
  </span>
</div>

<style>
  .pill {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    padding: 0 var(--s4) 0 var(--s2);
  }

  /* Mochi floats over this slot (IslandMochi). */
  .slot {
    width: 36px;
    height: 36px;
    flex: none;
  }

  .count {
    color: var(--ink-2);
    font-size: var(--fs-3);
    white-space: nowrap;
  }

  .count b {
    color: var(--ink);
    font-weight: 600;
  }

  .ticker {
    position: relative;
    display: grid;
    flex: 1;
    min-width: 0;
    height: 40px;
    align-items: center;
    overflow: hidden;
  }

  .step {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--ink-2);
    font-size: var(--fs-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .step b {
    color: var(--ink);
    font-weight: 600;
  }

  .step .dot {
    width: var(--dot);
    height: var(--dot);
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }

  /* The file being edited and its lines: a quiet button to its change in the hub. */
  .counter {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    min-width: 0;
    margin: 0 -6px;
    padding: 2px 6px;
    border-radius: var(--r-chip);
    color: var(--ink);
    font-size: var(--fs-3);
    transition: background-color var(--t-quick);
  }

  .counter:hover {
    background: var(--hover);
  }

  .counter:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .counter .file {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .counter .add,
  .counter .del {
    flex: none;
    font: 600 var(--fs-2) var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .counter .add {
    color: var(--ok);
  }

  .counter .del {
    color: var(--err);
  }

  .badge {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--fs-1);
    font-weight: 600;
    animation: pop 0.4s var(--ease-out);
  }

  @keyframes pop {
    from {
      transform: scale(0.3);
    }
  }

  .dots {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ai {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  /* An AI's subagents at work, next to its dot; agents left out for room, at the end. */
  .ai b,
  .more {
    color: var(--ink-2);
    font-size: var(--fs-1);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .dots i {
    position: relative;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--c);
    opacity: 0.35;
    transition: opacity 0.3s;
  }

  .dots .work,
  .dots .perm,
  .dots .ok,
  .dots .err {
    opacity: 1;
  }

  /* An AI that starts working sends three rings out, then its dot stays lit: a ring
     playing for hours would have the whole window composited 60 times a second. */
  .dots .work::after {
    content: "";
    position: absolute;
    inset: -3px;
    border-radius: 50%;
    border: 1.5px solid var(--c);
    opacity: 0;
    animation: ping 1.6s var(--ease-out) 3;
  }

  @keyframes ping {
    from {
      transform: scale(0.6);
      opacity: 0.9;
    }
    to {
      transform: scale(1.9);
      opacity: 0;
    }
  }

  .dots .perm {
    background: var(--warn);
  }

  .dots .ok {
    background: var(--ok);
    box-shadow: var(--ok-ring);
  }

  .dots .err {
    background: var(--err);
    box-shadow: var(--err-ring);
  }

  .gauge {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--ink-2);
  }

  .bar {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--line-2);
    overflow: hidden;
  }

  .bar i {
    display: block;
    height: 100%;
    background: var(--ink);
    transform-origin: left;
    transition: transform var(--t-quick) var(--ease-out);
  }

  .level {
    min-width: 2ch;
    font-size: var(--fs-1);
    font-weight: 600;
    text-align: right;
  }

  @media (prefers-reduced-motion: reduce) {
    .dots .work::after,
    .badge {
      animation: none;
    }
  }
</style>
