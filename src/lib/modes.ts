// Display modes of the « Modes » tab (docs/09-DEMO-FINALE.md, section 7), kept in the
// settings by the Rust core (`settings.rs`, `Modes`). The first option is the default.
// Their names follow the language (i18n/fr/modes.ts); the ids never change.

import type { Catalog } from "./i18n/fr";
import { texts } from "./i18n/live.svelte";

type Names = Catalog["modes"];

/** An option: its id, and its name in the language of the moment. */
function option<V extends string, K extends keyof Names>(value: V, mode: K, name: Exclude<keyof Names[K], "label">): readonly [V, string] {
  const pair: [V, string] = [value, ""];
  Object.defineProperty(pair, 1, { get: () => texts().modes[mode][name], enumerable: true });
  return pair;
}

/** A mode: its name in the language of the moment, and its options. */
function named<K extends keyof Names, const O extends readonly (readonly [string, string])[]>(key: K, options: O): { readonly label: string; readonly options: O } {
  return {
    get label() {
      return texts().modes[key].label;
    },
    options,
  };
}

export const MODES = {
  music: named("music", [option("titre", "music", "ticker"), option("disque", "music", "disc"), option("egaliseur", "music", "equalizer"), option("aucun", "music", "none")]),
  agents: named("agents", [option("compteur", "agents", "counter"), option("points", "agents", "dots"), option("activite", "agents", "activity")]),
  messages: named("messages", [option("apercu", "messages", "preview"), option("expediteur", "messages", "sender"), option("discret", "messages", "badge")]),
  done: named("done", [option("banniere", "done", "banner"), option("discret", "done", "quiet")]),
  mochi: named("mochi", [option("balade", "mochi", "wanders"), option("suit", "mochi", "follows"), option("reste", "mochi", "stays"), option("bureau", "mochi", "desktop")]),
  relief: named("relief", [option("3d", "relief", "deep"), option("leger", "relief", "light"), option("plat", "relief", "flat")]),
  position: named("position", [option("libre", "position", "free"), option("aimant", "position", "magnet")]),
} as const;

export type ModeKey = keyof typeof MODES;

export type ModeValue<K extends ModeKey> = (typeof MODES)[K]["options"][number][0];

export type Modes = { [K in ModeKey]: ModeValue<K> };

export const DEFAULT_MODES = Object.fromEntries(
  Object.entries(MODES).map(([key, mode]) => [key, mode.options[0][0]]),
) as Modes;
