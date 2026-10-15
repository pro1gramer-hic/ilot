// « OK Mochi » in the interface (prompt 12, lot 5): the microphone follows the setting (off
// by default), what Windows recognized becomes an action of the island, and a question for
// Claude waits on the island for a click before anything leaves the PC. Mochi answers in
// his bubble and aloud.

import { untrack } from "svelte";
import { decisionsSince } from "../agents";
import { texts } from "../i18n/live.svelte";
import { type VoiceGrammar, voiceAsk, voiceHush, voiceListen, voiceSay } from "../ipc";
import { island } from "../island/controller.svelte";
import { startOfDay } from "../timeline";
import { activityContext, commandFor, grammarFor } from "../voice";
import { actions } from "./actions.svelte";
import { banners } from "./banners.svelte";
import { bubble } from "./bubble.svelte";
import { decisions } from "./decisions.svelte";
import { history } from "./history.svelte";
import { mic } from "./mic.svelte";
import { mood } from "./mood.svelte";
import { report } from "./report.svelte";
import { sessions } from "./sessions.svelte";
import { settings } from "./settings.svelte";

/** The question on the island: what Mochi heard, exactly what would be sent, then the
 * answer (or why there is none). */
export type VoiceAsk = {
  question: string;
  context: string;
  status: "confirm" | "sending" | "answer" | "error";
  answer: string | null;
};

class VoiceStore {
  /** The microphone is open (the dot on Mochi shows it). */
  listening = $state(false);
  error = $state<string | null>(null);
  /** Replaced whole, never changed in place (`$state.raw`): `confirm` compares it. */
  ask = $state.raw<VoiceAsk | null>(null);
  /** Counts the keyword heard: the dot answers each time. */
  wakes = $state(0);
  /** What the core was last asked: "off", or the grammar it listens for. */
  #applied = "off";

  /** Follows the setting and the actions ready (their phrases come and go with them). The
   * chat's dictation uses the same speech recognizer: « OK Mochi » pauses while it runs. */
  follow(): () => void {
    return $effect.root(() => {
      $effect(() => {
        const on = settings.value.voice.listen && !mic.dictating;
        const grammar = grammarFor(settings.value.mochi.name, actions.ready);
        untrack(() => void this.apply(on, grammar));
      });
    });
  }

  /** Asks the core to open (with these words) or close the microphone, once per change. */
  async apply(on: boolean, grammar: VoiceGrammar): Promise<void> {
    const wanted = on ? JSON.stringify(grammar) : "off";
    if (wanted === this.#applied) return;
    this.#applied = wanted;
    try {
      this.listening = await voiceListen(on, on ? grammar : null);
      this.error = null;
    } catch (error) {
      this.listening = false;
      this.error = String(error);
    }
  }

  /** The dot on Mochi, or « arrête d'écouter »: the microphone closes and the setting goes
   * off (it comes back from Réglages, Raccourcis). */
  stop(): void {
    void settings.update({ voice: { ...settings.value.voice, listen: false } });
    banners.toast("acc", texts().voice.micOff.title, texts().voice.micOff.sub);
  }

  /** Mochi says it in his bubble, and aloud when the setting allows. */
  reply(text: string): void {
    bubble.say(text);
    if (settings.value.voice.speak) void voiceSay(text);
  }

  /** What Windows recognized: the keyword alone wakes Mochi up for a phrase; a phrase
   * starts its action. */
  heard({ wake, phrase }: { wake: boolean; phrase: string }): void {
    if (!phrase) {
      if (!wake) return;
      this.wakes += 1;
      mood.flash("wow", 1800);
      this.reply(texts().voice.yes);
      return;
    }
    const command = commandFor(phrase, actions.ready);
    if (!command) {
      this.reply(texts().voice.notUnderstood);
      return;
    }
    switch (command.kind) {
      case "action":
        this.reply(actions.run(command.action) ? command.reply : texts().voice.cannotYet);
        break;
      case "report":
        island.openPanel("today");
        void report.show("last");
        this.reply(command.reply);
        break;
      case "stop":
        this.reply(command.reply);
        this.stop();
        break;
      case "ask":
        this.prepare(command.question);
        break;
    }
  }

  /** Shows on the island what Claude would get; nothing goes before « Envoyer à Claude ». */
  prepare(question: string): void {
    const today = {
      finished: history.log.finished.length,
      failures: history.log.failures,
      decisions: decisionsSince(decisions.list, startOfDay(Date.now())),
    };
    this.ask = { question, context: activityContext(sessions.list, today), status: "confirm", answer: null };
    this.#showAgain();
    this.reply(texts().voice.clickToSend);
  }

  /** The user's click: the question goes to Claude with their key, the answer comes back
   * on the island and aloud. */
  async confirm(): Promise<void> {
    const asked = this.ask;
    if (asked?.status !== "confirm") return;
    const sending: VoiceAsk = { ...asked, status: "sending" };
    this.ask = sending;
    banners.hold("reply", true);
    let next: VoiceAsk;
    try {
      next = { ...asked, status: "answer", answer: await voiceAsk(asked.question, asked.context) };
    } catch (error) {
      next = { ...asked, status: "error", answer: error instanceof Error ? error.message : String(error) };
    } finally {
      banners.hold("reply", false);
    }
    // Closed meanwhile: the answer is dropped.
    if (this.ask !== sending) return;
    this.ask = next;
    this.#showAgain();
    if (next.answer) this.reply(next.answer);
  }

  /** « Répéter ». */
  repeat(): void {
    if (this.ask?.answer) void voiceSay(this.ask.answer);
  }

  close(): void {
    this.ask = null;
    void voiceHush();
    if (banners.current?.kind === "voice") banners.queue.dismissCurrent();
  }

  /** The opening (again, with its full time) for the question or its answer. */
  #showAgain(): void {
    if (banners.current?.kind === "voice") banners.queue.dismissCurrent();
    banners.queue.push({ kind: "voice" });
  }
}

export const voice = new VoiceStore();
