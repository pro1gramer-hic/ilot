// Mochi's three silhouettes, ported from `MV_KIND` in reference/mochi-vectoriel.js
// (docs/09-DEMO-FINALE.md, section 6). Same engine, moves and faces for all three.

import { liveRecord, texts } from "../i18n/live.svelte";

export type KindId = "meche" | "ecran" | "oreilles";

/** Silhouette ids in display order; « Mèche » is the default. */
export const KIND_IDS: readonly KindId[] = ["meche", "ecran", "oreilles"];

export const DEFAULT_KIND: KindId = "meche";

/** Names of the silhouettes, in the language of the moment. */
export const KIND_LABELS: Readonly<Record<KindId, string>> = liveRecord(KIND_IDS, (id) => texts().mochi.kinds[id]);

/**
 * Body of a silhouette: a superellipse with its own exponent above and below its center
 * (2 is an ellipse, higher is squarer), and its radii in grid units.
 */
export interface BodyShape {
  readonly top: number;
  readonly bottom: number;
  readonly rx: number;
  readonly ry: number;
}

export const BODIES: Readonly<Record<KindId, BodyShape>> = {
  meche: { top: 2.5, bottom: 3.1, rx: 8.4, ry: 6.7 },
  ecran: { top: 3.6, bottom: 3.6, rx: 8.4, ry: 6.8 },
  oreilles: { top: 2.4, bottom: 3.3, rx: 8.4, ry: 6.5 },
};
