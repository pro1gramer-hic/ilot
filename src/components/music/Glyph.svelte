<!-- The two signs the player needs beyond Icon.svelte, in the same 16 px grid and solid
     style: « detach » (a window with an arrow leaving it) and « dock » (an arrow going
     down into the island). Decorative: the button around it carries the name. -->
<script lang="ts" module>
  const PATHS = {
    detach: "M9 1.5h5.5V7h-1.6V4.2L8.6 8.5 7.5 7.4l4.3-4.3H9zM2 5.5A2 2 0 0 1 4 3.5h2.5v1.6H4a.4.4 0 0 0-.4.4V12c0 .2.2.4.4.4h6.5a.4.4 0 0 0 .4-.4V9.5h1.6V12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z",
    dock: "M7.2 1.5h1.6v7.3l2.6-2.6 1.1 1.1L8 11.8 3.5 7.3l1.1-1.1 2.6 2.6zM2.5 13h11v1.6h-11z",
  } as const;

  export type GlyphName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 14 }: { name: GlyphName; size?: number } = $props();
</script>

<svg viewBox="0 0 16 16" width={size} height={size} aria-hidden="true" focusable="false">
  <path d={PATHS[name]} fill-rule="evenodd" />
</svg>

<style>
  svg {
    display: block;
    flex: none;
    fill: currentColor;
  }
</style>
