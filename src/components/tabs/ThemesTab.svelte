<!-- « Thèmes »: the seven skins of the island, each with a small preview of the pill in its
     own colors. The choice is saved at once (settings.theme) and kept for next time. -->
<script lang="ts">
  import { i18n } from "../../lib/i18n/i18n.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { THEMES, type ThemeId } from "../../lib/themes";

  /** Themes whose preview speaks in their own type. */
  const FONT: Partial<Record<ThemeId, string>> = { terminal: "var(--font-mono)", chaos: "var(--font-pixel)" };
</script>

<div class="themes" role="group" aria-label={i18n.t.settings.themes.label}>
  {#each THEMES as theme (theme.id)}
    {@const [bg, island, accent, second] = theme.preview}
    <button
      type="button"
      class="theme"
      aria-pressed={settings.value.theme === theme.id}
      onclick={() => void settings.update({ theme: theme.id })}
    >
      <span
        class="preview"
        class:arcade={theme.id === "chaos"}
        style:--pv-bg={bg}
        style:--pv-island={island}
        style:--pv-accent={accent}
        style:--pv-second={second}
        style:font-family={FONT[theme.id]}
        aria-hidden="true"
      >
        <span class="pill"><i class="a"></i>{theme.label}<i class="b"></i><i class="c"></i></span>
      </span>
      <b>{theme.label}</b>
      <small>{theme.description}</small>
    </button>
  {/each}
</div>

<style>
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(5 * var(--s8) + var(--s5)), 1fr));
    gap: var(--s3);
  }

  .theme {
    display: flex;
    flex-direction: column;
    padding: var(--s3);
    border-radius: var(--r-card);
    background: var(--surface);
    border: 1px solid var(--line);
    text-align: left;
    transition:
      border-color var(--t-quick) var(--ease-out),
      transform var(--t-quick) var(--ease-out);
  }

  .theme:hover {
    border-color: var(--line-2);
    transform: translateY(var(--lift));
  }

  .theme:active {
    transform: scale(var(--press));
  }

  .theme[aria-pressed="true"] {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .preview {
    display: grid;
    place-items: center;
    height: calc(var(--s8) * 2 + var(--s2));
    margin-bottom: var(--s3);
    border-radius: var(--r-ctl);
    background: var(--pv-bg);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .pill {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: var(--h-ctl);
    padding: 0 var(--s3);
    border-radius: var(--r-pill);
    background: var(--pv-island);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink);
    font-size: var(--fs-1);
    font-weight: 500;
  }

  .arcade .pill {
    border-radius: var(--s1);
    box-shadow: var(--s1) var(--s1) 0 var(--pv-second);
  }

  .pill i {
    width: var(--dot);
    height: var(--dot);
    border-radius: var(--r-pill);
  }

  .a {
    background: var(--pv-accent);
  }

  .b {
    background: var(--pv-second);
  }

  .c {
    background: var(--ink-3);
  }

  b {
    font-size: var(--fs-body);
    font-weight: 600;
  }

  small {
    margin-top: calc(var(--s1) / 2);
    color: var(--ink-2);
    font-size: var(--fs-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .theme:hover,
    .theme:active {
      transform: none;
    }
  }
</style>
