<!-- One line of code of the preview's staged session, colored by its tokens (keyword,
     string, comment). The app itself never shows code: îlot does not read the files. -->
<script lang="ts">
  import type { CodeToken } from "../../lib/hub";

  let { tokens }: { tokens: readonly CodeToken[] } = $props();
</script>

{#each tokens as [kind, text], index (index)}<span class={kind || undefined}>{text}</span>{/each}

<style>
  .k {
    color: var(--accent-text);
  }

  .s {
    color: var(--ok);
  }

  .c {
    color: var(--ink-3);
  }
</style>
