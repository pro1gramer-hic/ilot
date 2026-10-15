// Stand-in for src/lib/i18n/live.svelte.ts outside Svelte: Mochi's French texts only.
import { mochi } from "../../src/lib/i18n/fr/mochi";

export function texts(): { mochi: typeof mochi } {
  return { mochi };
}

export function liveRecord<K extends string, T>(keys: readonly K[], value: (key: K) => T): Readonly<Record<K, T>> {
  const record = {} as Record<K, T>;
  for (const key of keys) record[key] = value(key);
  return record;
}
