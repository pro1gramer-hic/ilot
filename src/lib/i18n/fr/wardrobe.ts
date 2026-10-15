// wardrobe: Mochi's wardrobe, in the island and in Réglages › Mochi
// (src/lib/i18n/i18n.svelte.ts).

export const wardrobe = {
  nothing: "Rien",
  seasonal: (piece: string) => `${piece}, tenue de saison`,
  seasons: "Tenues de saison",
  noSeason: "Aucune en ce moment : bonnet l'hiver, lunettes l'été, chapeau à Halloween.",
  /** « Noël : bonnet de Noël et écharpe. » The pieces' names start lowercase in a sentence. */
  seasonOutfit: (season: string, pieces: readonly string[]) =>
    `${season} : ${pieces.map((piece) => piece.charAt(0).toLocaleLowerCase("fr") + piece.slice(1)).join(" et ")}.`,
};
