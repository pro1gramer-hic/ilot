// today, in English: the same keys as ../fr/today.ts.

import type { Catalog } from "../fr";

export const today: Catalog["today"] = {
  stats: {
    finished: "Tasks done",
    failures: "Failures",
    decisions: "Decisions made",
    activeTime: "Active time",
  },
  activity: "Activity, last hour",
  activityHint: "Hover a bar or a mark",
  running: "Running",
  since: "for ",
  nobodyWorks: "No agent is working.",
  lastDecision: "Last decision",
  decidedBy: (tool, time) => `${tool}, at ${time}`,
  noDecision: "No decisions yet. Your agents' requests will show up here.",
  numbers: "Today",
  recent: "Recent messages",
  seeAll: "See all",
  hidden: (app) => `Content hidden, ${app}`,
  noMessage: "No messages yet.",
  week: "Week in review",
  weekHint: "Coding time, sessions, lines and permissions, day by day. Stays on your PC.",
  seeWeek: "See the review",
  timeline: {
    kinds: { done: "Done", failed: "Failed", permission: "Permission" },
    work: "Work",
    now: "now",
    minutesAgo: (minutes) => `−${minutes} min`,
    bar: (label, minutes, running) => `${label}, ${minutes}${running ? ", running" : ""}`,
    mark: (agent, label, verdict, time) => `${agent}, ${label}${verdict ? `, ${verdict.toLowerCase()}` : ""}, at ${time}`,
    target: (title, detail) => `${title}: ${detail}`,
    empty: "No activity this hour. Agents show up here as soon as they work.",
    targets: "Timeline marks, browse with the arrow keys",
    legend: "Legend",
  },
};
