// report, in English: the same keys as ../fr/report.ts.

import type { Catalog } from "../fr";

export const report: Catalog["report"] = {
  weeks: { last: "Last week", current: "This week" },
  week: "Week",
  back: "Today",
  counting: "Adding it all up…",
  emptyTitle: "No review for this week yet",
  emptyText: "îlot counts your agents' time from now on. Come back on Monday: the review will be ready.",
  title: (range) => `Your week ${range}`,
  hero: "of code with your agents",
  days: "Coding time per day",
  chart: "Coding time per day, the busiest highlighted",
  day: (day, time) => `${day}: ${time}`,
  byAgent: "By agent",
  share: (time, percent) => `${time}, ${percent}%`,
  hideProjects: "Hide project names",
  exported: {
    drawing: "Preparing the image…",
    saved: "Image saved.",
    cancelled: "Saving canceled.",
    error: "The image couldn't be saved.",
  },
  export: "Export image",
  local: "The counters stay on your PC: numbers and project names, never a command or a message.",
  confirmClear: "Yes, erase my stats",
  clear: "Erase my stats",
};
