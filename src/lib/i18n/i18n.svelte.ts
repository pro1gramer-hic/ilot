// The interface's texts in the language of the moment (docs/JOURNAL.md, prompt 12 lot 5).
// Components read `i18n.t.<namespace>.<key>`: plain texts, or functions for texts that
// take a number or a name (plurals differ from one language to the other). The French
// catalog is the reference; the English one must have exactly its shape (checked by the
// compiler). To add a language: a folder like `en/`, its entry in CATALOGS and LOCALES.
//
// It depends on nothing but the catalogs: the settings store gives it the language chosen
// (`language`), so that ipc.ts, the settings and the logic modules can all read texts
// without loading each other in a circle.

import type { Language } from "../ipc";
import { en } from "./en";
import { type Catalog, fr } from "./fr";
import { type Locale, previewLocale, resolveLocale } from "./locale";

const CATALOGS: Readonly<Record<Locale, Catalog>> = { fr, en };

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Outside the app (browser preview, tests), French unless the address asks for another,
// until a language is chosen in Réglages › Général.
const forced: Locale | null = inTauri ? null : ((typeof location === "undefined" ? null : previewLocale(location.search)) ?? "fr");

class I18n {
  /** The language chosen in Réglages › Général (stores/settings.svelte.ts keeps it). */
  language = $state<Language>("auto");
  locale: Locale = $derived(
    forced !== null && this.language === "auto"
      ? forced
      : resolveLocale(this.language, typeof navigator === "undefined" ? [] : navigator.languages),
  );
  t: Catalog = $derived(CATALOGS[this.locale]);
}

export const i18n = new I18n();
