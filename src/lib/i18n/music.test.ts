import { describe, expect, it } from "vitest";
import { music as en } from "./en/music";
import { music as fr } from "./fr/music";

describe("texts of the player", () => {
  it("name the repeat mode after its label, each language in its own words", () => {
    expect(fr.modes.repeat("Répéter la liste")).toBe("Répétition : répéter la liste");
    expect(fr.modes.repeatTitle("Pas de répétition", "Répéter la liste")).toBe("Pas de répétition. Clic : répéter la liste");
    expect(en.modes.repeat("Repeat playlist")).toBe("Repeat mode: repeat playlist");
    expect(en.modes.repeatTitle("No repeat", "Repeat playlist")).toBe("No repeat. Click: repeat playlist");
  });

  it("write a level as each language writes a percentage", () => {
    expect(fr.volume.level(64)).toBe("64 %");
    expect(en.volume.level(64)).toBe("64%");
  });

  it("put the app's name where each language wants it", () => {
    expect(fr.volume.app("Spotify")).toBe("Volume de Spotify");
    expect(en.volume.app("Spotify")).toBe("Spotify volume");
    expect(fr.bubble.open("Nuit calme, Kairo")).toBe("En lecture : Nuit calme, Kairo. Ouvrir le lecteur");
    expect(en.bubble.open("Nuit calme, Kairo")).toBe("Now playing: Nuit calme, Kairo. Open the player");
  });
});
