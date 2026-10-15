<!-- One AI in the Agents tab: its dot, name and folder, its state in a chip, the switch that
     plugs it into îlot (or a button for any other tool), its latest actions with their
     time, and how much of the last hour it spent at work. -->
<script lang="ts">
  import { AI, type AiSource, agentState, answersInTerminal, sessionTool } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { decisions } from "../../lib/stores/decisions.svelte";
  import { goToTerminal } from "../../lib/terminal";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import { busyShare, clockLabel, laneOf } from "../../lib/timeline";
  import { connectors } from "../panel/connectors.svelte";
  import Button from "../settings/Button.svelte";
  import Switch from "../settings/Switch.svelte";
  import Status from "./Status.svelte";
  import Subagents from "./Subagents.svelte";

  /** `compact`: an agent îlot follows without answering for it, listed under the main
   * ones; it shows its actions and its hour only once it worked. */
  let { source, compact = false }: { source: AiSource; compact?: boolean } = $props();

  /** Agents îlot follows without ever answering for them (docs/AGENTS.md). */
  const FOLLOWED_ONLY: readonly AiSource[] = ["copilot", "opencode", "amp", "bob", "kiro"];

  const t = $derived(i18n.t.agentsPanel.card);
  const other = $derived(source === "generic");
  const name = $derived(other ? t.other : AI[source].tool);
  const view = $derived(connectors.view(source));
  const plug = $derived(connectors.plug(source));
  const mine = $derived(sessions.list.filter((s) => s.source === source));
  const shown = $derived(mine.find((s) => s.status === "waiting") ?? mine.find((s) => s.status === "active") ?? mine[0]);
  const standing = $derived(
    connectors.list === null ? { label: t.searching, tone: "idle" as const } : agentState(sessions.list, source, plug, decisions.suspended),
  );
  const helpers = $derived(mine.flatMap((s) => s.subagents ?? []));
  const lane = $derived(laneOf(timeline.value, source));
  const share = $derived(Math.round(busyShare(lane, timeline.now) * 100));
  const places = $derived(view?.targets.filter((target) => target.status !== "notInstalled") ?? []);
  const busy = $derived(connectors.busy === source);

  // The switch shows what the core says: after each answer it goes back to the truth.
  let on = $state(false);
  $effect(() => {
    void connectors.list;
    on = plug === "on";
  });

  const where = $derived.by(() => {
    if (plug === "absent") return other ? t.noWsl : t.notInstalled(name);
    const plugged = places.filter((target) => target.status === "connected").map((target) => target.label);
    if (plugged.length > 0) return t.pluggedIn(plugged);
    return t.installedIn(places.map((target) => target.label));
  });

  /** One line that matters for this AI right now, if any. */
  const note = $derived.by((): { todo: boolean; text: string } | null => {
    const notes = t.notes;
    if (plug === "repair" && source === "claude-code") return { todo: true, text: notes.claudeRepair };
    if (plug === "repair") return { todo: true, text: notes.repair };
    if (source === "codex" && plug === "on") return { todo: true, text: notes.codexHooks };
    if ((source === "opencode" || source === "amp") && plug === "on" && mine.length === 0) {
      return { todo: true, text: notes.reload(name) };
    }
    if (source === "gemini-cli" && mine.some(answersInTerminal)) return { todo: false, text: notes.geminiTerminal };
    if (mine.some(answersInTerminal)) return { todo: false, text: notes.inTerminal(AI[source].name) };
    if (source === "claude-code" && plug === "on" && mine.length === 0) return { todo: false, text: notes.claudeTrust };
    if (source === "cursor") return { todo: false, text: notes.cursor };
    if (FOLLOWED_ONLY.includes(source)) return { todo: false, text: notes.followedOnly };
    if (other) return { todo: false, text: notes.other };
    return null;
  });

  let reaching = $state(false);

  async function reach(): Promise<void> {
    if (!shown || reaching) return;
    reaching = true;
    try {
      await goToTerminal(shown.id);
    } finally {
      reaching = false;
    }
  }
</script>

<article
  class="agent"
  class:off={plug !== "on" && mine.length === 0}
  class:compact
  aria-labelledby="agent-{source}"
  data-agent={compact ? undefined : source}
  data-followed={compact ? source : undefined}
>
  <header>
    <i class="dot" style:background={AI[source].color} aria-hidden="true"></i>
    <h3 id="agent-{source}">{name}</h3>
    {#if shown}<span class="folder mono">{shown.project}</span>{/if}
    <Subagents subagents={helpers} />
    <Status tone={standing.tone} label={standing.label} chip />
    <span class="control">
      {#if other}
        <Button
          variant={plug === "on" ? "plain" : "primary"}
          disabled={plug === "absent" || busy}
          onclick={() => void connectors.switch(source, plug !== "on")}
        >
          {busy ? t.wait : plug === "on" ? t.disconnect : t.connect}
        </Button>
      {:else}
        <label class="plug">
          <span class="visually-hidden">{t.connectName(name)}</span>
          <Switch bind:checked={on} disabled={plug === "absent" || busy} onchange={(value) => void connectors.switch(source, value)} />
        </label>
      {/if}
    </span>
  </header>

  {#if lane.steps.length > 0}
    <ol class="steps" aria-label={t.steps(name)}>
      {#each lane.steps as step (step.at + step.text)}
        <li><time class="mono">{clockLabel(step.at)}</time><span>{step.text}</span></li>
      {/each}
    </ol>
  {:else if plug === "on" && !compact}
    <p class="quiet">{t.noAction(other ? null : name)}</p>
  {/if}

  {#if (plug === "on" && !compact) || share > 0}
    <div class="busy" style:--c={AI[source].color}>
      <span>{t.busy(share)}</span>
      <span class="lane" aria-hidden="true"><i style:transform="scaleX({share / 100})"></i></span>
    </div>
  {/if}

  <div class="foot">
    <p class="where">{where}{#if mine.length > 1}{t.sessions(mine.length, mine.map(sessionTool).join(", "))}{/if}</p>
    {#if shown}
      <Button variant="quiet" disabled={reaching} title={t.reachTip(shown.project)} onclick={() => void reach()}>
        {t.reach}
      </Button>
    {/if}
  </div>
  {#if note}
    <p class="note" class:todo={note.todo}>{#if note.todo}<i aria-hidden="true"></i>{/if}<span>{note.text}</span></p>
  {/if}
</article>

<style>
  .agent {
    padding: var(--s3) var(--s4) var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r-card);
    transition: border-color var(--t-quick) var(--ease-out);
  }

  .agent.compact {
    padding: var(--s3) var(--s4);
  }

  .agent:hover {
    border-color: var(--line-2);
  }

  header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s2) var(--s3);
  }

  .dot {
    flex: none;
    width: calc(var(--mark) + var(--gap-mark));
    height: calc(var(--mark) + var(--gap-mark));
    border-radius: var(--r-pill);
  }

  .off .dot {
    opacity: var(--dim);
  }

  h3 {
    font-size: var(--fs-4);
    font-weight: 600;
  }

  .folder {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  .control {
    margin-left: auto;
    display: flex;
    align-items: center;
  }

  .plug {
    display: flex;
    padding: var(--s1);
    cursor: pointer;
  }

  .steps {
    list-style: none;
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--s1) var(--s4);
    margin-top: var(--s3);
    font-size: var(--fs-3);
  }

  .steps li {
    display: contents;
  }

  .steps time {
    color: var(--ink-3);
    font-size: var(--fs-2);
    line-height: 1.6;
  }

  .steps span {
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .steps li:first-child span {
    color: var(--ink);
  }

  .busy {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    align-items: center;
    gap: var(--s4);
    margin-top: var(--s3);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  .lane {
    height: var(--dot);
    border-radius: var(--r-pill);
    background: var(--surface-2);
    overflow: hidden;
  }

  .lane i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--c);
    transform-origin: left;
    transition: transform var(--t-bar) var(--ease-out);
  }

  .where,
  .quiet {
    margin-top: var(--s2);
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  /* Where it is plugged, and the way to its terminal. */
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
  }

  .foot :global(button) {
    margin-top: var(--s2);
  }

  .quiet {
    margin-top: var(--s3);
  }

  .note {
    display: flex;
    align-items: flex-start;
    gap: var(--s2);
    margin-top: var(--s2);
    color: var(--ink-3);
    font-size: var(--fs-2);
    line-height: 1.45;
  }

  /* A step left to the user: a quiet box, the amber only on its dot. */
  .note.todo {
    padding: var(--s2);
    border-radius: var(--r-ctl);
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--ink-2);
  }

  .note.todo i {
    flex: none;
    margin-top: calc((1lh - var(--dot)) / 2);
    width: var(--dot);
    height: var(--dot);
    border-radius: var(--r-pill);
    background: var(--warn);
  }

  @media (prefers-reduced-motion: reduce) {
    .lane i {
      transition: none;
    }
  }
</style>
