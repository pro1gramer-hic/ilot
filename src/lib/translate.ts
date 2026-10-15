// DeepL in îlot (prompt 12): the languages it offers, the language a text goes into, and
// when a text may be translated at all. The request leaves from the Rust core
// (src-tauri/src/translate.rs) with the key kept in the vault; nothing here sends anything.
// https://developers.deepl.com/docs/getting-started/supported-languages

import { texts } from "./i18n/live.svelte";
import type { Locale } from "./i18n/locale";
import type { TranslateFailure } from "./ipc";

/** The vault entry of the user's DeepL key (never read back by the interface). */
export const DEEPL_SECRET = "service:deepl";

/** Where the user finds or creates a key; îlot never opens it by itself (« Copier le lien »).
 * https://developers.deepl.com/docs/getting-started/quickstart */
export const DEEPL_KEY_PAGE = "https://www.deepl.com/your-account/keys";

/** The longest text the core sends (MAX_TEXT_CHARS in translate.rs). */
export const MAX_TRANSLATE_CHARS = 30_000;

/** Every target language DeepL lists, in its order (TARGETS in translate.rs). */
export const DEEPL_TARGETS: readonly string[] = [
  "ACE", "AF", "SQ", "AR", "AN", "HY", "AS", "AY", "AZ", "BA", "EU", "BE", "BN", "BHO", "BS", "BR", "BG", "MY", "YUE",
  "CA", "CEB", "ZH-HANS", "ZH-HANT", "ZH", "HR", "CS", "DA", "PRS", "NL", "EN", "EN-US", "EN-GB", "EO", "ET", "FI",
  "FR", "FR-CA", "FR-FR", "GL", "KA", "DE", "DE-DE", "DE-CH", "EL", "GN", "GU", "HT", "HA", "HE", "HI", "HU", "IS",
  "IG", "ID", "GA", "IT", "JA", "JV", "PAM", "KK", "GOM", "KO", "KMR", "CKB", "KY", "LA", "LV", "LN", "LT", "LMO",
  "LB", "MK", "MAI", "MG", "MS", "ML", "MT", "MI", "MR", "MN", "NE", "NB", "OC", "OM", "PAG", "PS", "FA", "PL",
  "PT-BR", "PT-PT", "PT", "PA", "QU", "RO", "RU", "SA", "SR", "ST", "SCN", "SK", "SL", "ES", "ES-ES", "ES-419", "SU",
  "SW", "SV", "TL", "TG", "TA", "TT", "TE", "TH", "TS", "TN", "TR", "TK", "UK", "UR", "UZ", "VI", "CY", "WO", "XH",
  "YI", "ZU",
];

/** The language a text goes into when none is chosen: the interface's. */
const INTERFACE_TARGET: Readonly<Record<Locale, string>> = { fr: "FR", en: "EN-US" };

export function isTarget(code: string): boolean {
  return DEEPL_TARGETS.includes(code);
}

/** The chosen language (Réglages › Chat), or the interface's when none is (or an odd one). */
export function targetOf(chosen: string, locale: Locale): string {
  return isTarget(chosen) ? chosen : INTERFACE_TARGET[locale];
}

/** DeepL's code as a language tag Intl knows: « ZH-HANS » → « zh-Hans », « ES-419 » → « es-419 ». */
export function languageTag(code: string): string {
  const [base, rest] = code.split("-");
  if (!rest) return base.toLowerCase();
  return `${base.toLowerCase()}-${rest.length === 4 ? rest.charAt(0) + rest.slice(1).toLowerCase() : rest}`;
}

/** The language's name in the interface's language (« anglais », « English »), its code when
 * Intl has no name for it. */
export function languageName(code: string, locale: Locale): string {
  try {
    const name = new Intl.DisplayNames([locale], { type: "language", fallback: "none" }).of(languageTag(code));
    return name ?? code;
  } catch {
    return code;
  }
}

/** The choices of Réglages › Chat: every language, named and sorted in the interface's. */
export function targetChoices(locale: Locale): { code: string; name: string }[] {
  const capitalize = (text: string): string => text.charAt(0).toLocaleUpperCase(locale) + text.slice(1);
  const collator = new Intl.Collator(locale);
  return DEEPL_TARGETS.map((code) => ({ code, name: capitalize(languageName(code, locale)) })).sort((a, b) =>
    collator.compare(a.name, b.name),
  );
}

/** A text worth a « Traduire » button: some letters, not hidden, not too long. A masked
 * message (« Masquer le contenu », « Expéditeur seul ») is never translated. */
export function translatable(text: string | null | undefined, hidden = false): boolean {
  if (hidden || !text) return false;
  const trimmed = text.trim();
  return /\p{L}/u.test(trimmed) && trimmed.length <= MAX_TRANSLATE_CHARS;
}

const FAILURES: readonly TranslateFailure[] = [
  "noKey",
  "keyRefused",
  "quota",
  "busy",
  "unavailable",
  "network",
  "tooLong",
  "empty",
  "badLanguage",
  "unexpected",
];

/** The core's refusal as a code (anything unknown is « unexpected »). */
export function failureOf(error: unknown): TranslateFailure {
  return FAILURES.find((code) => code === error) ?? "unexpected";
}

/** What to tell the user, in the language of the moment. */
export function failureText(failure: TranslateFailure): string {
  return texts().translate.failures[failure];
}
