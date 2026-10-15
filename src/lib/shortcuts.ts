// Global shortcuts (prompt 12, lot 5): the actions a shortcut can start, how a key press
// becomes a shortcut ("Ctrl+Alt+C"), how a shortcut reads (« Ctrl Alt C »), and the rules
// the core applies again before asking Windows (src-tauri/src/shortcuts.rs): the tab says
// what is wrong without waiting for it.

import type { Catalog } from "./i18n/fr";
import { texts } from "./i18n/live.svelte";
import type { Settings } from "./ipc";

export type ActionId =
  | "toggle-island"
  | "open-chat"
  | "go-to-alert"
  | "go-to-terminal"
  | "next-pastille"
  | "mute"
  | "mochi-to-desktop"
  | "open-wardrobe"
  | "translate-clipboard";

/** The island's shortcut lives in `Settings.shortcut`; it can change, never go. */
export const ISLAND_ACTION: ActionId = "toggle-island";

export type Action = { id: ActionId; label: string; hint: string };

/** An action, its name and what it does in the language of the moment. */
function action(id: ActionId, key: keyof Catalog["common"]["actions"]): Action {
  return {
    id,
    get label() {
      return texts().common.actions[key].label;
    },
    get hint() {
      return texts().common.actions[key].hint;
    },
  };
}

/** Every action, in the tab's order. */
export const ACTIONS: readonly Action[] = [
  action("toggle-island", "toggleIsland"),
  action("go-to-alert", "goToAlert"),
  action("open-chat", "openChat"),
  action("go-to-terminal", "goToTerminal"),
  action("next-pastille", "nextPastille"),
  action("mute", "mute"),
  action("mochi-to-desktop", "mochiToDesktop"),
  action("open-wardrobe", "openWardrobe"),
  action("translate-clipboard", "translateClipboard"),
];

export function actionLabel(id: ActionId): string {
  return ACTIONS.find((a) => a.id === id)?.label ?? id;
}

const MODIFIERS = ["Ctrl", "Alt", "Shift", "Super"] as const;
type Modifier = (typeof MODIFIERS)[number];

const MODIFIER_NAMES: Record<string, Modifier> = {
  CTRL: "Ctrl",
  CONTROL: "Ctrl",
  ALT: "Alt",
  OPTION: "Alt",
  SHIFT: "Shift",
  SUPER: "Super",
  CMD: "Super",
  COMMAND: "Super",
};

/** Keys a shortcut may end with, as the core reads them (global-hotkey's names). */
const NAMED_KEYS = [
  "Space",
  "Escape",
  "Enter",
  "Tab",
  "Backspace",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "Up",
  "Down",
  "Left",
  "Right",
  "Comma",
  "Period",
  "Slash",
  "Semicolon",
  "Quote",
  "BracketLeft",
  "BracketRight",
  "Backslash",
  "Minus",
  "Equal",
  "Backquote",
] as const;

/** The key's canonical name, or null when the core would not know it. */
function keyName(key: string): string | null {
  const upper = key.toUpperCase();
  if (/^[A-Z]$/.test(upper) || /^[0-9]$/.test(upper)) return upper;
  if (/^KEY[A-Z]$/.test(upper)) return upper.slice(3);
  if (/^DIGIT[0-9]$/.test(upper)) return upper.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(upper)) return upper;
  if (/^(NUMPAD|NUM)[0-9]$/.test(upper)) return `Numpad${upper.at(-1)}`;
  const arrows: Record<string, string> = { ARROWUP: "Up", ARROWDOWN: "Down", ARROWLEFT: "Left", ARROWRIGHT: "Right" };
  if (arrows[upper]) return arrows[upper];
  return NAMED_KEYS.find((k) => k.toUpperCase() === upper) ?? null;
}

export type Parsed = { mods: Modifier[]; key: string };

/** Reads "Ctrl+Alt+C" (the core's own spellings too: "control+alt+KeyC"). */
export function parseShortcut(text: string): Parsed | null {
  const parts = text.split("+").map((p) => p.trim());
  if (parts.some((p) => p === "")) return null;
  const last = parts.pop();
  const key = last === undefined ? null : keyName(last);
  if (!key) return null;
  const mods: Modifier[] = [];
  for (const part of parts) {
    const mod = MODIFIER_NAMES[part.toUpperCase()];
    if (!mod) return null;
    if (!mods.includes(mod)) mods.push(mod);
  }
  return { mods: MODIFIERS.filter((m) => mods.includes(m)), key };
}

/** One spelling per shortcut, to compare them. */
export function canonical(text: string): string | null {
  const parsed = parseShortcut(text);
  return parsed ? [...parsed.mods, parsed.key].join("+") : null;
}

/** Why the core would refuse this shortcut, or null when it follows the rules. */
export function shortcutProblem(text: string): string | null {
  const parsed = parseShortcut(text);
  const said = texts().common.shortcuts;
  if (!parsed) return said.invalid(text.trim());
  if (parsed.mods.includes("Super")) return said.windowsKey;
  if (parsed.key === "F12") return said.f12;
  if (!parsed.mods.includes("Ctrl") && !parsed.mods.includes("Alt")) return said.needsCtrlOrAlt;
  return null;
}

/** Every shortcut of the settings with its action, the island's first. */
export function assigned(settings: Pick<Settings, "shortcut" | "shortcuts">): [ActionId, string][] {
  const actions = Object.entries(settings.shortcuts.actions).filter(
    (entry): entry is [ActionId, string] => typeof entry[1] === "string" && ACTIONS.some((a) => a.id === entry[0]),
  );
  return [[ISLAND_ACTION, settings.shortcut], ...actions];
}

export function shortcutOf(settings: Pick<Settings, "shortcut" | "shortcuts">, id: ActionId): string | null {
  return assigned(settings).find(([action]) => action === id)?.[1] ?? null;
}

/** The other action that already has this shortcut, if any. */
export function usedBy(settings: Pick<Settings, "shortcut" | "shortcuts">, text: string, except: ActionId): ActionId | null {
  const wanted = canonical(text);
  if (!wanted) return null;
  return assigned(settings).find(([action, s]) => action !== except && canonical(s) === wanted)?.[0] ?? null;
}

/** What to tell the user before asking Windows, or null when the shortcut can be tried. */
export function assignProblem(settings: Pick<Settings, "shortcut" | "shortcuts">, id: ActionId, text: string): string | null {
  if (text.trim() === "") return id === ISLAND_ACTION ? texts().common.shortcuts.islandKeepsOne : null;
  const problem = shortcutProblem(text);
  if (problem) return problem;
  const other = usedBy(settings, text, id);
  return other ? texts().common.shortcuts.takenBy(actionLabel(other)) : null;
}

/** The core's answer, for the browser preview: same rules, and `takenElsewhere` stands for
 * the shortcuts Windows or another app holds. Throws the core's message. */
export function assignShortcut(current: Settings, id: ActionId, text: string, takenElsewhere: readonly string[]): Settings {
  const problem = assignProblem(current, id, text);
  if (problem) throw new Error(problem);
  const clean = text.trim();
  if (takenElsewhere.some((s) => canonical(s) === canonical(clean))) {
    throw new Error(texts().common.shortcuts.takenElsewhere(clean));
  }
  if (id === ISLAND_ACTION) return { ...current, shortcut: clean };
  const actions = { ...current.shortcuts.actions };
  if (clean === "") delete actions[id];
  else actions[id] = clean;
  return { ...current, shortcuts: { actions } };
}

/** The shortcut a key press makes, or null while only modifiers are held. Letters follow
 * the keyboard's layout (the key that reads A gives A, on AZERTY too); the top row gives
 * its digit. */
export function fromKeyboard(event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">): string | null {
  if (["Control", "Alt", "Shift", "Meta", "AltGraph", "OS"].includes(event.key)) return null;
  const letter = /^[a-z]$/i.test(event.key) ? event.key.toUpperCase() : null;
  const key = letter ?? keyName(event.code);
  if (!key) return null;
  const mods = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super"].filter(
    (m): m is Modifier => typeof m === "string",
  );
  return [...mods, key].join("+");
}

/** Keys whose cap the language names (the core's name → the catalog's entry). */
const SPOKEN_KEYS: Readonly<Record<string, keyof Catalog["common"]["keys"]>> = {
  Shift: "shift",
  Super: "super",
  Space: "space",
  Escape: "escape",
  Enter: "enter",
  Backspace: "backspace",
  Delete: "delete",
  Insert: "insert",
  Home: "home",
  End: "end",
  PageUp: "pageUp",
  PageDown: "pageDown",
  Up: "up",
  Down: "down",
  Left: "left",
  Right: "right",
};

/** The keys to show, one per key cap: « Ctrl », « Alt », « Espace ». */
export function keyCaps(text: string): string[] {
  const parsed = parseShortcut(text);
  if (!parsed) return [text];
  const keys = texts().common.keys;
  return [...parsed.mods, parsed.key].map((k) => (k in SPOKEN_KEYS ? keys[SPOKEN_KEYS[k]] : k));
}
