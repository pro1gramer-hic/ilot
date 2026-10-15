<!-- The last hour of every AI (dataviz): one line per AI on one time axis, 8 px bars with
     round ends while it worked, marks for what ended (green with a tick), failed (red with
     a cross) or asked (amber diamond), and a « maintenant » line. Each bar and mark has a
     larger invisible target: hovering or focusing it shows its details; arrow keys move
     from one to the next. Colors follow the AI, never its rank; text stays in ink. -->
<script lang="ts">
  import { AI, AI_SOURCES, DECISION, MAIN_SOURCES } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import { HOUR_MS, type MarkKind, clockLabel, durationLabel, hasActivity, laneOf, position } from "../../lib/timeline";

  /** Room for the AI names, the « maintenant » dot, each line and the time axis. */
  const NAMES_W = 72;
  const END_PAD = 12;
  const TOP = 14;
  const LANE_H = 40;
  const AXIS_H = 26;
  const BAR_H = 8;
  const MARK_R = 6;
  /** Invisible targets are larger than the marks they carry. */
  const HIT = 24;
  const TICKS = [60, 45, 30, 15, 0];
  const MINUTE_MS = 60_000;

  const COLOR: Record<MarkKind, string> = { done: "var(--ok)", failed: "var(--err)", permission: "var(--warn)" };

  const t = $derived(i18n.t.today.timeline);

  let width = $state(0);

  const now = $derived(timeline.now);
  /** Claude, Gemini and Codex always; any other agent once it shows up. */
  const sources = $derived(
    AI_SOURCES.filter(
      (source) =>
        MAIN_SOURCES.includes(source) || hasActivity(laneOf(timeline.value, source)) || sessions.list.some((s) => s.source === source),
    ),
  );
  const plot = $derived(Math.max(0, width - NAMES_W - END_PAD));
  const height = $derived(TOP + sources.length * LANE_H + AXIS_H);
  const x = (at: number): number => NAMES_W + position(at, now) * plot;
  const laneY = (index: number): number => TOP + index * LANE_H + LANE_H / 2;

  type Item = {
    key: string;
    lane: number;
    at: number;
    /** Target box, from the box's top left corner. */
    left: number;
    top: number;
    w: number;
    h: number;
    title: string;
    detail: string;
  };

  const bars = $derived(
    sources.flatMap((source, lane) =>
      laneOf(timeline.value, source).segments
        .filter((s) => (s.end ?? now) >= now - HOUR_MS)
        .map((s) => {
          const x1 = Math.max(NAMES_W, x(s.start));
          const x2 = Math.max(x1 + BAR_H / 2, x(s.end ?? now));
          const minutes = durationLabel((s.end ?? now) - Math.max(s.start, now - HOUR_MS));
          return { source, lane, segment: s, x1, x2, minutes };
        }),
    ),
  );

  const marks = $derived(
    sources.flatMap((source, lane) =>
      laneOf(timeline.value, source).marks.filter((m) => m.at >= now - HOUR_MS).map((mark) => ({ source, lane, mark, cx: x(mark.at) })),
    ),
  );

  /** Every target in time order, for the arrow keys. */
  const items: Item[] = $derived(
    [
      ...bars.map(
        ({ source, lane, segment, x1, x2, minutes }): Item => ({
          key: `bar-${source}-${segment.start}`,
          lane,
          at: segment.start,
          left: x1,
          top: laneY(lane) - HIT / 2,
          w: Math.max(HIT, x2 - x1),
          h: HIT,
          title: AI[source].name,
          detail: t.bar(segment.label, minutes, segment.end === null),
        }),
      ),
      ...marks.map(
        ({ source, lane, mark, cx }): Item => ({
          key: `mark-${source}-${mark.kind}-${mark.at}`,
          lane,
          at: mark.at,
          left: cx - HIT / 2,
          top: laneY(lane) - HIT / 2,
          w: HIT,
          h: HIT,
          title: t.kinds[mark.kind],
          detail: t.mark(AI[source].name, mark.label, mark.verdict ? DECISION[mark.verdict].label : null, clockLabel(mark.at)),
        }),
      ),
    ].sort((a, b) => a.at - b.at || a.lane - b.lane),
  );

  const empty = $derived(items.length === 0);

  let shown = $state<string | null>(null);
  /** The target that takes the keyboard: the one last shown, else the latest. */
  let current = $state<string | null>(null);
  const tip = $derived(items.find((item) => item.key === shown));
  const focusKey = $derived(items.some((item) => item.key === current) ? current : (items.at(-1)?.key ?? null));
  const targets = $state<Record<string, HTMLButtonElement>>({});

  function onKey(event: KeyboardEvent, index: number): void {
    const step: Record<string, number> = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 };
    let next: number | undefined;
    if (event.key in step) next = Math.min(items.length - 1, Math.max(0, index + step[event.key]));
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = items.length - 1;
    else if (event.key === "Escape" && shown) {
      // The details close first; a second Escape closes the panel.
      event.stopPropagation();
      shown = null;
      return;
    }
    if (next === undefined) return;
    event.preventDefault();
    const key = items[next].key;
    current = key;
    targets[key]?.focus();
  }

  const tipLeft = $derived(tip ? Math.min(Math.max(0, tip.left + tip.w / 2), width) : 0);
</script>

<div class="frame" bind:clientWidth={width} style:height="{height}px">
  {#if width > 0}
    <svg class="plot" {width} {height} viewBox="0 0 {width} {height}" aria-hidden="true">
      {#each TICKS as minutes (minutes)}
        {@const tx = x(now - minutes * MINUTE_MS)}
        <line class="grid" x1={tx} x2={tx} y1={TOP / 2} y2={height - AXIS_H} />
        <text class="axis" x={minutes === 0 ? tx + END_PAD / 2 : tx} y={height - AXIS_H / 3} text-anchor={minutes === 0 ? "end" : minutes === 60 ? "start" : "middle"}>
          {minutes === 0 ? t.now : t.minutesAgo(minutes)}
        </text>
      {/each}
      {#each sources as source, lane (source)}
        <text class="name" x="0" y={laneY(lane)} dominant-baseline="central">{AI[source].name}</text>
        <line class="grid" x1={NAMES_W} x2={NAMES_W + plot} y1={laneY(lane)} y2={laneY(lane)} />
      {/each}
      {#each bars as bar (`${bar.source}-${bar.segment.start}`)}
        <rect
          class="bar"
          x={bar.x1}
          y={laneY(bar.lane) - BAR_H / 2}
          width={bar.x2 - bar.x1}
          height={BAR_H}
          rx={Math.min(BAR_H / 2, (bar.x2 - bar.x1) / 2)}
          style:fill={AI[bar.source].color}
          class:lit={shown === `bar-${bar.source}-${bar.segment.start}`}
        />
      {/each}
      <line class="now" x1={NAMES_W + plot} x2={NAMES_W + plot} y1={TOP / 2} y2={height - AXIS_H} />
      <circle class="now-dot" cx={NAMES_W + plot} cy={TOP / 2} r="3" />
      {#each marks as { source, lane, mark, cx } (`${source}-${mark.kind}-${mark.at}`)}
        {@const cy = laneY(lane)}
        <g class="mark" class:lit={shown === `mark-${source}-${mark.kind}-${mark.at}`} style:--fill={COLOR[mark.kind]}>
          {#if mark.kind === "permission"}
            <path d="M{cx} {cy - MARK_R - 1}l{MARK_R + 1} {MARK_R + 1}-{MARK_R + 1} {MARK_R + 1}-{MARK_R + 1}-{MARK_R + 1}z" />
          {:else}
            <circle {cx} {cy} r={MARK_R} />
            {#if mark.kind === "done"}
              <path class="glyph" d="M{cx - 2.6} {cy}l1.8 1.9 3.4-3.6" />
            {:else}
              <path class="glyph" d="M{cx - 2} {cy - 2}l4 4m0-4-4 4" />
            {/if}
          {/if}
        </g>
      {/each}
    </svg>
  {/if}
  {#if empty}
    <p class="empty" style:left="{NAMES_W}px" style:right="{END_PAD}px" style:top="{TOP}px" style:height="{sources.length * LANE_H}px">
      <span>{t.empty}</span>
    </p>
  {:else}
    <div class="targets" role="group" aria-label={t.targets}>
      {#each items as item, index (item.key)}
        <button
          type="button"
          class="target"
          style:left="{item.left}px"
          style:top="{item.top}px"
          style:width="{item.w}px"
          style:height="{item.h}px"
          tabindex={item.key === focusKey ? 0 : -1}
          aria-label={t.target(item.title, item.detail)}
          bind:this={targets[item.key]}
          onpointerenter={() => (shown = item.key)}
          onpointerleave={() => (shown = null)}
          onfocus={() => {
            shown = item.key;
            current = item.key;
          }}
          onblur={() => (shown = null)}
          onkeydown={(event) => onKey(event, index)}
        ></button>
      {/each}
    </div>
  {/if}
  {#if tip}
    <div class="tip" class:flip={tipLeft > width / 2} style:left="{tipLeft}px" style:top="{tip.top}px" aria-hidden="true">
      <b>{tip.title}</b> <span>{tip.detail}</span>
    </div>
  {/if}
</div>
<ul class="legend" aria-label={t.legend}>
  <li><i class="key-bar" aria-hidden="true"></i>{t.work}</li>
  <li>
    <svg class="key" viewBox="-7 -7 14 14" aria-hidden="true">
      <circle r={MARK_R} style:fill="var(--ok)" /><path class="glyph" d="M-2.6 0l1.8 1.9 3.4-3.6" />
    </svg>{t.kinds.done}
  </li>
  <li>
    <svg class="key" viewBox="-7 -7 14 14" aria-hidden="true">
      <circle r={MARK_R} style:fill="var(--err)" /><path class="glyph" d="M-2-2l4 4m0-4-4 4" />
    </svg>{t.kinds.failed}
  </li>
  <li>
    <svg class="key" viewBox="-8 -8 16 16" aria-hidden="true"><path d="M0-7l7 7-7 7-7-7z" style:fill="var(--warn)" /></svg>{t.kinds.permission}
  </li>
</ul>

<style>
  .frame {
    position: relative;
  }

  .plot {
    position: absolute;
    inset: 0;
    display: block;
    overflow: visible;
  }

  .grid {
    stroke: var(--line);
    stroke-width: 1;
  }

  .axis {
    fill: var(--ink-3);
    font: var(--fs-1) var(--font-mono);
  }

  .name {
    fill: var(--ink-2);
    font: 500 var(--fs-2) var(--font-ui);
  }

  .bar {
    transition: opacity var(--t-quick) var(--ease-out);
  }

  /* The hovered mark lifts: every other one steps back a little. */
  .frame:has(.lit) .bar:not(.lit),
  .frame:has(.lit) .mark:not(.lit) {
    opacity: var(--dim);
  }

  .now {
    stroke: var(--accent);
    stroke-width: 1.5;
  }

  .now-dot {
    fill: var(--accent);
  }

  .mark {
    transition: opacity var(--t-quick) var(--ease-out);
  }

  .mark circle,
  .mark > path:not(.glyph) {
    fill: var(--fill);
    stroke: var(--panel);
    stroke-width: 2;
  }

  .glyph {
    fill: none;
    stroke: var(--panel);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .empty {
    position: absolute;
    display: grid;
    place-items: center;
    padding: 0 var(--s4);
    color: var(--ink-3);
    font-size: var(--fs-3);
    text-align: center;
    pointer-events: none;
  }

  /* The sentence sits on the panel, above the grid lines. */
  .empty span {
    padding: var(--s1) var(--s3);
    border-radius: var(--r-pill);
    background: var(--panel);
  }

  .targets {
    position: absolute;
    inset: 0;
  }

  .target {
    position: absolute;
    border-radius: var(--r-pill);
  }

  .target:focus-visible {
    outline-offset: 0;
  }

  .tip {
    position: absolute;
    z-index: 1;
    max-width: calc(10 * var(--s8));
    padding: calc(var(--s1) + var(--gap-mark)) var(--s2);
    border-radius: var(--r-ctl);
    background: var(--surface-2);
    border: 1px solid var(--line-2);
    box-shadow: var(--shadow-soft);
    font-size: var(--fs-2);
    line-height: 1.35;
    transform: translate(calc(-1 * var(--s4)), calc(-100% - var(--s1)));
    pointer-events: none;
    animation: tip-in var(--t-quick) var(--ease-out);
  }

  .tip.flip {
    transform: translate(calc(-100% + var(--s4)), calc(-100% - var(--s1)));
  }

  .tip span {
    color: var(--ink-2);
  }

  @keyframes tip-in {
    from {
      opacity: 0;
    }
  }

  .legend {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s4);
    margin-top: var(--s1);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .key-bar {
    width: calc(var(--s4) + var(--gap-mark));
    height: var(--dot);
    border-radius: var(--r-pill);
    background: var(--ink-2);
  }

  .key {
    width: calc(var(--s3) + var(--gap-mark));
    height: calc(var(--s3) + var(--gap-mark));
    overflow: visible;
  }

  @media (prefers-reduced-motion: reduce) {
    .tip {
      animation: none;
    }
  }
</style>
