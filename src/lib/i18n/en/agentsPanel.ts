// agentsPanel, in English: the same keys as ../fr/agentsPanel.ts.

import type { Catalog } from "../fr";

const s = (n: number): string => (n === 1 ? "" : "s");

export const agentsPanel: Catalog["agentsPanel"] = {
  safe: { before: "Nothing gets plugged in without your click. Before any change, the AI's config is backed up (", after: ")." },
  followed: "Followed only",
  followedCount: (n) => `${n} agents`,
  followedLead: "îlot shows what they do. Their permissions are always given in their own terminal.",
  desktop:
    "Its Code tab runs Claude Code, with the same hooks: the Claude card already follows it, permissions and questions included. The chat itself sends no events to follow.",
  badgeTitle: "Another agent",
  badge: {
    add: "Add",
    middle: "to its hooks' payload to give it its name and color, in the pill and the hub (see",
    end: ").",
  },
  card: {
    other: "Other agent",
    searching: "Looking…",
    noWsl: "No WSL distribution found.",
    notInstalled: (name) => `${name} isn't installed on Windows or in WSL.`,
    pluggedIn: (places) => `Plugged into ${places.join(" and ")}`,
    installedIn: (places) => `Installed in ${places.join(" and ")}`,
    notes: {
      claudeRepair: "Turn the switch back on to redo its connection: its questions will then show in the island.",
      repair: "Its connection needs fixing: turn the switch back on to redo it.",
      codexHooks: "In Codex, type /hooks and approve îlot's hooks.",
      reload: (name) => `Restart ${name} so it loads îlot's plugin.`,
      geminiTerminal: "Gemini asks for its permissions in its terminal: answer there directly.",
      inTerminal: (name) => `${name} is waiting for your answer in its terminal: answer there directly.`,
      claudeTrust: "Claude ignores the hooks of a folder you haven't trusted yet.",
      cursor: "Cursor also runs Claude's hooks: îlot counts them for Cursor, only once.",
      followedOnly: "Activity only: its permissions are given in its terminal, never through îlot.",
      other: "For any other tool: ilot emit and ilot run -- command in WSL, or ilot-cli.exe on Windows.",
    },
    wait: "One moment…",
    disconnect: "Disconnect",
    connect: "Connect",
    connectName: (name) => `Connect ${name}`,
    steps: (name) => `Latest actions of ${name}`,
    noAction: (name) => `No actions yet. Start a session in ${name ?? "your tool"}: it will show up here.`,
    busy: (share) => `Active ${share}% of the hour`,
    sessions: (n, tools) => `, ${n} sessions (${tools})`,
    reachTip: (project) => `Brings the ${project} window to the front`,
    reach: "Go to terminal",
  },
  journal: {
    title: "Decision log",
    count: (n) => `${n} decision${s(n)}`,
    less: "Show less",
    all: (n) => `Show all (${n})`,
    empty: "No decisions yet. Every request you allow or deny will be logged here.",
  },
};
