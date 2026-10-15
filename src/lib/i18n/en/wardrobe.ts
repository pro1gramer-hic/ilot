// wardrobe, in English: the same keys as ../fr/wardrobe.ts.

import type { Catalog } from "../fr";

export const wardrobe: Catalog["wardrobe"] = {
  nothing: "Nothing",
  seasonal: (piece) => `${piece}, seasonal outfit`,
  seasons: "Seasonal outfits",
  noSeason: "None right now: a beanie in winter, sunglasses in summer, a hat at Halloween.",
  // Piece names keep their capitals: some hold a proper noun.
  seasonOutfit: (season, pieces) => `${season}: ${new Intl.ListFormat("en", { type: "conjunction" }).format(pieces)}.`,
};
