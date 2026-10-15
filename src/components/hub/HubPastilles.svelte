<!-- The hub's « Pastilles » view (prompt 12, lot 4), laid out as its home: the chosen
     pastille in full on the left, every pastille as a small Mochi on the right. Shown only
     while at least one pastille is on (HubBar). -->
<script lang="ts">
  import { pastilles } from "../../lib/stores/pastilles.svelte";
  import PastilleCard from "../pastilles/PastilleCard.svelte";
  import PastilleTiles from "../pastilles/PastilleTiles.svelte";

  let { onopenchat }: { onopenchat?: (model: string, provider: "ollama" | "lmstudio") => void } = $props();
</script>

<div class="pastilles">
  {#if pastilles.shown}
    <PastilleCard id={pastilles.shown} {onopenchat} />
  {/if}
  <PastilleTiles />
</div>

<style>
  .pastilles {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: var(--s3);
    height: 100%;
    min-height: 0;
  }
</style>
