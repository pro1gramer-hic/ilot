import { describe, expect, it, vi } from "vitest";
import { DEMO_THIS_WEEK, DEMO_WEEK } from "./fixtures";
import { barHeights, count, figures, imageName, isEmpty, mondayOf, projectLabel, weekRange } from "./report";
import { banners } from "./stores/banners.svelte";
import { report } from "./stores/report.svelte";

describe("the week's report", () => {
  it("says the week's dates in French", () => {
    expect(weekRange("2026-10-05")).toBe("du 5 au 11 octobre 2026");
    expect(weekRange("2026-09-28")).toBe("du 28 septembre au 4 octobre 2026");
    expect(weekRange("2025-12-29")).toBe("du 29 décembre 2025 au 4 janvier 2026");
    expect(weekRange("2026-05-25")).toBe("du 25 au 31 mai 2026");
    expect(weekRange("2026-06-01")).toBe("du 1er au 7 juin 2026");
  });

  it("finds the Monday of any day", () => {
    expect(mondayOf(new Date(2026, 9, 11))).toBe("2026-10-05");
    expect(mondayOf(new Date(2026, 9, 12))).toBe("2026-10-12");
    expect(mondayOf(new Date(2026, 0, 1))).toBe("2025-12-29");
  });

  it("gives every key figure of the prompt", () => {
    const shown = Object.fromEntries(figures(DEMO_WEEK, false).map((f) => [f.id, f]));
    expect(shown.time.value).toBe("11 h 45");
    expect(shown.sessions.value).toBe("26");
    expect(shown.files.value).toBe("72");
    expect(shown.lines.value).toBe(`+${count(2368)} −789`);
    expect(shown.commands.value).toBe("211");
    expect(shown.permissions).toMatchObject({ value: "34 autorisées", detail: "5 refusées" });
    expect(shown.agent).toMatchObject({ value: "Claude Code", detail: "7 h 50" });
    expect(shown.project.value).toBe("epilot");
    expect(shown.busiest).toMatchObject({ value: "Mardi", detail: "3 h 25" });
    expect(shown.longest).toMatchObject({ value: "1 h 52", detail: "Claude Code, epilot" });
  });

  it("hides project names on demand, everywhere", () => {
    const shown = figures(DEMO_WEEK, true);
    const text = JSON.stringify(shown);
    for (const name of ["epilot", "ilot", "site-vitrine"]) expect(text).not.toContain(`"${name}`);
    expect(shown.find((f) => f.id === "project")?.value).toBe("Projet A");
    expect(projectLabel(DEMO_WEEK, "site-vitrine", true)).toBe("Projet C");
    expect(projectLabel(DEMO_WEEK, "inconnu", true)).toBe("Projet masqué");
  });

  it("draws bars against the busiest day", () => {
    const heights = barHeights(DEMO_WEEK);
    expect(heights[1]).toBe(1);
    expect(heights[6]).toBe(0);
    expect(Math.max(...heights)).toBe(1);
    const empty = { ...DEMO_WEEK, days: DEMO_WEEK.days.map((d) => ({ ...d, activeMs: 0 })) };
    expect(barHeights(empty).every((h) => h === 0)).toBe(true);
  });

  it("knows an empty week", () => {
    expect(isEmpty(DEMO_THIS_WEEK)).toBe(false);
    const zero = { ...DEMO_WEEK.total, activeMs: 0, sessions: 0, files: 0, added: 0, removed: 0, commands: 0, allowed: 0, denied: 0 };
    expect(isEmpty({ ...DEMO_WEEK, total: zero })).toBe(true);
  });

  it("names the image in plain letters", () => {
    expect(imageName(DEMO_WEEK)).toBe("Bilan ilot 2026-10-05");
  });
});

describe("the report in the panel", () => {
  it("loads last week or this week", async () => {
    await report.show("last");
    expect(report.data?.monday).toBe("2026-10-05");
    await report.show("current");
    expect(report.data?.monday).toBe("2026-10-12");
    report.close();
    expect(report.open).toBe(false);
  });

  it("asks once more before erasing the counters", async () => {
    await report.show("last");
    await report.clear();
    expect(report.confirmingClear).toBe(true);
    expect(report.data).not.toBeNull();
    await report.clear();
    expect(report.data).toBeNull();
    expect(banners.current).toMatchObject({ kind: "toast", title: "Statistiques effacées" });
    banners.queue.clear();
  });

  it("exports through the painter, and nothing happens without a file chosen", async () => {
    await report.show("last");
    report.hideProjects = true;
    const painter = vi.fn(async () => new Uint8Array([1, 2, 3]));
    await report.export(painter);
    expect(painter).toHaveBeenCalledWith(DEMO_WEEK, true);
    // Outside îlot there is no dialog: as if the user cancelled.
    expect(report.exported).toBe("cancelled");
  });
});
