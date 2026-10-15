// The Monday report as an image (1080 × 1920, a phone story): Mochi, the week, its hours
// of code, one bar per day, the key figures, the main agent and project (masked on
// demand). Drawn on a canvas in the theme's own colors, from the counters only; saved
// where the user chooses (export.rs). Nothing goes on the network.

import { sourceColor, sourceTool } from "./agents";
import { texts } from "./i18n/live.svelte";
import type { MochiSettings } from "./ipc";
import { drawMochi } from "./mochi/engine";
import { stillPose } from "./mochi/moves";
import { DAY_LETTERS, type WeekReport, barHeights, count, figures, projectLabel, weekRange } from "./report";
import { durationLabel } from "./timeline";

export const IMAGE_WIDTH = 1080;
export const IMAGE_HEIGHT = 1920;
const MARGIN = 96;
const FONT = "Inter, system-ui, sans-serif";

/** The theme's colors, read from its tokens (tokens.css). */
type Palette = {
  panel: string;
  glow: string;
  line: string;
  ink: string;
  ink2: string;
  ink3: string;
  accent: string;
};

function palette(): Palette {
  const style = getComputedStyle(document.documentElement);
  const token = (name: string): string => style.getPropertyValue(name).trim();
  return {
    panel: token("--panel"),
    glow: token("--bg-glow"),
    line: token("--line-2"),
    ink: token("--ink"),
    ink2: token("--ink-2"),
    ink3: token("--ink-3"),
    accent: token("--accent-2"),
  };
}

/** An AI's color as the canvas needs it (`var(--ai-…)` resolved). */
function aiColor(source: string): string {
  const color = sourceColor(source);
  const name = /var\((--[\w-]+)\)/.exec(color)?.[1];
  return name ? getComputedStyle(document.documentElement).getPropertyValue(name).trim() : color;
}

type TextStyle = { size: number; weight?: number; color: string; align?: CanvasTextAlign; max?: number };

/** One line of text, cut with « … » when wider than `max`. */
function write(c: CanvasRenderingContext2D, text: string, x: number, y: number, style: TextStyle): void {
  c.font = `${style.weight ?? 400} ${style.size}px ${FONT}`;
  c.fillStyle = style.color;
  c.textAlign = style.align ?? "left";
  c.textBaseline = "alphabetic";
  let shown = text;
  if (style.max) {
    while (shown.length > 1 && c.measureText(shown).width > style.max) shown = shown.slice(0, -1);
    if (shown !== text) shown = `${shown.slice(0, -1)}…`;
  }
  c.fillText(shown, x, y);
}

/** A bar with a rounded top, square on its baseline. */
function bar(c: CanvasRenderingContext2D, x: number, base: number, width: number, height: number, radius: number): void {
  const r = Math.min(radius, height, width / 2);
  c.beginPath();
  c.moveTo(x, base);
  c.lineTo(x, base - height + r);
  c.arcTo(x, base - height, x + r, base - height, r);
  c.lineTo(x + width - r, base - height);
  c.arcTo(x + width, base - height, x + width, base - height + r, r);
  c.lineTo(x + width, base);
  c.closePath();
  c.fill();
}

function drawMochiAt(c: CanvasRenderingContext2D, mochi: Pick<MochiSettings, "palette" | "kind">, x: number, y: number, size: number): void {
  const art = document.createElement("canvas");
  art.width = size;
  art.height = size;
  const context = art.getContext("2d");
  if (!context) return;
  drawMochi(context, size, { ...stillPose("breathe", { face: "happy" }), palette: mochi.palette, kind: mochi.kind, wave: true }, 250);
  c.drawImage(art, x, y);
}

/** Paints the whole report on a 1080 × 1920 context. */
export function drawReport(c: CanvasRenderingContext2D, report: WeekReport, hideProjects: boolean, mochi: Pick<MochiSettings, "palette" | "kind">): void {
  const p = palette();
  const said = texts().timeline.report;
  const center = IMAGE_WIDTH / 2;
  const inner = IMAGE_WIDTH - 2 * MARGIN;

  c.fillStyle = p.panel;
  c.fillRect(0, 0, IMAGE_WIDTH, IMAGE_HEIGHT);
  const glow = c.createRadialGradient(center, 300, 0, center, 300, 700);
  glow.addColorStop(0, p.glow);
  glow.addColorStop(1, "transparent");
  c.fillStyle = glow;
  c.fillRect(0, 0, IMAGE_WIDTH, 1000);

  drawMochiAt(c, mochi, center - 168, 96, 336);
  write(c, said.imageTitle, center, 520, { size: 60, weight: 600, color: p.ink, align: "center" });
  write(c, weekRange(report.monday), center, 580, { size: 34, color: p.ink2, align: "center" });

  // The one big number: hours of code.
  write(c, durationLabel(report.total.activeMs), center, 780, { size: 150, weight: 600, color: p.ink, align: "center" });
  write(c, said.imageHours, center, 840, { size: 36, color: p.ink2, align: "center" });

  // One bar per day on a hairline baseline, the others in a quiet gray, the busiest in
  // the accent with its value on top.
  const base = 1190;
  const tall = 220;
  const slot = inner / 7;
  const width = 56;
  const heights = barHeights(report);
  c.fillStyle = p.line;
  c.fillRect(MARGIN, base, inner, 2);
  heights.forEach((h, day) => {
    const x = MARGIN + day * slot + (slot - width) / 2;
    const busiest = day === report.busiestDay;
    c.fillStyle = busiest ? p.accent : p.ink3;
    c.globalAlpha = busiest ? 1 : 0.6;
    if (h > 0) bar(c, x, base, width, Math.max(10, h * tall), 12);
    c.globalAlpha = 1;
    write(c, DAY_LETTERS[day], x + width / 2, base + 50, { size: 30, weight: 500, color: busiest ? p.ink : p.ink3, align: "center" });
    if (busiest) {
      write(c, durationLabel(report.days[day].activeMs), x + width / 2, base - h * tall - 22, { size: 30, weight: 600, color: p.ink, align: "center" });
    }
  });

  // Key figures, two columns set apart by a hairline.
  const shown = new Map(figures(report, hideProjects).map((f) => [f.id, f]));
  const grid = ["sessions", "files", "lines", "commands", "permissions", "longest"];
  const top = 1300;
  const row = 120;
  const column = inner / 2;
  grid.forEach((id, index) => {
    const figure = shown.get(id);
    if (!figure) return;
    const x = MARGIN + (index % 2) * column + (index % 2 ? 40 : 0);
    const y = top + Math.floor(index / 2) * row;
    write(c, figure.value, x, y + 56, { size: 54, weight: 600, color: p.ink, max: column - 48 });
    const label = id === "permissions" && figure.detail ? `${figure.label}, ${figure.detail}` : figure.label;
    write(c, label, x, y + 94, { size: 28, color: p.ink2, max: column - 48 });
  });
  c.fillStyle = p.line;
  c.fillRect(center, top + 10, 2, 3 * row - 20);

  // The main agent (its color in a dot only) and the main project, each under its name.
  const agent = report.agents[0];
  const project = report.projects[0];
  const y = 1800;
  c.fillStyle = p.line;
  c.fillRect(MARGIN, 1690, inner, 2);
  if (agent) {
    write(c, said.mainAgent, MARGIN, y - 52, { size: 28, color: p.ink2 });
    c.fillStyle = aiColor(agent.name);
    c.beginPath();
    c.arc(MARGIN + 11, y - 11, 11, 0, Math.PI * 2);
    c.fill();
    write(c, `${sourceTool(agent.name)}, ${durationLabel(agent.ms)}`, MARGIN + 36, y, { size: 34, weight: 600, color: p.ink, max: column - 56 });
  }
  if (project) {
    const x = MARGIN + column + 40;
    write(c, said.mainProject, x, y - 52, { size: 28, color: p.ink2 });
    write(c, projectLabel(report, project.name, hideProjects), x, y, { size: 34, weight: 600, color: p.ink, max: column - 48 });
  }
  write(c, said.imageFooter(count(report.total.sessions)), center, IMAGE_HEIGHT - 44, {
    size: 26,
    color: p.ink3,
    align: "center",
  });
}

/** The report drawn as a PNG file's bytes. */
export async function paintReport(report: WeekReport, hideProjects: boolean, mochi: Pick<MochiSettings, "palette" | "kind">): Promise<Uint8Array> {
  await Promise.all([document.fonts.load(`600 60px ${FONT}`), document.fonts.load(`400 34px ${FONT}`), document.fonts.load(`500 32px ${FONT}`)]);
  const canvas = document.createElement("canvas");
  canvas.width = IMAGE_WIDTH;
  canvas.height = IMAGE_HEIGHT;
  const context = canvas.getContext("2d");
  if (!context) throw new Error(texts().timeline.report.drawFailed);
  drawReport(context, report, hideProjects, mochi);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (!blob) throw new Error(texts().timeline.report.imageFailed);
  return new Uint8Array(await blob.arrayBuffer());
}
