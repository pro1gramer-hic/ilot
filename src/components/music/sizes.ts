// Sizes the island gives the music openings and the bubble (docs/09-DEMO-FINALE.md,
// sections 3 and 4), in pixels: the island draws its shape at these sizes, the components
// lay their content inside.

import type { Modes } from "../../lib/modes";

export type MusicMode = Modes["music"];

/** The drop stuck to the folded island, per « Musique » mode (0 × 0: no bubble). */
export const BUBBLE_SIZES: Record<MusicMode, { width: number; height: number }> = {
  titre: { width: 190, height: 40 },
  disque: { width: 40, height: 40 },
  egaliseur: { width: 40, height: 40 },
  aucun: { width: 0, height: 0 },
};

/** « Musique » opening: cover, title, progress and commands. */
export const OPENING_SIZE = { width: 500, height: 96 } as const;

/** « Nouveau morceau » opening. */
export const NEW_TRACK_SIZE = { width: 420, height: 56 } as const;
