<script lang="ts" module>
  const PATHS = {
    play: "M4.5 2.5v11l9-5.5z",
    pause: "M4 2.5h3v11H4zm5 0h3v11H9z",
    check: "M6.2 11.6 2.8 8.2l1.1-1.1 2.3 2.3 5.9-5.9 1.1 1.1z",
    previous: "M3 3h2v10H3zm3 5 7-5v10z",
    next: "M11 3h2v10h-2zM3 3l7 5-7 5z",
    close: "M4 3l9 9-1 1-9-9zM12 3l1 1-9 9-1-1z",
    /** Mini-player: a small screen inside a larger one. */
    pip: "M1 3h14v10H1zm1.5 1.5v7h11v-7zM8 7.5h4.5V11H8z",
    /** Open the app: arrow leaving a box. */
    open: "M9 2h5v5h-1.6V4.7L8.1 9 7 7.9l4.3-4.3H9zM2 4h5v1.6H3.6v6.8h6.8V9H12v5H2z",
    lock: "M8 1.5A3.5 3.5 0 0 0 4.5 5v2h-1v7.5h9V7h-1V5A3.5 3.5 0 0 0 8 1.5zM6 7V5a2 2 0 1 1 4 0v2z",
    /** Wheel gauge: a speaker and one wave. */
    volume: "M2 6h3l4-3.5v11L5 10H2zm9-0.8a4 4 0 0 1 0 5.6l-1-1a2.6 2.6 0 0 0 0-3.6z",
    /** Quick menu. */
    moon: "M10.5 1.8a6.2 6.2 0 1 0 3.7 9.2 5.2 5.2 0 0 1-3.7-9.2z",
    timer: "M6 .8h4v1.4H6zM8 3a6 6 0 1 1 0 12A6 6 0 0 1 8 3zm0 1.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9zM7.3 6h1.4v3.4H7.3z",
    sliders: "M2 3.5h7V5H2zm9.5-1H13V6h-1.5zM13 3.5h1V5h-1zM2 11h2v1.5H2zm3.5-1H7v3.5H5.5zM7 11h7v1.5H7z",
    /** Recentrer l'île: a target. */
    target: "M8 1.5a6.5 6.5 0 1 1 0 13 6.5 6.5 0 0 1 0-13zM8 3a5 5 0 1 0 0 10A5 5 0 0 0 8 3zm0 3a2 2 0 1 1 0 4 2 2 0 0 1 0-4z",
    /** A risky command's warnings. */
    warn: "M8 1.5 15 14H1zM7.3 6v4h1.4V6zm0 5v1.4h1.4V11z",
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 14 }: { name: IconName; size?: number } = $props();
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
