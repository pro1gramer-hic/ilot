<!-- Line icons of the panel (16 px grid, drawn as in reference/ilot-demo-finale.html):
     one per tab (Son and Général added by prompt 12), and the cross that closes.
     Decorative: a word always goes with them. -->
<script lang="ts" module>
  const PATHS = {
    today: "M8 2.5v1.5M8 12v1.5M2.5 8H4M12 8h1.5M4.1 4.1l1 1M10.9 10.9l1 1M4.1 11.9l1-1M10.9 5.1l1-1M10.6 8a2.6 2.6 0 1 1-5.2 0 2.6 2.6 0 0 1 5.2 0z",
    agents: "M5.5 3h5A2.5 2.5 0 0 1 13 5.5v5a2.5 2.5 0 0 1-2.5 2.5h-5A2.5 2.5 0 0 1 3 10.5v-5A2.5 2.5 0 0 1 5.5 3zM6 6.5v3M10 6.5v3M1.5 6v4M14.5 6v4",
    messages: "M2.5 4.5a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2H7l-3 2.5v-2.5a2 2 0 0 1-1.5-2z",
    music: "M6 12V3.5l7-1.5v8.5M6.3 12a1.8 1.8 0 1 1-3.6 0 1.8 1.8 0 0 1 3.6 0zM13.3 10.5a1.8 1.8 0 1 1-3.6 0 1.8 1.8 0 0 1 3.6 0z",
    chat: "M7.5 2.5c.4 2.7 2 4.3 4.7 4.7-2.7.4-4.3 2-4.7 4.7-.4-2.7-2-4.3-4.7-4.7 2.7-.4 4.3-2 4.7-4.7zM12.5 10.8v2.6M11.2 12.1h2.6",
    mochi: "M3 9.5a5 4.5 0 0 1 10 0v2a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 3 11.5zM8 5V3.2l1.4-.8",
    themes:
      "M8 2.5a5.5 5.5 0 1 0 0 11c.9 0 1.2-.6 1-1.3-.3-.9.2-1.7 1.2-1.7h1.3a2 2 0 0 0 2-2A5.8 5.8 0 0 0 8 2.5zM5.9 7a.7.7 0 1 1-1.4 0 .7.7 0 0 1 1.4 0zM8.3 5a.7.7 0 1 1-1.4 0 .7.7 0 0 1 1.4 0zM11.1 5.8a.7.7 0 1 1-1.4 0 .7.7 0 0 1 1.4 0z",
    modes: "M3 4.5h6M12 4.5h1M3 11.5h1M7 11.5h6M12 4.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0zM7 11.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0z",
    sound: "M2.8 6.2h2.3L8.4 3.4v9.2L5.1 9.8H2.8zM10.9 6a2.7 2.7 0 0 1 0 4M12.7 4.1a5.4 5.4 0 0 1 0 7.8",
    general: "M2.5 3h11a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM6 14h4M8 11.5V14",
    shortcuts: "M2.5 4.5h11a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1zM4.5 7h.01M7 7h.01M9.5 7h.01M12 7h.01M5.5 9.5h5",
    mic: "M8 2.5a2 2 0 0 1 2 2v3a2 2 0 0 1-4 0v-3a2 2 0 0 1 2-2zM4.5 7.5a3.5 3.5 0 0 0 7 0M8 11v2.5",
    back: "M9.5 3.5 5 8l4.5 4.5",
    save: "M8 2.5v8M4.8 7.3 8 10.5l3.2-3.2M3 13.5h10",
    pastilles: "M5 4.5h6a3.5 3.5 0 0 1 0 7H5a3.5 3.5 0 0 1 0-7zM5.3 6.6a1.4 1.4 0 1 1 0 2.8 1.4 1.4 0 0 1 0-2.8zM8.8 8h2.4",
    close: "M4 4l8 8M12 4l-8 8",
    /** « Traduire le presse-papiers » (DeepL, prompt 12). */
    clipboard: "M5.5 3h-1a1 1 0 0 0-1 1v9a1 1 0 0 0 1 1h7a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1h-1M6 2h4a.5.5 0 0 1 .5.5v1a.5.5 0 0 1-.5.5H6a.5.5 0 0 1-.5-.5v-1A.5.5 0 0 1 6 2zM5.5 8h5M5.5 10.5h3.5",
  } as const;

  export type PanelIconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name }: { name: PanelIconName } = $props();
</script>

<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d={PATHS[name]} /></svg>

<style>
  svg {
    display: block;
    flex: none;
    width: var(--s4);
    height: var(--s4);
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
