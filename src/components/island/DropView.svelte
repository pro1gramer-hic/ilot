<!-- A file dragged over the island (340 × 88): a dashed zone, Mochi opens his mouth. The
     island never reads the file: once dropped, only its path is copied for the agent. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
</script>

<div class="drop">
  <div class="zone">
    <span class="slot" aria-hidden="true"></span>
    <div class="text" role="status">
      <b>{i18n.t.island.drop.title}</b>
      <span>{i18n.t.island.drop.sub}</span>
    </div>
  </div>
</div>

<style>
  .drop {
    height: 88px;
    padding: var(--s3);
  }

  .zone {
    height: 100%;
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: 0 var(--s4);
    border: 1.5px dashed color-mix(in srgb, var(--accent) 60%, transparent);
    border-radius: var(--r-card);
    background: var(--accent-soft);
  }

  .slot {
    width: 36px;
    height: 36px;
    flex: none;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.3;
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  span {
    color: var(--ink-2);
    font-size: var(--fs-sub);
  }
</style>
