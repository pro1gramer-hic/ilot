// notifications, in English: the same keys as ../fr/notifications.ts.

import type { Catalog } from "../fr";

export const notifications: Catalog["notifications"] = {
  grouped: (n) => ` · ${n} new messages`,
  newFrom: "New message from ",
  said: (text) => `: ${text}`,
  masked: "Content hidden",
  noMessage: "No messages",
  mine: (text) => `You: ${text}`,
  quickReplies: {
    whatsapp: ["Be there around 6", "Perfect, thanks!", "I'll call you back"],
    discord: ["I'll look tonight", "Send the link", "Not before tomorrow"],
    snapchat: ["I'll look later", "Haha", "I'll text you after"],
    teams: ["On my way", "I'll look into it", "Let's talk after"],
    slack: ["I'll look into it", "Thanks!", "I'll get back to you"],
    gmail: ["Got it, thanks", "I'll look into it", "I'll reply soon"],
    other: ["Okay", "Thanks!", "I'll reply later"],
  },
  replyCopied: (app, opened) => (opened ? `Reply copied: paste it in ${app}` : `Reply copied: open ${app} and paste it`),
  writeFirst: "Write a reply first.",
  notCopied: (reason) => `Reply not copied: ${reason}`,
  mailOpened: (client) => (client === null ? "Message opened in your mail app" : `Message opened in ${client}`),
  mailCheck: "Check it, then send it from there.",
  finished: (name, project, duration) => `${name} finished · ${project} · ${duration}`,
  finishedMany: (n) => `${n} agents finished`,
  allowed: { title: "Command allowed", sub: (tool) => `${tool} can go on.` },
  denied: { title: "Command denied", sub: (tool) => `${tool} has been told.` },
  always: { title: "Always allowed", sub: (detail) => `Only for: ${detail}` },
  inTerminal: { title: "Answer in the terminal", sub: (tool) => `${tool} is waiting for your answer there.` },
  answered: { title: "Answer sent", sub: (tool) => `${tool} goes on with your answer.` },
  dropped: {
    title: (file) => `${file}: path copied`,
    sub: "Paste it in your agent's terminal. îlot didn't open the file.",
  },
};
