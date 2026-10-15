<!-- « Aujourd'hui »: the last hour of every AI on a live timeline, then three columns set
     apart by hairlines (what runs now and since when, the last decision, today's
     numbers), then the latest messages. Everything comes from memory: nothing is read
     back from disk. -->
<script lang="ts">
  import { AI, DECISION, decisionsSince, sessionTool, sourceTool } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { stillness } from "../../lib/motion.svelte";
  import { appColor } from "../../lib/notifications";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { history } from "../../lib/stores/history.svelte";
  import { notifications } from "../../lib/stores/notifications.svelte";
  import { report } from "../../lib/stores/report.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import { activeToday, clockLabel, doing, durationLabel, elapsedLabel, startOfDay } from "../../lib/timeline";
  import AppTile from "../AppTile.svelte";
  import { inbox } from "../panel/inbox";
  import Timeline from "../panel/Timeline.svelte";
  import WeekReport from "../report/WeekReport.svelte";
  import Button from "../settings/Button.svelte";

  /** Recent messages shown here; the Messages tab has them all. */
  const RECENT = 3;

  // Live: bars grow and times run once a second, only while the tab can be seen.
  $effect(() => {
    if (stillness.current || report.open) return;
    timeline.tick();
    const timer = setInterval(() => timeline.tick(), 1000);
    return () => clearInterval(timer);
  });

  const t = $derived(i18n.t.today);
  const now = $derived(timeline.now);
  const running = $derived(sessions.list.filter((s) => s.status === "active"));
  const last = $derived(decisions.list[0]);
  const midnight = $derived(startOfDay(now));
  const stats = $derived([
    { label: t.stats.finished, value: String(history.log.finished.length) },
    { label: t.stats.failures, value: String(history.log.failures) },
    { label: t.stats.decisions, value: String(decisionsSince(decisions.list, midnight)) },
    { label: t.stats.activeTime, value: durationLabel(activeToday(timeline.value.day, now)) },
  ]);
  const recent = $derived(inbox(notifications.queue, notifications.postponed).slice(0, RECENT));
  const hidden = $derived(settings.value.notifications.hideContent);
</script>

{#if report.open}
  <WeekReport />
{:else}
<div class="today">
  <section class="block" aria-labelledby="today-activity">
    <div class="head">
      <h3 class="label" id="today-activity">{t.activity}</h3>
      <span class="hint">{t.activityHint}</span>
    </div>
    <div class="box timeline">
      <Timeline />
    </div>
  </section>

  <div class="block box columns">
    <section aria-labelledby="today-running">
      <h3 class="label" id="today-running">{t.running}</h3>
      {#if running.length > 0}
        <ul class="running">
          {#each running as session (session.id)}
            <li>
              <i class="dot" style:background={AI[session.source].color} aria-hidden="true"></i>
              <span class="what">
                <b>{sessionTool(session)}</b>
                <small>{doing(session)}</small>
              </span>
              <time class="mono">
                <span class="visually-hidden">{t.since}</span>{elapsedLabel(now - (timeline.value.since[session.id] ?? now))}
              </time>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="quiet">{t.nobodyWorks}</p>
      {/if}
    </section>
    <section aria-labelledby="today-decision">
      <h3 class="label" id="today-decision">{t.lastDecision}</h3>
      {#if last}
        <p class="verdict {DECISION[last.decision].tone}"><i aria-hidden="true"></i>{DECISION[last.decision].label}</p>
        <code class="command" title={last.command}>{last.command}</code>
        <p class="by">{t.decidedBy(sourceTool(last.source), clockLabel(last.at))}</p>
      {:else}
        <p class="quiet">{t.noDecision}</p>
      {/if}
    </section>
    <section aria-labelledby="today-numbers">
      <h3 class="label" id="today-numbers">{t.numbers}</h3>
      <dl class="stats">
        {#each stats as stat (stat.label)}
          <div><dt>{stat.label}</dt><dd>{stat.value}</dd></div>
        {/each}
      </dl>
    </section>
  </div>

  <section class="block" aria-labelledby="today-messages">
    <div class="head">
      <h3 class="label" id="today-messages">{t.recent}</h3>
      <Button onclick={() => (island.tab = "messages")}>{t.seeAll}</Button>
    </div>
    {#if recent.length > 0}
      <ul class="box messages">
        {#each recent as message (message.id)}
          <li>
            <AppTile icon={message.icon} monogram={message.monogram} size={28} ring={appColor(message.app)} />
            <span class="what">
              <b>{message.sender}</b>
              <small>{message.masked || hidden ? t.hidden(message.appLabel) : message.text}</small>
            </span>
            <time class="mono">{clockLabel(message.receivedAt)}</time>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="box quiet empty">{t.noMessage}</p>
    {/if}
  </section>

  <section class="block box week" aria-labelledby="today-week">
    <span class="what">
      <b id="today-week">{t.week}</b>
      <small>{t.weekHint}</small>
    </span>
    <Button onclick={() => void report.show("last")}>{t.seeWeek}</Button>
  </section>
</div>
{/if}

<style>
  .today {
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    min-height: var(--h-ctl);
    margin-bottom: var(--s2);
  }

  .label {
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .hint {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .box {
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .timeline {
    padding: var(--s3) var(--s4) var(--s2);
    background: linear-gradient(to bottom, var(--hover), transparent 30%);
  }

  /* Three columns set apart by hairlines; the first one is a little wider. */
  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr) minmax(0, 1fr);
  }

  .columns > section {
    min-width: 0;
    padding: var(--s3) var(--s4) var(--s4);
  }

  .columns > section + section {
    border-left: 1px solid var(--line);
  }

  .running {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    margin-top: var(--s3);
  }

  .running li,
  .messages li {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
  }

  .dot {
    flex: none;
    width: var(--mark);
    height: var(--mark);
    border-radius: var(--r-pill);
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.35;
  }

  .what b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  .what small {
    color: var(--ink-2);
    font-size: var(--fs-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  time {
    flex: none;
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .quiet {
    margin-top: var(--s3);
    color: var(--ink-3);
    font-size: var(--fs-3);
  }

  .verdict {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-top: var(--s3);
    font-size: var(--fs-4);
    font-weight: 600;
  }

  .verdict i {
    flex: none;
    width: var(--mark);
    height: var(--mark);
    border-radius: var(--r-pill);
    background: var(--ink-3);
  }

  .verdict.ok i {
    background: var(--ok);
  }

  .verdict.err i {
    background: var(--err);
  }

  .verdict.wait i {
    background: var(--warn);
  }

  .command {
    display: block;
    margin-top: var(--s2);
    padding: calc(var(--s1) + var(--gap-mark)) var(--s2);
    border-radius: var(--r-chip);
    background: var(--surface);
    border: 1px solid var(--line);
    font: var(--fs-2) var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .by {
    margin-top: var(--s2);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .stats {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    margin-top: var(--s3);
    font-size: var(--fs-body);
  }

  .stats div {
    display: flex;
    justify-content: space-between;
    gap: var(--s2);
  }

  dt {
    color: var(--ink-2);
  }

  dd {
    font-weight: 600;
    white-space: nowrap;
  }

  .messages {
    list-style: none;
    overflow: hidden;
  }

  .messages li {
    padding: var(--s3) var(--s4);
  }

  .messages li + li {
    border-top: 1px solid var(--line);
  }

  .empty {
    margin: 0;
    padding: var(--s4);
    text-align: center;
  }

  /* The week's report, one row away. */
  .week {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) var(--s4);
  }

  /* A narrow panel stacks the three columns. */
  @container panel (max-width: 760px) {
    .columns {
      grid-template-columns: minmax(0, 1fr);
    }

    .columns > section + section {
      border-left: 0;
      border-top: 1px solid var(--line);
    }
  }
</style>
