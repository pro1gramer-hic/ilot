// Texts that follow the language of the moment, for the logic modules of src/lib and their
// stores (prompt 12, lot 5), read at the time of the call; the components read
// i18n.svelte.ts directly.

import type { Catalog } from "./fr";
import { i18n } from "./i18n.svelte";
import type { Locale } from "./locale";

/** `i18n.t`, read at the time of the call. */
export function texts(): Catalog {
  return i18n.t;
}

/** `i18n.locale`, for Intl. */
export function textsLocale(): Locale {
  return i18n.locale;
}

/** A list of `length` items, each read when asked (`item(index)`): an array to index and
 * to loop over, whose texts follow the language. */
export function liveList<T>(length: number, item: (index: number) => T): readonly T[] {
  const list: T[] = [];
  for (let index = 0; index < length; index += 1) {
    Object.defineProperty(list, index, { get: () => item(index), enumerable: true });
  }
  return list;
}

/** A record of `keys`, each value read when asked (`value(key)`): names by id that follow
 * the language. */
export function liveRecord<K extends string, T>(keys: readonly K[], value: (key: K) => T): Readonly<Record<K, T>> {
  const record = {} as Record<K, T>;
  for (const key of keys) Object.defineProperty(record, key, { get: () => value(key), enumerable: true });
  return record;
}
