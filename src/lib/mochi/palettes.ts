// Every color Mochi can paint, ported from `PAL` (reference/ilot-demo-finale.html), the
// `V_*` constants and the inline colors of `drawMochiV()` (reference/mochi-vectoriel.js).
// The strings stay byte-identical to the reference: tests replay both engines call by call.
// The palettes' names follow the language (i18n/fr/mochi.ts).

import { texts } from "../i18n/live.svelte";

/** The six palettes of the reference (the user picks among them), then those of the agents
 * îlot follows for their activity and of the badges `ilot_agent` gives (hub only). */
export type PaletteId =
  | "lavande"
  | "peche"
  | "menthe"
  | "ciel"
  | "citron"
  | "nuit"
  | "ivoire"
  | "lagon"
  | "anis"
  | "fuchsia"
  | "pervenche"
  | "glycine";

export interface Palette {
  /** Display name shown in the settings, in the language of the moment. */
  readonly name: string;
  /** Five shades, from the lit side to the shadow side: gradients, feet, arms, lock, ears. */
  readonly tiers: readonly [string, string, string, string, string];
  /** A darker shade of the same hue: the thin outline and the shading at the bottom. */
  readonly outline: string;
}

export const DEFAULT_PALETTE: PaletteId = "lavande";

export const PALETTES: Readonly<Record<PaletteId, Palette>> = {
  lavande: {
    get name() {
      return texts().mochi.palettes.lavande;
    },
    tiers: ["#f6f5ff", "#cfd1ff", "#a3a8ff", "#7a80f2", "#5c61d2"],
    outline: "#2c2869",
  },
  peche: {
    get name() {
      return texts().mochi.palettes.peche;
    },
    tiers: ["#fff5ee", "#ffd6bf", "#ffb08f", "#f2876a", "#d2664f"],
    outline: "#5e2420",
  },
  menthe: {
    get name() {
      return texts().mochi.palettes.menthe;
    },
    tiers: ["#f0fff8", "#c2f4de", "#86e3bf", "#52c89f", "#35a17f"],
    outline: "#17473c",
  },
  ciel: {
    get name() {
      return texts().mochi.palettes.ciel;
    },
    tiers: ["#f1fbff", "#c3ebff", "#8ad1f8", "#5bafeb", "#3f89cd"],
    outline: "#173a63",
  },
  citron: {
    get name() {
      return texts().mochi.palettes.citron;
    },
    tiers: ["#fffde9", "#fff3ad", "#ffe16b", "#f3c33c", "#d39b20"],
    outline: "#5e400e",
  },
  nuit: {
    get name() {
      return texts().mochi.palettes.nuit;
    },
    tiers: ["#d6d9f5", "#9298cb", "#60669c", "#43487c", "#2e3262"],
    outline: "#0d0f2c",
  },
  // Agents' palettes: the same five steps from light to shadow, in the hue of each
  // agent's token (--ai-cursor, --ai-copilot… in tokens.css).
  ivoire: {
    get name() {
      return texts().mochi.palettes.ivoire;
    },
    tiers: ["#fbfaf6", "#ece8de", "#d6d0c2", "#b3ac9c", "#8f8878"],
    outline: "#3d382e",
  },
  lagon: {
    get name() {
      return texts().mochi.palettes.lagon;
    },
    tiers: ["#effcff", "#c4f1f8", "#8fdeec", "#55c2d6", "#3299b0"],
    outline: "#11404c",
  },
  anis: {
    get name() {
      return texts().mochi.palettes.anis;
    },
    tiers: ["#f8fdea", "#e3f2b8", "#c9e284", "#a6c455", "#7f9c37"],
    outline: "#2f3d10",
  },
  fuchsia: {
    get name() {
      return texts().mochi.palettes.fuchsia;
    },
    tiers: ["#fff0f7", "#ffcfe6", "#f9a3cd", "#e874ad", "#c4508a"],
    outline: "#4f1534",
  },
  pervenche: {
    get name() {
      return texts().mochi.palettes.pervenche;
    },
    tiers: ["#f2f4ff", "#d1d9ff", "#aab9ff", "#8199f2", "#5f74d2"],
    outline: "#1f2a63",
  },
  glycine: {
    get name() {
      return texts().mochi.palettes.glycine;
    },
    tiers: ["#faf2ff", "#e8d2ff", "#d2aefb", "#b585ea", "#9061c9"],
    outline: "#3b1d5e",
  },
};

/** Palette ids in display order (same order as the reference). */
export const PALETTE_IDS: readonly PaletteId[] = ["lavande", "peche", "menthe", "ciel", "citron", "nuit"];

/** Fixed colors shared by every palette: face, light and ground shadow. */
export const INK = {
  eye: "#1b1830",
  /** The white glint in an open eye. */
  glint: "#fff",
  cheek: "rgba(255,128,160,.5)",
  mouth: "#3a2238",
  tongue: "#ff7a9a",
  heart: "#ff4f86",
  star: "#ffd34d",
  sweat: "#9fd4ff",
  earInside: "rgba(255,150,180,.75)",
  shine: "rgba(255,255,255,.78)",
  topLight: "rgba(255,255,255,.35)",
  noLight: "rgba(255,255,255,0)",
  groundShadow: "rgba(0,0,0,.32)",
  noShadow: "rgba(0,0,0,0)",
} as const;

/** The « Écran » silhouette: a dark glossy visor where the eyes light up in cyan. */
export const SCREEN = {
  visor: "#13112a",
  gloss: "rgba(255,255,255,.14)",
  glow: "#8ff5ff",
} as const;

/** Colors of the effects drawn around the body by some moves. */
export const EFFECTS = {
  snore: "#8a87a0",
  snoreFaint: "#b7b4cc",
  confetti: ["#ff6b81", "#ffc400", "#3ddc97", "#5bafeb", "#a3a8ff"],
} as const;

/** `#rrggbb` at opacity `alpha`, as the reference writes it: `rgba(r,g,b,alpha)`. */
export function rgba(hex: string, alpha: number): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`;
}
