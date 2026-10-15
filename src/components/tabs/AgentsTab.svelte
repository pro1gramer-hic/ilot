<!-- « Agents »: a request waiting for the user first (the full permission card, unchanged,
     or the agent's questions), then one card per AI and one for any other tool, the agents
     îlot follows without answering for them, then the decisions journal. -->
<script lang="ts">
  import type { AiSource } from "../../lib/agents";
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { stillness } from "../../lib/motion.svelte";
  import { asksQuestions, permissions } from "../../lib/stores/permissions.svelte";
  import { timeline } from "../../lib/stores/timeline.svelte";
  import AgentCard from "../agents/AgentCard.svelte";
  import DecisionLog from "../agents/DecisionLog.svelte";
  import { connectors } from "../panel/connectors.svelte";
  import PermissionCard from "../PermissionCard.svelte";
  import QuestionCard from "../QuestionCard.svelte";

  const t = $derived(i18n.t.agentsPanel);
  const CARDS: readonly AiSource[] = ["claude-code", "gemini-cli", "codex", "generic"];
  /** Followed for their activity only (docs/AGENTS.md). */
  const FOLLOWED: readonly AiSource[] = ["cursor", "copilot", "opencode", "amp", "bob", "kiro"];
  /** The share of the hour moves slowly: a look every 10 s is enough. */
  const TICK_MS = 10_000;

  $effect(() => {
    void connectors.load();
  });

  $effect(() => {
    if (stillness.current) return;
    timeline.tick();
    const timer = setInterval(() => timeline.tick(), TICK_MS);
    return () => clearInterval(timer);
  });
</script>

<div class="agents">
  {#if permissions.current}
    <!-- A new card for each request: no focus, scroll or answer carries over to the next. -->
    {#key permissions.current.id}
      {#if asksQuestions(permissions.current)}
        <QuestionCard request={permissions.current} waiting={permissions.queue.length - 1} />
      {:else}
        <PermissionCard request={permissions.current} waiting={permissions.queue.length - 1} />
      {/if}
    {/key}
  {/if}
  <div class="cards">
    {#each CARDS as source (source)}
      <AgentCard {source} />
    {/each}
  </div>
  {#if connectors.error}
    <p class="error" role="alert">{connectors.error}</p>
  {/if}
  <p class="safe">{t.safe.before}<code>.avant-ilot</code>{t.safe.after}</p>

  <section class="followed" aria-labelledby="followed-title">
    <div class="head">
      <h3 class="label" id="followed-title">{t.followed}</h3>
      <span class="label">{t.followedCount(FOLLOWED.length)}</span>
    </div>
    <p class="lead">{t.followedLead}</p>
    <div class="cards">
      {#each FOLLOWED as source (source)}
        <AgentCard {source} compact />
      {/each}
    </div>
    <article class="aside" aria-labelledby="desktop-title">
      <h4 id="desktop-title">Claude Desktop</h4>
      <p>{t.desktop}</p>
    </article>
    <article class="aside" aria-labelledby="badge-title">
      <h4 id="badge-title">{t.badgeTitle}</h4>
      <p>{t.badge.add} <code>ilot_agent</code> {t.badge.middle} <code>docs/AGENTS.md</code>{t.badge.end}</p>
    </article>
  </section>

  <DecisionLog />
</div>

<style>
  .agents {
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }

  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }

  .safe {
    margin-top: calc(-1 * var(--s3));
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  code {
    font-family: var(--font-mono);
    color: var(--ink-2);
  }

  .error {
    color: var(--err);
    font-size: var(--fs-3);
  }

  /* Same head as the decisions journal: a small label and a count. */
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s3);
    margin-bottom: var(--s2);
  }

  .label {
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .lead {
    margin-bottom: var(--s3);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  /* Agents with nothing to plug: a quiet note, no switch. */
  .aside {
    margin-top: var(--s3);
    padding: var(--s3) var(--s4);
    border-radius: var(--r-card);
    background: var(--surface);
  }

  .aside h4 {
    font-size: var(--fs-3);
    font-weight: 600;
  }

  .aside p {
    margin-top: var(--s1);
    color: var(--ink-2);
    font-size: var(--fs-2);
    line-height: 1.5;
  }
</style>
