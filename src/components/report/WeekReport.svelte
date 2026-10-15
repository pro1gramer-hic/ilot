<!-- « Bilan de la semaine » in the Today tab: the week's hours of code, one bar per day (the
     busiest in the accent, its value on it; every value in the hover title and in a table
     for screen readers), the key figures set apart by hairlines, the AIs as a thin bar in
     their colors with a legend, and the actions: hide project names, export the image,
     erase the counters. -->
<script lang="ts">
  import { AI_SOURCES, sourceColor, sourceTool } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { DAY_LETTERS, DAY_NAMES, barHeights, figures, isEmpty, weekRange } from "../../lib/report";
  import { paintReport } from "../../lib/report-image";
  import { report } from "../../lib/stores/report.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { durationLabel } from "../../lib/timeline";
  import PanelIcon from "../panel/PanelIcon.svelte";
  import Segmented from "../panel/Segmented.svelte";
  import Button from "../settings/Button.svelte";
  import Switch from "../settings/Switch.svelte";

  const t = $derived(i18n.t.report);
  const weeks = $derived([
    ["last", t.weeks.last],
    ["current", t.weeks.current],
  ] as const);

  $effect(() => {
    if (!report.data && !report.loading && !report.error) void report.show();
  });

  const data = $derived(report.data);
  const heights = $derived(data ? barHeights(data) : []);
  const shown = $derived(data ? figures(data, report.hideProjects) : []);
  const time = $derived(shown.find((f) => f.id === "time"));
  const tiles = $derived(shown.filter((f) => f.id !== "time"));
  const agentsMs = $derived(data ? data.agents.reduce((sum, a) => sum + a.ms, 0) : 0);
  /** The AIs in their fixed order (never by rank), as a share of the week. */
  const agents = $derived(
    data ? [...data.agents].sort((a, b) => order(a.name) - order(b.name)).filter((a) => a.ms > 0) : [],
  );

  function order(source: string): number {
    const index = AI_SOURCES.findIndex((s) => s === source);
    return index === -1 ? AI_SOURCES.length : index;
  }
</script>

<div class="report">
  <div class="top">
    <Button variant="quiet" onclick={() => report.close()}>
      <PanelIcon name="back" />{t.back}
    </Button>
    <span id="report-week" class="visually-hidden">{t.week}</span>
    <Segmented options={weeks} value={report.week} labelledby="report-week" onchange={(week) => void report.show(week === "current" ? "current" : "last")} />
  </div>

  {#if report.loading && !data}
    <p class="quiet" role="status">{t.counting}</p>
  {:else if report.error && !data}
    <p class="problem" role="alert">{report.error}</p>
  {:else if !data || isEmpty(data)}
    <div class="empty">
      <h3>{t.emptyTitle}</h3>
      <p>{t.emptyText}</p>
    </div>
  {:else}
    <header class="head">
      <h3>{t.title(weekRange(data.monday))}</h3>
      {#if time}
        <p class="hero"><b>{time.value}</b> {t.hero}</p>
      {/if}
    </header>

    <section class="days" aria-labelledby="report-days">
      <h4 class="label" id="report-days">{t.days}</h4>
      <div class="chart" role="img" aria-label={t.chart}>
        {#each heights as height, day (day)}
          {@const busiest = day === data.busiestDay}
          <div class="slot" title={t.day(DAY_NAMES[day], durationLabel(data.days[day].activeMs))}>
            {#if busiest}
              <span class="value">{durationLabel(data.days[day].activeMs)}</span>
            {/if}
            <i class="bar" class:busiest style:--h={height} style:--i={day}></i>
            <span class="day" class:busiest>{DAY_LETTERS[day]}</span>
          </div>
        {/each}
      </div>
      <table class="visually-hidden">
        <caption>{t.days}</caption>
        <tbody>
          {#each data.days as counters, day (day)}
            <tr><th scope="row">{DAY_NAMES[day]}</th><td>{durationLabel(counters.activeMs)}</td></tr>
          {/each}
        </tbody>
      </table>
    </section>

    <dl class="tiles">
      {#each tiles as figure (figure.id)}
        <div>
          <dt>{figure.label}</dt>
          <dd>{figure.value}</dd>
          {#if figure.detail}<dd class="detail">{figure.detail}</dd>{/if}
        </div>
      {/each}
    </dl>

    {#if agents.length > 0}
      <section class="agents" aria-labelledby="report-agents">
        <h4 class="label" id="report-agents">{t.byAgent}</h4>
        <div class="share" aria-hidden="true">
          {#each agents as agent (agent.name)}
            <i style:flex-grow={agent.ms} style:background={sourceColor(agent.name)}></i>
          {/each}
        </div>
        <ul class="legend">
          {#each agents as agent (agent.name)}
            <li>
              <i style:background={sourceColor(agent.name)} aria-hidden="true"></i>
              {sourceTool(agent.name)}
              <span>{t.share(durationLabel(agent.ms), Math.round((agent.ms / Math.max(1, agentsMs)) * 100))}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <footer class="actions">
      <label class="hide">
        <Switch checked={report.hideProjects} onchange={(on) => (report.hideProjects = on)} />
        {t.hideProjects}
      </label>
      <span class="grow"></span>
      {#if report.exported !== "idle"}
        <span class="status" class:problem={report.exported === "error"} role="status">{t.exported[report.exported]}</span>
      {/if}
      <Button variant="primary" disabled={report.exported === "drawing"} onclick={() => void report.export((week, hide) => paintReport(week, hide, settings.value.mochi))}>
        <PanelIcon name="save" />{t.export}
      </Button>
    </footer>
  {/if}

  <p class="erase">
    {t.local}
    <Button variant="quiet" onclick={() => void report.clear()}>
      {report.confirmingClear ? t.confirmClear : t.clear}
    </Button>
  </p>
</div>

<style>
  .report {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
  }

  .head h3,
  .empty h3 {
    font-size: var(--fs-title);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .hero {
    margin-top: var(--s1);
    color: var(--ink-2);
    font-size: var(--fs-4);
  }

  .hero b {
    color: var(--ink);
    font-size: var(--fs-display);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .label {
    margin-bottom: var(--s2);
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  /* Seven columns on one hairline baseline; bars at most 24 px wide, rounded on top. */
  .chart {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    height: calc(4 * var(--s8));
    margin-bottom: var(--s5);
    border-bottom: 1px solid var(--line-2);
  }

  .slot {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
  }

  .bar {
    width: min(var(--s6), 60%);
    height: calc(var(--h) * (100% - var(--s6) - var(--s1)));
    min-height: calc(var(--h) * 4px);
    border-radius: var(--r-chip) var(--r-chip) 0 0;
    /* The other days in a quiet gray that still reads against the panel (3:1). */
    background: color-mix(in oklab, var(--ink-3) 65%, var(--panel));
    transform-origin: 50% 100%;
    animation: grow var(--t-bar) var(--ease-out) both;
    animation-delay: calc(var(--i) * var(--stagger));
  }

  .bar.busiest {
    background: var(--accent);
  }

  @keyframes grow {
    from {
      transform: scaleY(0);
    }
  }

  .value {
    margin-bottom: var(--s1);
    color: var(--ink);
    font-size: var(--fs-2);
    font-weight: 600;
    white-space: nowrap;
  }

  .day {
    position: absolute;
    bottom: calc(-1 * var(--s5));
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .day.busiest {
    color: var(--ink);
    font-weight: 600;
  }

  /* The figures, three columns set apart by hairlines. */
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin-top: var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .tiles div {
    min-width: 0;
    padding: var(--s3) var(--s4);
  }

  .tiles div:not(:nth-child(3n + 1)) {
    border-left: 1px solid var(--line);
  }

  .tiles div:nth-child(n + 4) {
    border-top: 1px solid var(--line);
  }

  dt {
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  dd {
    margin-top: calc(var(--s1) / 2);
    font-size: var(--fs-4);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  dd.detail {
    margin-top: 0;
    color: var(--ink-3);
    font-size: var(--fs-2);
    font-weight: 400;
  }

  /* The AIs' colors as a thin bar only, cut by the panel's own color. */
  .share {
    display: flex;
    gap: var(--gap-mark);
    height: var(--bar);
  }

  .share i {
    flex-basis: 0;
    min-width: var(--s1);
    border-radius: var(--r-mark);
  }

  .legend {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2) var(--s5);
    margin-top: var(--s3);
    font-size: var(--fs-3);
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .legend i {
    width: var(--mark);
    height: var(--mark);
    border-radius: var(--r-pill);
  }

  .legend span {
    color: var(--ink-3);
  }

  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3);
    padding-top: var(--s4);
    border-top: 1px solid var(--line);
  }

  .hide {
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--fs-3);
    cursor: pointer;
  }

  .grow {
    flex: 1;
  }

  .status,
  .quiet {
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .problem {
    color: var(--err);
    font-size: var(--fs-3);
  }

  .empty p,
  .erase {
    color: var(--ink-3);
    font-size: var(--fs-2);
    line-height: 1.5;
  }

  .empty p {
    margin-top: var(--s2);
    color: var(--ink-2);
    font-size: var(--fs-3);
  }

  .erase {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
  }

  @media (prefers-reduced-motion: reduce) {
    .bar {
      animation: none;
    }
  }

  @container panel (max-width: 760px) {
    .tiles {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .tiles div:not(:nth-child(3n + 1)) {
      border-left: 0;
    }

    .tiles div:nth-child(even) {
      border-left: 1px solid var(--line);
    }

    .tiles div:nth-child(n + 3) {
      border-top: 1px solid var(--line);
    }
  }
</style>
