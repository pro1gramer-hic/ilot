// What Mochi wears now (prompt 12, lot 1): the pieces chosen in the settings, and the
// season's outfit in the free slots unless it is turned off. The day is read again every
// hour, so a new season dresses him without a restart.

import { BARE, type Choice, type Outfit, type PieceId, type SeasonId, type Slot, chosenOutfit, seasonOf, wornOutfit } from "../mochi/outfits";
import { settings } from "./settings.svelte";

const HOUR_MS = 3_600_000;

class WardrobeStore {
  /** Today; the browser preview may set another day to show a season. */
  today = $state(new Date());
  chosen: Choice = $derived(chosenOutfit(settings.value.wardrobe));
  /** The season of the day, whether or not its outfit is worn. */
  season: SeasonId | null = $derived(seasonOf(this.today));
  worn: Outfit = $derived(wornOutfit(this.chosen, settings.value.wardrobe.seasonal, this.today));

  constructor() {
    setInterval(() => (this.today = new Date()), HOUR_MS);
  }

  /** Puts `piece` on in its slot; the piece he wears, or `null`, leaves the slot bare. */
  async choose(slot: Slot, piece: PieceId | null): Promise<void> {
    const bare = piece === null || this.worn[slot] === piece;
    await settings.update({ wardrobe: { ...settings.value.wardrobe, [slot]: bare ? BARE : piece } });
  }

  /** Turning the seasons on gives the season back the slots kept bare. */
  async setSeasonal(on: boolean): Promise<void> {
    const current = settings.value.wardrobe;
    const freed = (value: string): string => (on && value === BARE ? "" : value);
    await settings.update({
      wardrobe: { head: freed(current.head), eyes: freed(current.eyes), neck: freed(current.neck), ears: freed(current.ears), seasonal: on },
    });
  }
}

export const wardrobe = new WardrobeStore();
