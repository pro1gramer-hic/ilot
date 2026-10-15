// timeline, in English: the same keys as ../fr/timeline.ts.

import type { Catalog } from "../fr";

const range = new Intl.DateTimeFormat("en", { day: "numeric", month: "long", year: "numeric", timeZone: "UTC" });

export const timeline: Catalog["timeline"] = {
  steps: {
    waiting: (task) => `Waiting for your OK: ${task}`,
    done: (task) => `Finished: ${task}`,
    failed: (task) => `Failed: ${task}`,
    paused: (task) => `Paused: ${task}`,
  },
  duration: {
    lessThanMinute: "less than a minute",
    minutes: (n) => `${n} min`,
    hours: (hours, minutes) => `${hours}h ${minutes}`,
    seconds: (n) => `${n} s`,
  },
  report: {
    /** "October 5 – 11, 2026", "September 28 – October 4, 2026". */
    weekRange: (start, end) => range.formatRange(start, end),
    codeTime: "Coding time",
    agentsAtWork: "agents at work",
    sessions: "Sessions",
    files: "Files touched",
    lines: "Lines",
    commands: "Commands run",
    permissions: "Permissions",
    allowed: (_n, count) => `${count} allowed`,
    denied: (_n, count) => `${count} denied`,
    mainAgent: "Main agent",
    mainProject: "Main project",
    busiestDay: "Busiest day",
    longest: "Longest session",
    none: "None",
    noSession: "None",
    project: (letter) => `Project ${letter}`,
    projectHidden: "Hidden project",
    imageName: (monday) => `ilot week ${monday}`,
    imageTitle: "My week with îlot",
    imageHours: "of code with my agents",
    imageFooter: (sessions) => `${sessions} sessions counted on my PC, by îlot`,
    drawFailed: "Can't draw it.",
    imageFailed: "Can't create the image.",
    cleared: { title: "Stats cleared", sub: "The report starts over from zero." },
    ready: { title: "Your week in review is ready", sub: "Click the island to see it" },
  },
};
