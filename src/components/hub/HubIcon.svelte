<!-- Line icons of the hub (16 × 16, 1.5 px strokes, round ends), drawn after the final
     demo's: its bar (home, messages, new session, settings, sound, quiet hours, back), the
     verbs of an agent (read, edit, run), and the marks of a state. Decorative: the
     button or the text next to each one names it. -->
<script lang="ts" module>
  const PATHS = {
    home: "M2.5 7.2 8 2.5l5.5 4.7V13a.5.5 0 0 1-.5.5H9.5V10h-3v3.5H3a.5.5 0 0 1-.5-.5z",
    chat: "M2.5 4.5a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2H7l-3 2.5v-2.5a2 2 0 0 1-1.5-2z",
    plus: "M8 3v10M3 8h10",
    gear: "M8 5.8a2.2 2.2 0 1 1 0 4.4 2.2 2.2 0 0 1 0-4.4zM8 1.8v1.6M8 12.6v1.6M1.8 8h1.6M12.6 8h1.6M3.6 3.6l1.1 1.1M11.3 11.3l1.1 1.1M3.6 12.4l1.1-1.1M11.3 4.7l1.1-1.1",
    moon: "M11 2.6a5.6 5.6 0 1 0 2.9 8.3A4.7 4.7 0 0 1 11 2.6z",
    sound: "M2.8 6.2h2.3L8.4 3.4v9.2L5.1 9.8H2.8zM10.9 6a2.7 2.7 0 0 1 0 4M12.7 4.1a5.4 5.4 0 0 1 0 7.8",
    mute: "M2.8 6.2h2.3L8.4 3.4v9.2L5.1 9.8H2.8zM11 6.3l3.3 3.4M14.3 6.3 11 9.7",
    back: "M9.5 3.5 5 8l4.5 4.5",
    read: "M4 2.5h5l3 3v8H4zM9 2.5v3h3M6 9h4M6 11h3",
    edit: "M10.5 3 13 5.5 6 12.5H3.5V10z",
    term: "M3 4.5 6 8l-3 3.5M8 12h5",
    check: "M3.5 8.3l3 3 6-6.5",
    cross: "M4.5 4.5l7 7M11.5 4.5l-7 7",
    wait: "M8 4.5V8l2.2 1.6M8 1.8a6.2 6.2 0 1 1 0 12.4A6.2 6.2 0 0 1 8 1.8z",
    note: "M6 12.2V3.8l6-1.3v8.2M6 12.2a1.7 1.7 0 1 1-3.4 0 1.7 1.7 0 0 1 3.4 0zM12 10.7a1.7 1.7 0 1 1-3.4 0 1.7 1.7 0 0 1 3.4 0z",
    alert: "M8 3.5v5.5M8 11.6v.4",
    sleep: "M4 4h4L4 9h4M9.5 8.5H12L9.5 12H12",
    // The chat and what goes with it (prompt 12, lot 3).
    spark: "M7.5 2.5c.4 2.7 2 4.3 4.7 4.7-2.7.4-4.3 2-4.7 4.7-.4-2.7-2-4.3-4.7-4.7 2.7-.4 4.3-2 4.7-4.7zM12.5 10.8v2.6M11.2 12.1h2.6",
    send: "M8 13V3.5M4 7.5l4-4 4 4",
    stop: "M5 5h6v6H5z",
    mic: "M8 2.5a2 2 0 0 1 2 2v3.3a2 2 0 0 1-4 0V4.5a2 2 0 0 1 2-2zM4.4 7.6a3.6 3.6 0 0 0 7.2 0M8 11.3v2.2",
    clip: "M10.6 5.2 6.3 9.5a1.3 1.3 0 0 0 1.9 1.9l4.4-4.4a2.7 2.7 0 0 0-3.8-3.8L4.3 7.6a4 4 0 0 0 5.7 5.7l2.8-2.8",
    window: "M2.5 3.5h11v9h-11zM2.5 6h11M4.3 4.8h.01M5.8 4.8h.01",
    mail: "M2.5 4h11v8h-11zM2.8 4.4 8 8.6l5.2-4.2",
    compose: "M7.5 3H4.5a2 2 0 0 0-2 2v6.5a2 2 0 0 0 2 2H11a2 2 0 0 0 2-2V8.5M11.6 2.4l2 2L8.2 9.8H6.2v-2z",
    chevron: "M4.5 6.5 8 10l3.5-3.5",
    /** A pastille: a small pill with its Mochi and its state line. */
    pastilles: "M5 4.5h6a3.5 3.5 0 0 1 0 7H5a3.5 3.5 0 0 1 0-7zM5.3 6.6a1.4 1.4 0 1 1 0 2.8 1.4 1.4 0 0 1 0-2.8zM8.8 8h2.4",
  } as const;

  export type HubIconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 16 }: { name: HubIconName; size?: number } = $props();
</script>

<svg viewBox="0 0 16 16" width={size} height={size} aria-hidden="true" focusable="false">
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    display: block;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
