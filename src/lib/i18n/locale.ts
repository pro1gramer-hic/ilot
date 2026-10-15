// The language the interface speaks (prompt 12, lot 5): the one chosen in Réglages ›
// Général, or Windows' own (the webview reports it) when « auto ». Pure, tested.

import type { Language } from "../ipc";

export type Locale = "fr" | "en";

export const LOCALES: readonly Locale[] = ["fr", "en"];

const isLocale = (value: string): value is Locale => (LOCALES as readonly string[]).includes(value);

/** `language` when chosen; with « auto », the first of the system's languages îlot speaks,
 * English when it speaks none of them. */
export function resolveLocale(language: Language, system: readonly string[]): Locale {
  if (language !== "auto") return language;
  for (const tag of system) {
    const base = tag.toLowerCase().split("-")[0];
    if (isLocale(base)) return base;
  }
  return "en";
}

/** A language asked for in the address (`?lang=en`), for the browser preview only. */
export function previewLocale(search: string): Locale | null {
  const asked = new URLSearchParams(search).get("lang");
  return asked !== null && isLocale(asked) ? asked : null;
}
