<script lang="ts">
  import Island from "./components/Island.svelte";
  import { i18n } from "./lib/i18n/i18n.svelte";

  // The page speaks the interface's language (screen readers, hyphenation).
  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
</script>

<main>
  <Island />
</main>

<style>
  main {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>
