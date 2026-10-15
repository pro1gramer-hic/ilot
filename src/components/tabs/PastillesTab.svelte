<!-- « Pastilles » (prompt 12, lot 4): small blocks of state in the island, each with its own
     little Mochi. None is on at first. The main tool (VS Code, Cursor, Codex or
     Antigravity), then up to four more: Ollama, LM Studio, Spotify, and services added one
     by one with a read-only token kept in the Windows Credential Manager. A pastille
     removed stops being read at once, and a service's token is forgotten with it. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import Mochi from "../../lib/mochi/Mochi.svelte";
  import {
    LOCALS,
    MAX_EXTRAS,
    type ServiceInfo,
    TOOLS,
    type ToolId,
    agentsIn,
    lookOf,
    stanceOf,
    toolLine,
    toolProject,
  } from "../../lib/pastilles";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Segmented from "../panel/Segmented.svelte";
  import ServiceForm from "../pastilles/ServiceForm.svelte";

  const t = $derived(i18n.t.pastilles);
  const toolOptions: readonly (readonly [string, string])[] = $derived([["", t.tab.noTool], ...TOOLS.map((tool) => [tool.id, tool.name] as const)]);

  let adding = $state<ServiceInfo | null>(null);

  $effect(() => {
    void pastilles.loadCatalog();
  });

  const extras = $derived(pastilles.config.extras);
  const full = $derived(extras.length >= MAX_EXTRAS);
  const toolView = $derived(pastilles.view("tool"));
  const project = $derived(toolProject(toolView?.tool, sessions.list));
  const toolName = $derived(TOOLS.find((tool) => tool.id === pastilles.config.tool)?.name ?? "");

  const added = (id: string): boolean => extras.some((e) => e.id === id);

  function lineOf(id: string): string {
    const view = pastilles.view(id);
    return view?.problem ?? view?.line ?? t.reading;
  }

  function choose(service: ServiceInfo): void {
    adding = adding?.id === service.id ? null : service;
  }
</script>

<div class="pastilles">
  <p class="intro">{t.tab.intro}</p>

  <section aria-labelledby="tool-title">
    <h3 class="label" id="tool-title">{t.tab.tool}</h3>
    <div class="box tool">
      <Segmented options={toolOptions} value={pastilles.config.tool} labelledby="tool-title" onchange={(value) => pastilles.setTool(value as ToolId | "")} />
      {#if pastilles.config.tool}
        {@const look = lookOf("tool", pastilles.config.tool, pastilles.catalog)}
        {@const stance = stanceOf(toolView?.tone ?? "idle")}
        <p class="state">
          <Mochi palette={look.palette} kind={look.kind} move={stance.move} face={stance.face} scale={1.25} shadow={false} label="" />
          <span>{toolLine(toolName, toolView?.tool, project, agentsIn(project, sessions.list))}</span>
        </p>
      {:else}
        <p class="hint">{t.tab.toolHint}</p>
      {/if}
    </div>
  </section>

  <section aria-labelledby="extras-title">
    <h3 class="label" id="extras-title">{t.tab.extras} <span class="count">{t.tab.count(extras.length, MAX_EXTRAS)}</span></h3>
    {#if extras.length > 0}
      <ul class="box rows">
        {#each extras as extra (extra.id)}
          {@const look = lookOf(extra.id, pastilles.config.tool, pastilles.catalog)}
          {@const view = pastilles.view(extra.id)}
          {@const stance = stanceOf(view?.tone ?? "idle")}
          <li>
            <Mochi palette={look.palette} kind={look.kind} move={stance.move} face={stance.face} scale={1.25} shadow={false} label="" />
            <span class="text">
              <b>{look.name}</b>
              <small class:problem={view?.problem !== undefined}>{lineOf(extra.id)}</small>
            </span>
            {#if view?.problem}
              <button type="button" class="btn sm" onclick={() => pastilles.retry(extra.id)}>{t.retry}</button>
            {/if}
            <button type="button" class="btn sm" onclick={() => pastilles.remove(extra.id)} aria-label={t.tab.removeName(look.name)}>{t.tab.remove}</button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="box empty">{t.tab.noExtra(MAX_EXTRAS)}</p>
    {/if}
  </section>

  <section aria-labelledby="add-title">
    <h3 class="label" id="add-title">{t.add}</h3>
    {#if full}<p class="hint">{t.tab.full}</p>{/if}
    <div class="catalog">
      {#each LOCALS as local (local.id)}
        <button type="button" class="choice" disabled={added(local.id) || full} onclick={() => pastilles.addLocal(local.id)}>
          <Mochi palette={local.palette} kind={local.kind} move="breathe" scale={1.25} shadow={false} label="" />
          <span class="text"><b>{local.name}</b><small>{added(local.id) ? t.tab.added : t.tab.local}</small></span>
        </button>
      {/each}
      {#each pastilles.catalog as service (service.id)}
        <button
          type="button"
          class="choice"
          aria-expanded={adding?.id === service.id}
          disabled={added(service.id) || full}
          onclick={() => choose(service)}
        >
          <Mochi palette={service.palette} kind={service.kind} move="breathe" scale={1.25} shadow={false} label="" />
          <span class="text"><b>{service.name}</b><small>{added(service.id) ? t.tab.added : service.warning ? t.tab.fullRights : t.tab.readToken}</small></span>
        </button>
      {/each}
    </div>
    {#if adding}
      {#key adding.id}
        <ServiceForm service={adding} ondone={() => (adding = null)} />
      {/key}
    {/if}
  </section>

  <p class="foot">{t.tab.foot}</p>
  {#if pastilles.notice}<p class="error" role="alert">{pastilles.notice}</p>{/if}
  {#if settings.error}<p class="error" role="alert">{settings.error}</p>{/if}
</div>

<style>
  .pastilles {
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }

  .intro,
  .hint,
  .foot {
    color: var(--ink-2);
    font-size: var(--fs-3);
    line-height: 1.45;
  }

  .foot {
    color: var(--ink-3);
    font-size: var(--fs-2);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .label {
    color: var(--ink-3);
    font-size: var(--fs-1);
    font-weight: 500;
    letter-spacing: var(--track-label);
    text-transform: uppercase;
  }

  .count {
    margin-left: var(--s1);
    color: var(--ink-2);
    letter-spacing: 0;
    text-transform: none;
    font-variant-numeric: tabular-nums;
  }

  .box {
    border: 1px solid var(--line);
    border-radius: var(--r-card);
  }

  .tool {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
  }

  .state {
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--fs-3);
  }

  .rows {
    list-style: none;
  }

  .rows li {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s2) var(--s4) var(--s2) var(--s3);
  }

  .rows li + li {
    border-top: 1px solid var(--line);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .text b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  .text small {
    color: var(--ink-2);
    font-size: var(--fs-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text small.problem {
    color: var(--err);
  }

  .empty {
    padding: var(--s4);
    border-style: dashed;
    border-color: var(--line-2);
    color: var(--ink-3);
    font-size: var(--fs-3);
  }

  .catalog {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
    gap: var(--s2);
  }

  .choice {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
    padding: var(--s2) var(--s3) var(--s2) var(--s2);
    border-radius: var(--r-ctl);
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
    text-align: left;
    transition:
      background var(--t-quick),
      box-shadow var(--t-quick),
      transform var(--t-quick) var(--ease-out);
  }

  .choice:hover:enabled {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }

  .choice:active:enabled {
    transform: scale(var(--press));
  }

  .choice[aria-expanded="true"] {
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }

  .choice:disabled {
    cursor: default;
    opacity: var(--dim);
  }

  .choice .text b {
    font-size: var(--fs-3);
  }

  .error {
    color: var(--err);
    font-size: var(--fs-3);
  }

  @media (prefers-reduced-motion: reduce) {
    .choice {
      transition: none;
    }

    .choice:active:enabled {
      transform: none;
    }
  }
</style>
