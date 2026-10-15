import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, PREVIEW_TAKEN_SHORTCUTS, type Settings } from "./ipc";
import {
  ACTIONS,
  assignProblem,
  assignShortcut,
  canonical,
  fromKeyboard,
  keyCaps,
  parseShortcut,
  shortcutOf,
  shortcutProblem,
  usedBy,
} from "./shortcuts";

const press = (key: string, code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  key,
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

function withShortcuts(actions: Settings["shortcuts"]["actions"]): Settings {
  return { ...DEFAULT_SETTINGS, shortcuts: { actions } };
}

describe("reading shortcuts", () => {
  it("reads the core's spellings as the same shortcut", () => {
    expect(canonical("Ctrl+Alt+C")).toBe("Ctrl+Alt+C");
    expect(canonical("control+alt+KeyC")).toBe("Ctrl+Alt+C");
    expect(canonical("Alt+Ctrl+c")).toBe("Ctrl+Alt+C");
    expect(canonical("Ctrl+Alt+ArrowUp")).toBe("Ctrl+Alt+Up");
    expect(parseShortcut("Ctrl+Banane")).toBeNull();
    expect(parseShortcut("Ctrl++")).toBeNull();
  });

  it("applies the core's rules", () => {
    expect(shortcutProblem("Ctrl+Alt+Space")).toBeNull();
    expect(shortcutProblem("Shift+C")).toMatch(/^Ajoute Ctrl ou Alt/);
    expect(shortcutProblem("Super+Alt+C")).toBe("La touche Windows est réservée à Windows.");
    expect(shortcutProblem("Ctrl+F12")).toBe("F12 est réservée par Windows au débogage.");
    expect(shortcutProblem("Ctrl+Banane")).toBe("Raccourci invalide : Ctrl+Banane");
  });

  it("shows keys in French, one cap each", () => {
    expect(keyCaps("Ctrl+Alt+Space")).toEqual(["Ctrl", "Alt", "Espace"]);
    expect(keyCaps("Ctrl+Shift+Up")).toEqual(["Ctrl", "Maj", "Haut"]);
  });
});

describe("recording a key press", () => {
  it("waits while only modifiers are held", () => {
    expect(fromKeyboard(press("Control", "ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(fromKeyboard(press("Alt", "AltLeft", { ctrlKey: true, altKey: true }))).toBeNull();
  });

  it("follows the layout for letters and the top row for digits (AZERTY)", () => {
    // On AZERTY, the key that reads A sits where QWERTY has Q.
    expect(fromKeyboard(press("a", "KeyQ", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+A");
    // Top row without Shift: « é », the digit 2.
    expect(fromKeyboard(press("é", "Digit2", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+2");
    expect(fromKeyboard(press("F5", "F5", { altKey: true }))).toBe("Alt+F5");
    expect(fromKeyboard(press(" ", "Space", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+Space");
    expect(fromKeyboard(press("w", "KeyW", { metaKey: true, altKey: true }))).toBe("Alt+Super+W");
  });
});

describe("conflicts and duplicates", () => {
  it("finds the action that already holds a shortcut, the island included", () => {
    const settings = withShortcuts({ "go-to-alert": "Ctrl+Alt+A" });
    expect(usedBy(settings, "control+alt+KeyA", "mute")).toBe("go-to-alert");
    expect(usedBy(settings, "Ctrl+Alt+Space", "mute")).toBe("toggle-island");
    expect(usedBy(settings, "Ctrl+Alt+A", "go-to-alert")).toBeNull();
    expect(assignProblem(settings, "mute", "Ctrl+Alt+A")).toBe("Déjà pris par « Aller à l'alerte ».");
    expect(assignProblem(settings, "mute", "Ctrl+Alt+M")).toBeNull();
  });

  it("never leaves the island without a shortcut", () => {
    expect(assignProblem(DEFAULT_SETTINGS, "toggle-island", "")).toMatch(/^L'île garde toujours/);
    expect(assignProblem(DEFAULT_SETTINGS, "mute", "")).toBeNull();
  });

  it("keeps the old shortcut when Windows or another app holds the new one", () => {
    const before = withShortcuts({ "go-to-alert": "Ctrl+Alt+A" });
    expect(() => assignShortcut(before, "go-to-alert", PREVIEW_TAKEN_SHORTCUTS[0], PREVIEW_TAKEN_SHORTCUTS)).toThrow(
      "Ctrl+Shift+Escape est déjà pris par Windows ou une autre app. L'ancien raccourci reste.",
    );
    expect(shortcutOf(before, "go-to-alert")).toBe("Ctrl+Alt+A");
    const after = assignShortcut(before, "go-to-alert", "Ctrl+Alt+G", PREVIEW_TAKEN_SHORTCUTS);
    expect(shortcutOf(after, "go-to-alert")).toBe("Ctrl+Alt+G");
    const cleared = assignShortcut(after, "go-to-alert", "", PREVIEW_TAKEN_SHORTCUTS);
    expect(shortcutOf(cleared, "go-to-alert")).toBeNull();
    const island = assignShortcut(after, "toggle-island", "Ctrl+Alt+I", PREVIEW_TAKEN_SHORTCUTS);
    expect(island.shortcut).toBe("Ctrl+Alt+I");
  });

  it("lists every action of the prompt, the island first", () => {
    expect(ACTIONS.map((a) => a.id)).toEqual([
      "toggle-island",
      "go-to-alert",
      "open-chat",
      "go-to-terminal",
      "next-pastille",
      "mute",
      "mochi-to-desktop",
      "open-wardrobe",
      "translate-clipboard",
    ]);
  });
});
