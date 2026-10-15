<!-- The left card of the hub's home (09, section 8 bis): the followed session, Mochi large
     in its colors with his state badge, and what goes on, from the stores only: what the
     agent does now and its latest actions, the command it runs and for how long, its
     failure, its wait, or the music and its controls. îlot never reads an agent's files:
     where the demo shows code, the app shows the agent's journal (the browser preview
     stages the demo's code). Another session arrives out of a light blur. -->
<script lang="ts">
  import { answersInTerminal } from "../../lib/agents";
  import { type HubSession, canFollow, focusKind, liveSteps, stepCount, stepOfActivity } from "../../lib/hub";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { island } from "../../lib/island/controller.svelte";
  import { detachLabel, progress, timeLabel } from "../../lib/media";
  import { media } from "../../lib/stores/media.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import { goToTerminal } from "../../lib/terminal";
  import { clockLabel, elapsedLabel, laneOf } from "../../lib/timeline";
  import Icon from "../Icon.svelte";
  import Subagents from "../agents/Subagents.svelte";
  import RoundButton from "../music/RoundButton.svelte";
  import Transport from "../music/Transport.svelte";
  import Code from "./Code.svelte";
  import HubIcon, { type HubIconName } from "./HubIcon.svelte";
  import HubMochi from "./HubMochi.svelte";
  import { swap } from "./motion";
  import { play } from "./play.svelte";

  let { followed }: { followed: HubSession | undefined } = $props();

  /** Lines of the agent's journal on the card. */
  const JOURNAL_LINES = 2;
  const ICON_OF_STEP: HubIconName[] = ["read", "edit", "term"];

  const t = $derived(i18n.t.hub.focus);
  const kind = $derived(followed ? focusKind(followed) : null);
  const agent = $derived(followed?.kind === "agent" ? followed.session : undefined);
  const track = $derived(followed?.kind === "music" ? followed.track : undefined);
  const lane = $derived(agent ? laneOf(timeline.value, agent.source) : undefined);
  const journal = $derived(lane?.steps ?? []);
  const steps = $derived(agent ? liveSteps(agent, journal.map((step) => step.text)) : []);
  const verb = $derived(agent?.activity?.verb ?? "");
  const verbIcon = $derived(ICON_OF_STEP[stepOfActivity(agent?.activity) ?? 0]);
  const failure = $derived(lane?.marks.filter((mark) => mark.kind === "failed").at(-1)?.label ?? agent?.task ?? "");
  const elapsed = $derived(agent ? timeline.now - (timeline.value.since[agent.id] ?? timeline.now) : 0);
  /** The demo's two lines of code, staged by the browser preview only. */
  const snippet = $derived.by(() => {
    const stage = island.hubStage;
    if (!stage || stage.sessionId !== agent?.id) return null;
    return stage.lines.filter((line) => stage.snippet.includes(line.n));
  });
  const follows = $derived(canFollow(followed));

  const capitalize = (text: string): string => text.charAt(0).toLocaleUpperCase(i18n.locale) + text.slice(1);

  /** The session followed live stays on the left card when the hub comes back home. */
  function followLive(): void {
    if (!followed) return;
    play.chosen = followed.key;
    island.showHub("live");
  }

  function open(event: MouseEvent): void {
    if (follows && !(event.target instanceof Element && event.target.closest("button"))) followLive();
  }
</script>

<!-- The whole card follows the session on a click; the keyboard has its own button. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="card" class:follows style:--sc={followed?.color} onclick={open}>
  {#if followed}
    <HubMochi palette={followed.palette} status={followed.status} />
    {#key followed.key}
      <div class="body" in:swap>
        <p class="head">
          <i class="dot" aria-hidden="true"></i>
          <b>{followed.name}</b>
          <span>{followed.tool}</span>
          {#if agent?.subagents}<Subagents subagents={agent.subagents} compact />{/if}
          {#if follows}<em>{stepCount(steps)}</em>{/if}
        </p>
        {#if kind === "music" && track}
          <p class="track" title={track.artist}><span>{track.artist || track.app}</span></p>
          <div class="progress" aria-hidden="true"><i style:width="{progress(track, media.now) * 100}%"></i></div>
          <p class="time">{timeLabel(track, media.now)}</p>
          <div class="controls">
            <Transport {track} />
            <RoundButton aria-label={detachLabel(track)} title={detachLabel(track)} onclick={() => void media.detach(track)}>
              <Icon name="pip" size={14} />
            </RoundButton>
          </div>
        {:else if kind === "failed"}
          <p class="act err"><HubIcon name="cross" size={14} /><b>{t.failed}</b></p>
          <p class="mini">{failure}</p>
          <div class="actions">
            <button type="button" class="btn sm" onclick={() => island.openPanel("agents")}>{i18n.t.hub.viewError}</button>
          </div>
        {:else if kind === "waiting" && agent}
          <p class="act wait"><HubIcon name="wait" size={14} /><b>{t.waiting}</b></p>
          <p class="mini mono">{agent.activity?.target ?? agent.task}</p>
          <p class="hint">{answersInTerminal(agent) ? t.inTerminal : t.inIsland}</p>
        {:else if kind === "done" && agent}
          <p class="act ok"><HubIcon name="check" size={14} /><b>{t.done}</b><span>{agent.task}</span></p>
          {@render lines()}
        {:else if kind === "command" && agent}
          <p class="act"><HubIcon name="term" size={14} /><b>{t.runs}</b><code>{agent.activity?.target ?? ""}</code></p>
          <p class="elapsed">
            <strong>{elapsedLabel(elapsed)}</strong>
            <span>{t.running}</span>
          </p>
          {@render lines(1)}
        {:else if kind === "action" && agent}
          <p class="act">
            <HubIcon name={verbIcon} size={14} /><b>{capitalize(verb)}</b>
            {#if agent.activity?.target}<code>{agent.activity.target}</code>{/if}
          </p>
          {#if snippet}
            <p class="mini mono code">
              {#each snippet as line (line.n)}<span class="line"><span class="n">{line.n}</span><Code tokens={line.tokens} /></span>{/each}
            </p>
          {:else}
            {@render lines()}
          {/if}
        {:else if agent}
          <p class="act idle"><HubIcon name="sleep" size={14} /><b>{t.idle}</b></p>
          <p class="hint">{t.nothing(agent.project)}</p>
        {/if}
        {#if follows || agent}
          <span class="links">
            {#if follows}
              <button type="button" class="follow" onclick={followLive}>{t.follow}</button>
            {/if}
            {#if agent}
              <button type="button" class="follow" onclick={() => void goToTerminal(agent.id)}>{i18n.t.hub.goToTerminal}</button>
            {/if}
          </span>
        {/if}
      </div>
    {/key}
  {:else}
    <HubMochi palette="lavande" status="idle" move="sleep" face="sleep" />
    <div class="body" in:swap>
      <p class="head"><b>{t.none}</b></p>
      <p class="hint">{t.noneHint}</p>
      <div class="actions">
        <button type="button" class="btn sm" onclick={() => island.openPanel("agents")}>{t.plug}</button>
      </div>
    </div>
  {/if}
</div>

{#snippet lines(count = JOURNAL_LINES)}
  {#if journal.length > 0}
    <ol class="mini journal" aria-label={i18n.t.hub.recent}>
      {#each journal.slice(0, count) as step (step.at + step.text)}
        <li><time class="n">{clockLabel(step.at)}</time><span>{step.text}</span></li>
      {/each}
    </ol>
  {/if}
{/snippet}

<style>
  .card {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
    height: 100%;
    padding: var(--s3) var(--s4) var(--s3) var(--s2);
    overflow: hidden;
    border-radius: var(--r-card);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow var(--t-quick);
  }

  .card.follows {
    cursor: pointer;
  }

  .card.follows:hover {
    box-shadow: inset 0 0 0 1px var(--line-2);
  }

  /* The session's color, as a soft light behind Mochi (identity, never a block). */
  .card::before {
    content: "";
    position: absolute;
    left: -40px;
    top: 20px;
    width: 200px;
    height: 160px;
    border-radius: 50%;
    background: radial-gradient(closest-side, var(--sc, var(--accent)), transparent);
    opacity: 0.16;
    pointer-events: none;
  }

  .body {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(--s1) + var(--gap-mark));
  }

  .head {
    display: flex;
    align-items: center;
    gap: calc(var(--s1) + var(--gap-mark));
    min-width: 0;
    font-size: var(--fs-3);
  }

  .head b {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .head span {
    color: var(--ink-2);
    white-space: nowrap;
  }

  .head em {
    margin-left: auto;
    color: var(--ink-3);
    font: var(--fs-2) var(--font-mono);
    font-style: normal;
  }

  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--sc);
  }

  .act {
    display: flex;
    align-items: center;
    gap: calc(var(--s1) + var(--gap-mark));
    min-width: 0;
    font-size: var(--fs-3);
  }

  .act b {
    font-weight: 600;
    white-space: nowrap;
  }

  .act span {
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .act code {
    min-width: 0;
    color: var(--ink-2);
    font: var(--fs-2) var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .act.err {
    color: var(--err);
  }

  .act.ok {
    color: var(--ok);
  }

  .act.wait {
    color: var(--warn);
  }

  .act.idle {
    color: var(--ink-2);
  }

  .mini {
    margin: 0;
    padding: var(--s1) var(--s2);
    border-radius: var(--r-ctl);
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.6;
    overflow: hidden;
  }

  .mini.mono {
    font-family: var(--font-mono);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .code {
    display: flex;
    flex-direction: column;
    white-space: pre;
  }

  .journal {
    list-style: none;
  }

  .journal li,
  .code .line {
    display: flex;
    gap: var(--s2);
    min-width: 0;
    white-space: nowrap;
  }

  .journal span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .n {
    flex: none;
    color: var(--ink-3);
    font-family: var(--font-mono);
  }

  .elapsed {
    display: flex;
    align-items: baseline;
    gap: var(--s2);
  }

  /* A running time, not a progress: no agent says how far its command is. */
  .elapsed strong {
    font-size: var(--fs-display);
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }

  .elapsed span,
  .time {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .track {
    color: var(--ink-2);
    font-size: var(--fs-sub);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .progress {
    height: 4px;
    border-radius: var(--r-mark);
    background: var(--line-2);
    overflow: hidden;
  }

  .progress i {
    display: block;
    height: 100%;
    border-radius: var(--r-mark);
    background: var(--ink);
  }

  .controls,
  .actions {
    display: flex;
    align-items: center;
    gap: calc(var(--s1) + var(--gap-mark));
  }

  .hint {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .follow {
    align-self: flex-start;
    margin-left: -4px;
    padding: var(--gap-mark) var(--s1);
    border-radius: var(--r-chip);
    color: var(--ink-3);
    font-size: var(--fs-2);
    transition: color var(--t-quick);
  }

  .card.follows:hover .follow,
  .follow:hover {
    color: var(--ink);
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 0 var(--s2);
  }

  .follow:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
