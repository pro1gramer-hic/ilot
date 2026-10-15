// voice, in English: the same keys as ../fr/voice.ts. The phrases are what Windows listens
// for when îlot speaks English.

import type { Catalog } from "../fr";

const s = (n: number): string => (n === 1 ? "" : "s");

export const voice: Catalog["voice"] = {
  commands: {
    toggleIsland: { phrases: ["open the island"], reply: "Here's the island." },
    goToAlert: { phrases: ["show the alert", "what's waiting for me"], reply: "Here's what's waiting for you." },
    openChat: { phrases: ["open the chat"], reply: "Opening the chat." },
    goToTerminal: { phrases: ["go to the terminal"], reply: "Taking you to the terminal." },
    nextPastille: { phrases: ["next widget"], reply: "Next widget." },
    mute: { phrases: ["mute the sounds"], reply: "Muting the sounds." },
    mochiToDesktop: { phrases: ["go to the desktop"], reply: "On my way." },
    openWardrobe: { phrases: ["open the wardrobe"], reply: "Here's my wardrobe." },
    translateClipboard: { phrases: ["translate the clipboard"], reply: "Let me look at the clipboard." },
    report: { phrases: ["show the report"], reply: "Here's your week in review." },
    stop: { phrases: ["stop listening"], reply: "I'll stop listening." },
  },
  summary: { phrases: ["sum up the activity", "what's new"], question: "Sum up my activity." },
  status: {
    active: "at work",
    waiting: "waiting for an answer",
    paused: "paused",
    done: "finished",
    failed: "failed",
  },
  agentLine: (tool, status, project) => `${tool} ${status} on ${project}`,
  agentsSeparator: "; ",
  noAgent: "no agent open",
  agents: (list) => `Agents: ${list}.`,
  today: (finished, failures, decisions) =>
    `Today: ${finished} task${s(finished)} done, ${failures} failure${s(failures)}, ${decisions} decision${s(decisions)}.`,
  micOff: { title: "Mic off", sub: "Turn “OK Mochi” back on in Settings, Shortcuts." },
  yes: "Yes?",
  notUnderstood: "I didn't get that.",
  cannotYet: "I can't do that yet.",
  clickToSend: "Click Send and I'll ask Claude.",
  noClaudeKey: "I don't have your Claude key. Add it in the chat settings, then ask me again.",
};
