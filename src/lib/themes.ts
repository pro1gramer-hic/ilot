// The seven themes of the final demo (docs/09-DEMO-FINALE.md, section 1). Each one only
// rewrites the design tokens of src/lib/tokens.css (`:root[data-theme=…]`), never adds a
// decorative class. The preview colors draw the theme's card in the Thèmes tab; its name
// and description follow the language (i18n/fr/themes.ts).

import type { Catalog } from "./i18n/fr";
import { texts } from "./i18n/live.svelte";

/** A theme: its id, its name and description in the language of the moment, its colors. */
function theme<I extends keyof Catalog["themes"]>(id: I, preview: readonly [string, string, string, string]) {
  return {
    id,
    get label(): string {
      return texts().themes[id].label;
    },
    get description(): string {
      return texts().themes[id].description;
    },
    preview,
  } as const;
}

export const THEMES = [
  theme("fusion", ["#08080B", "#0A0A0E", "#7C5CFF", "#F5F5F7"]),
  theme("obsidienne", ["#060607", "#0B0B0C", "#EDEDEF", "#EDEDEF"]),
  theme("neon", ["#07040C", "#0A0610", "#FF2BD6", "#00E5FF"]),
  theme("terminal", ["#030503", "#050805", "#39FF88", "#CFFFE0"]),
  theme("aurore", ["#06070D", "#090A12", "#6E8BFF", "#B36BFF"]),
  theme("bonbon", ["#0A070A", "#0D090D", "#FF6FB5", "#FFD84D"]),
  theme("chaos", ["#050505", "#0A0A0A", "#FFE600", "#FF3D7F"]),
] as const;

export type ThemeId = (typeof THEMES)[number]["id"];

export const DEFAULT_THEME: ThemeId = "fusion";

/** The theme after `id`, for « Changer de thème ». */
export function nextTheme(id: ThemeId): ThemeId {
  const index = THEMES.findIndex((theme) => theme.id === id);
  return THEMES[(index + 1) % THEMES.length].id;
}
