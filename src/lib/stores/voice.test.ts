import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

// The settings store marks the page's theme: a bare document is enough here.
vi.hoisted(() => {
  if (typeof document === "undefined") Object.assign(globalThis, { document: { documentElement: { dataset: {} } } });
});

// The store talks to the Rust core through ipc.ts: a fake core records every call.
const core = vi.hoisted(() => ({ calls: [] as unknown[][], answer: "Deux agents travaillent, tout va bien." as string | null }));

vi.mock("../ipc", async (importOriginal) => {
  const real = await importOriginal<typeof import("../ipc")>();
  return {
    ...real,
    saveSettings: async (settings: unknown) => settings,
    voiceListen: async (on: boolean, grammar: unknown) => {
      core.calls.push(["voice_listen", on, grammar]);
      return on;
    },
    voiceSay: async (text: string) => {
      core.calls.push(["voice_say", text]);
    },
    voiceHush: async () => {
      core.calls.push(["voice_hush"]);
    },
    voiceAsk: async (question: string, context: string) => {
      core.calls.push(["voice_ask", question, context]);
      if (core.answer === null) throw new Error(real.NO_CLAUDE_KEY);
      return core.answer;
    },
  };
});

const { NO_CLAUDE_KEY } = await import("../ipc");
const { registerActions } = await import("../actions");
const { banners } = await import("./banners.svelte");
const { bubble } = await import("./bubble.svelte");
const { settings } = await import("./settings.svelte");
const { voice } = await import("./voice.svelte");
const { actions } = await import("./actions.svelte");
const { grammarFor } = await import("../voice");

const called = (name: string): unknown[][] => core.calls.filter((call) => call[0] === name);

/** Lets the store's effect and its call to the core run. */
async function settle(): Promise<void> {
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

registerActions();
voice.follow();

beforeEach(() => {
  core.calls = [];
  core.answer = "Deux agents travaillent, tout va bien.";
  voice.ask = null;
  banners.queue.clear();
});

describe("« OK Mochi »", () => {
  it("keeps the microphone closed until the user turns it on", async () => {
    await settle();
    expect(settings.value.voice.listen).toBe(false);
    expect(called("voice_listen")).toEqual([]);
    // What the store's effect does when the setting changes (effects do not run under
    // Node; the interface tests cover the switch in the tab).
    const words = grammarFor(settings.value.mochi.name, actions.ready);
    await voice.apply(false, words);
    expect(called("voice_listen"), "off stays off: the core is not even asked").toEqual([]);
    await settings.update({ voice: { listen: true, speak: true } });
    await voice.apply(settings.value.voice.listen, words);
    await voice.apply(settings.value.voice.listen, words);
    expect(called("voice_listen")).toEqual([["voice_listen", true, words]]);
    expect(words.phrases).toContain("montre l'alerte");
    expect(words.phrases).toContain("coupe le son");
    expect(words.phrases).toContain("va au terminal");
    expect(voice.listening).toBe(true);
  });

  it("sends nothing before the click on « Envoyer à Claude »", async () => {
    voice.heard({ wake: true, phrase: "résume l'activité" });
    expect(voice.ask?.status).toBe("confirm");
    expect(banners.current?.kind).toBe("voice");
    expect(called("voice_ask")).toEqual([]);
    await voice.confirm();
    expect(called("voice_ask")).toHaveLength(1);
    const [, question, context] = called("voice_ask")[0];
    expect(question).toBe("Résume mon activité.");
    expect(context).toMatch(/^Agents : /);
    expect(voice.ask).toMatchObject({ status: "answer", answer: "Deux agents travaillent, tout va bien." });
    expect(called("voice_say").at(-1)).toEqual(["voice_say", "Deux agents travaillent, tout va bien."]);
  });

  it("drops the question when the user cancels it", async () => {
    voice.heard({ wake: true, phrase: "quoi de neuf" });
    voice.close();
    await voice.confirm();
    expect(called("voice_ask")).toEqual([]);
    expect(banners.current).toBeUndefined();
  });

  it("says so when there is no Claude key", async () => {
    core.answer = null;
    voice.heard({ wake: true, phrase: "résume l'activité" });
    await voice.confirm();
    expect(voice.ask).toMatchObject({ status: "error", answer: NO_CLAUDE_KEY });
    expect(bubble.text).toBe(NO_CLAUDE_KEY);
    expect(called("voice_say").at(-1)).toEqual(["voice_say", NO_CLAUDE_KEY]);
  });

  it("runs local actions at once, and says what it cannot do yet", () => {
    voice.heard({ wake: true, phrase: "montre l'alerte" });
    expect(bubble.text).toBe("Voilà ce qui t'attend.");
    voice.heard({ wake: true, phrase: "fais le café" });
    expect(bubble.text).toBe("Je n'ai pas compris.");
    expect(called("voice_ask")).toEqual([]);
  });

  it("wakes up on the keyword alone", () => {
    const before = voice.wakes;
    voice.heard({ wake: true, phrase: "" });
    expect(voice.wakes).toBe(before + 1);
    expect(bubble.text).toBe("Oui ?");
  });

  it("closes the microphone in one click, and the setting with it", async () => {
    voice.stop();
    await settle();
    expect(settings.value.voice.listen).toBe(false);
    expect(banners.current).toMatchObject({ kind: "toast", title: "Micro coupé" });
    await voice.apply(settings.value.voice.listen, grammarFor("Mochi", actions.ready));
    expect(called("voice_listen").at(-1)).toEqual(["voice_listen", false, null]);
    expect(voice.listening).toBe(false);
  });
});
