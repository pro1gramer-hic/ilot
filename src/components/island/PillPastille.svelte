<!-- The pastille the folded pill shows at rest, in place of « En veille » (prompt 12,
     lot 4): a dot in its Mochi's color and a few words (the tool's project, the model in
     memory, « 3 PR »…). Another pastille (`nextPastille()`) rises in its place. The pill
     keeps the agents first: this only shows while none works. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { rise } from "../../lib/motion.svelte";
  import { PALETTES } from "../../lib/mochi/palettes";
  import { lookOf, toolProject } from "../../lib/pastilles";
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import { sessions } from "../../lib/stores/sessions.svelte";

  let { id }: { id: string } = $props();

  const look = $derived(lookOf(id, pastilles.config.tool, pastilles.catalog));
  const view = $derived(pastilles.view(id));
  const words = $derived(
    (id === "tool" ? toolProject(view?.tool, sessions.list) : view?.tone === "off" ? null : view?.short) || look.name,
  );
  const said = $derived(view?.line ? i18n.t.island.pastille.said(look.name, view.line) : look.name);
</script>

{#key `${id}:${words}`}
  <span class="pastille" class:off={view?.tone === "off"} class:alert={view?.tone === "err"} title={said} in:rise={{ shift: 3 }}>
    <i style:--c={PALETTES[look.palette].tiers[2]} aria-hidden="true"></i>
    <span class="words">{words}</span>
    <span class="visually-hidden">, {said}</span>
  </span>
{/key}

<style>
  .pastille {
    display: inline-flex;
    vertical-align: middle;
    align-items: center;
    gap: 6px;
    min-width: 0;
    max-width: 104px;
    color: var(--ink-2);
    font-size: var(--fs-3);
    white-space: nowrap;
  }

  .words {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  i {
    width: var(--dot);
    height: var(--dot);
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }

  /* Off: the dot turns to a ring; a failure gets a red ring around its color. */
  .off i {
    background: none;
    box-shadow: inset 0 0 0 1.5px var(--c);
  }

  .alert i {
    box-shadow: 0 0 0 2px var(--err-soft), 0 0 0 1px var(--err);
  }
</style>
