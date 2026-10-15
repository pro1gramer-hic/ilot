import { beforeEach, describe, expect, it, vi } from "vitest";

// The settings store marks the page's theme: a bare document is enough here.
vi.hoisted(() => {
  if (typeof document === "undefined") Object.assign(globalThis, { document: { documentElement: { dataset: {} } } });
});

// The store talks to the Rust core through ipc.ts and opens Réglages through the island:
// both replaced here, so that every request is seen, and when.
const core = vi.hoisted(() => ({
  calls: [] as [string, ...unknown[]][],
  keys: new Set<string>(),
  clipboard: "Quick update: the release moves to Thursday." as string | null,
  refuse: null as string | null,
  panels: [] as unknown[],
}));

vi.mock("../ipc", async (importOriginal) => {
  const real = await importOriginal<typeof import("../ipc")>();
  return {
    ...real,
    saveSettings: async (settings: unknown) => settings,
    focusIsland: async () => {},
    secretHas: async (name: string) => core.keys.has(name),
    secretStore: async (name: string, secret: string) => {
      core.calls.push(["secret_store", name]);
      core.keys.add(name);
      void secret;
    },
    secretRemove: async (name: string) => {
      core.calls.push(["secret_remove", name]);
      core.keys.delete(name);
    },
    translateText: async (text: string, target: string) => {
      core.calls.push(["translate_text", text, target]);
      if (core.refuse) throw core.refuse;
      return { text: `[${target}] ${text}`, detected: "EN" };
    },
    readClipboardText: async () => {
      core.calls.push(["read_clipboard_text"]);
      if (core.clipboard === null) throw "empty";
      return core.clipboard;
    },
    copyText: async (text: string) => {
      core.calls.push(["copy_text", text]);
    },
  };
});

vi.mock("../island/controller.svelte", () => ({
  island: { openPanel: (tab: unknown) => core.panels.push(tab) },
}));

const { translations } = await import("./translate.svelte");
const { banners } = await import("./banners.svelte");
const { settings } = await import("./settings.svelte");
const { actions } = await import("./actions.svelte");
const { registerActions } = await import("../actions");

const sent = (): unknown[] => core.calls.filter((call) => call[0] === "translate_text");
const read = (): unknown[] => core.calls.filter((call) => call[0] === "read_clipboard_text");

beforeEach(async () => {
  core.calls = [];
  core.keys = new Set();
  core.clipboard = "Quick update: the release moves to Thursday.";
  core.refuse = null;
  core.panels = [];
  translations.hasKey = undefined;
  translations.items = {};
  translations.wantsKey = false;
  translations.forgetClipboard();
  banners.queue.clear();
  await settings.update({ translate: { target: "" } });
});

describe("« Traduire » by a message or an answer", () => {
  it("offers the key when there is none, sends nothing, then opens Réglages › Chat", async () => {
    await translations.press("n-1", "See you at 6pm?");
    expect(translations.state("n-1")).toEqual({ status: "offer" });
    expect(sent()).toEqual([]);
    await translations.press("n-1", "See you at 6pm?");
    expect(core.panels).toEqual(["chat"]);
    expect(translations.wantsKey).toBe(true);
    expect(sent()).toEqual([]);
  });

  it("sends the text only on the click, into the interface's language, then toggles", async () => {
    core.keys.add("service:deepl");
    expect(translations.shown("n-1", "  See you at 6pm? ")).toBe("  See you at 6pm? ");
    expect(sent()).toEqual([]);
    await translations.press("n-1", "  See you at 6pm? ");
    expect(sent()).toEqual([["translate_text", "See you at 6pm?", "FR"]]);
    expect(translations.shown("n-1", "See you at 6pm?")).toBe("[FR] See you at 6pm?");
    await translations.press("n-1", "See you at 6pm?");
    expect(translations.shown("n-1", "See you at 6pm?")).toBe("See you at 6pm?");
    await translations.press("n-1", "See you at 6pm?");
    expect(translations.shown("n-1", "See you at 6pm?")).toBe("[FR] See you at 6pm?");
    expect(sent()).toHaveLength(1);
  });

  it("never sends a hidden message", async () => {
    core.keys.add("service:deepl");
    await translations.press("n-4", "Tu rentres ?", true);
    await translations.press("n-5", "");
    expect(sent()).toEqual([]);
    expect(translations.state("n-4")).toBeUndefined();
  });

  it("follows the language chosen in Réglages › Chat", async () => {
    core.keys.add("service:deepl");
    await translations.press("r-1", "Bonjour");
    await settings.update({ translate: { target: "DE" } });
    expect(translations.state("r-1")).toBeUndefined();
    await translations.press("r-1", "Bonjour");
    expect(sent().at(-1)).toEqual(["translate_text", "Bonjour", "DE"]);
  });

  it("says a refusal and tries again on the next click", async () => {
    core.keys.add("service:deepl");
    core.refuse = "quota";
    await translations.press("n-1", "Hello");
    expect(translations.state("n-1")).toEqual({ status: "error", failure: "quota" });
    core.refuse = null;
    await translations.press("n-1", "Hello");
    expect(translations.state("n-1")).toMatchObject({ status: "done", text: "[FR] Hello" });
  });

  it("goes back to the offer when the core no longer finds the key", async () => {
    core.keys.add("service:deepl");
    core.refuse = "noKey";
    await translations.press("n-1", "Hello");
    expect(translations.state("n-1")).toEqual({ status: "offer" });
    expect(translations.hasKey).toBe(false);
  });

  it("keeps the key in the vault only, and drops the offers once it is there", async () => {
    await translations.press("n-1", "Hello");
    expect(await translations.saveKey("abc:fx")).toBeNull();
    expect(core.calls).toContainEqual(["secret_store", "service:deepl"]);
    expect(JSON.stringify(core.calls)).not.toContain("abc:fx");
    expect(JSON.stringify(settings.value)).not.toContain("abc:fx");
    expect(translations.state("n-1")).toBeUndefined();
    expect(await translations.removeKey()).toBeNull();
    expect(translations.hasKey).toBe(false);
  });

  it("keeps a bounded number of translations in memory", async () => {
    core.keys.add("service:deepl");
    for (let i = 0; i < 70; i += 1) await translations.press(`m-${i}`, `Hello ${i}`);
    expect(Object.keys(translations.items)).toHaveLength(60);
    expect(translations.state("m-0")).toBeUndefined();
    expect(translations.state("m-69")).toMatchObject({ status: "done" });
  });
});

describe("« Traduire le presse-papiers »", () => {
  it("is an action of the shortcuts and of « OK Mochi »", () => {
    registerActions();
    expect(actions.has("translate-clipboard")).toBe(true);
  });

  it("without a key, offers it and never reads the clipboard", async () => {
    await translations.startClipboard();
    expect(translations.clipboard?.status).toBe("noKey");
    expect(read()).toEqual([]);
    expect(banners.current?.kind).toBe("translation");
    translations.addKey();
    expect(core.panels).toEqual(["chat"]);
  });

  it("reads the clipboard once, shows it, and sends it only on « Traduire »", async () => {
    core.keys.add("service:deepl");
    await translations.startClipboard();
    expect(read()).toHaveLength(1);
    expect(translations.clipboard).toMatchObject({ status: "confirm", text: "Quick update: the release moves to Thursday.", target: "FR" });
    expect(sent()).toEqual([]);
    await translations.sendClipboard();
    expect(sent()).toEqual([["translate_text", "Quick update: the release moves to Thursday.", "FR"]]);
    expect(translations.clipboard).toMatchObject({ status: "done", translation: "[FR] Quick update: the release moves to Thursday.", detected: "EN" });
    expect(banners.current?.kind).toBe("translation");
    translations.toggleClipboardOriginal();
    expect(translations.clipboard?.original).toBe(true);
    await translations.copyClipboard();
    expect(core.calls).toContainEqual(["copy_text", "[FR] Quick update: the release moves to Thursday."]);
    translations.closeClipboard();
    expect(translations.clipboard).toBeNull();
    expect(banners.current).toBeUndefined();
    expect(read()).toHaveLength(1);
  });

  it("closed before the answer, drops it", async () => {
    core.keys.add("service:deepl");
    await translations.startClipboard();
    const sending = translations.sendClipboard();
    translations.closeClipboard();
    await sending;
    expect(translations.clipboard).toBeNull();
  });

  it("says when there is nothing to translate, or too much, without sending", async () => {
    core.keys.add("service:deepl");
    core.clipboard = null;
    await translations.startClipboard();
    expect(translations.clipboard).toMatchObject({ status: "unreadable", failure: "empty" });
    core.clipboard = "  1234 !! ";
    await translations.startClipboard();
    expect(translations.clipboard).toMatchObject({ status: "unreadable", failure: "empty" });
    core.clipboard = "a".repeat(30_001);
    await translations.startClipboard();
    expect(translations.clipboard).toMatchObject({ status: "unreadable", failure: "tooLong" });
    await translations.sendClipboard();
    expect(sent()).toEqual([]);
  });

  it("says a refusal, and « Réessayer » sends again", async () => {
    core.keys.add("service:deepl");
    await translations.startClipboard();
    core.refuse = "network";
    await translations.sendClipboard();
    expect(translations.clipboard).toMatchObject({ status: "error", failure: "network" });
    core.refuse = null;
    await translations.retryClipboard();
    expect(translations.clipboard?.status).toBe("done");
    expect(sent()).toHaveLength(2);
  });
});
