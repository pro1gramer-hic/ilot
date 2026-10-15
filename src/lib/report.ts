// The Monday report (« Bilan du lundi », prompt 12, lot 5): the week's counters as the core
// keeps them (src-tauri/src/stats.rs, numbers and project names only) and how the report
// says them: the week's dates, durations, the key figures, projects masked on demand.

import { sourceTool } from "./agents";
import { liveList, texts, textsLocale } from "./i18n/live.svelte";
import { durationLabel } from "./timeline";

export type ReportWeek = "last" | "current";

/** One day's counters (stats.rs, `Day`). */
export type DayCounters = {
  activeMs: number;
  sessions: number;
  files: number;
  added: number;
  removed: number;
  commands: number;
  allowed: number;
  denied: number;
};

export type Share = { name: string; ms: number };

/** A week (stats.rs, `Report`): Monday first, the busiest AI and projects first. */
export type WeekReport = {
  /** The Monday, local date "2026-10-05". */
  monday: string;
  days: DayCounters[];
  total: DayCounters;
  agents: Share[];
  projects: Share[];
  busiestDay: number | null;
  longest: { ms: number; source: string; project: string } | null;
};

/** A weekday in the language of the moment, Monday first: « lundi », « L » ("Monday", "M"). */
function weekday(day: number, width: "long" | "narrow"): string {
  // 1 January 2024 was a Monday.
  return new Intl.DateTimeFormat(textsLocale(), { weekday: width, timeZone: "UTC" }).format(Date.UTC(2024, 0, 1 + day));
}

export const DAY_NAMES: readonly string[] = liveList(7, (day) => weekday(day, "long"));
export const DAY_LETTERS: readonly string[] = liveList(7, (day) => weekday(day, "narrow"));

function dateOf(iso: string, plusDays = 0): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d + plusDays));
}

/** « du 5 au 11 octobre 2026 », « du 28 septembre au 4 octobre 2026 ». */
export function weekRange(monday: string): string {
  return texts().timeline.report.weekRange(dateOf(monday), dateOf(monday, 6));
}

/** The Monday of the week that holds `date` (local), "2026-10-05". */
export function mondayOf(date: Date): string {
  const monday = new Date(date.getFullYear(), date.getMonth(), date.getDate() - ((date.getDay() + 6) % 7));
  const two = (n: number): string => String(n).padStart(2, "0");
  return `${monday.getFullYear()}-${two(monday.getMonth() + 1)}-${two(monday.getDate())}`;
}

/** « 1 204 »: grouped as the language writes it (a thin space in French). */
export function count(n: number): string {
  return new Intl.NumberFormat(textsLocale()).format(n);
}

/** Projects as the report may show them: their names, or « Projet A », « Projet B »… in
 * the order of the week when the user hides them. */
export function projectLabel(report: WeekReport, name: string, hidden: boolean): string {
  if (!hidden) return name;
  const rank = report.projects.findIndex((p) => p.name === name);
  const said = texts().timeline.report;
  return rank >= 0 && rank < 26 ? said.project(String.fromCharCode(65 + rank)) : said.projectHidden;
}

export type Figure = { id: string; label: string; value: string; detail?: string };

/** The key figures, in the order the report and the image show them. */
export function figures(report: WeekReport, hidden: boolean): Figure[] {
  const { total, longest, busiestDay } = report;
  const main = report.agents[0];
  const project = report.projects[0];
  const said = texts().timeline.report;
  return [
    { id: "time", label: said.codeTime, value: durationLabel(total.activeMs), detail: said.agentsAtWork },
    { id: "sessions", label: said.sessions, value: count(total.sessions) },
    { id: "files", label: said.files, value: count(total.files) },
    { id: "lines", label: said.lines, value: `+${count(total.added)} −${count(total.removed)}` },
    { id: "commands", label: said.commands, value: count(total.commands) },
    {
      id: "permissions",
      label: said.permissions,
      value: said.allowed(total.allowed, count(total.allowed)),
      detail: said.denied(total.denied, count(total.denied)),
    },
    { id: "agent", label: said.mainAgent, value: main ? sourceTool(main.name) : said.none, detail: main ? durationLabel(main.ms) : undefined },
    {
      id: "project",
      label: said.mainProject,
      value: project ? projectLabel(report, project.name, hidden) : said.none,
      detail: project ? durationLabel(project.ms) : undefined,
    },
    {
      id: "busiest",
      label: said.busiestDay,
      value: busiestDay === null ? said.none : capitalize(DAY_NAMES[busiestDay]),
      detail: busiestDay === null ? undefined : durationLabel(report.days[busiestDay].activeMs),
    },
    {
      id: "longest",
      label: said.longest,
      value: longest ? durationLabel(longest.ms) : said.noSession,
      detail: longest ? `${sourceTool(longest.source)}, ${projectLabel(report, longest.project, hidden)}` : undefined,
    },
  ];
}

function capitalize(text: string): string {
  return text.charAt(0).toLocaleUpperCase(textsLocale()) + text.slice(1);
}

/** True when nothing at all was counted that week. */
export function isEmpty(report: WeekReport): boolean {
  return Object.values(report.total).every((value) => value === 0);
}

/** Height of each day's bar, from 0 to 1 against the busiest day. */
export function barHeights(report: WeekReport): number[] {
  const top = Math.max(0, ...report.days.map((d) => d.activeMs));
  return report.days.map((d) => (top > 0 ? d.activeMs / top : 0));
}

/** The image's suggested file name, ASCII only: « Bilan ilot 2026-10-05 ». */
export function imageName(report: WeekReport): string {
  return texts().timeline.report.imageName(report.monday);
}
