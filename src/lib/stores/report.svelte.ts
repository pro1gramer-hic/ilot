// The Monday report in the panel's « Aujourd'hui » tab: which week it shows, project names
// hidden or not, its export to a 1080 × 1920 image, and the Monday offer (once, when last
// week has something to tell). Counters come from the core; nothing here is kept.

import { texts } from "../i18n/live.svelte";
import { clearStats, saveReportImage, takeMondayReport, weekReport } from "../ipc";
import { type ReportWeek, type WeekReport, imageName } from "../report";
import { banners } from "./banners.svelte";

export type ExportState = "idle" | "drawing" | "saved" | "cancelled" | "error";

/** Draws the image; report-image.ts in the app, a stand-in in the tests. */
export type Painter = (report: WeekReport, hideProjects: boolean) => Promise<Uint8Array>;

class ReportStore {
  /** The report replaces the day's view in the tab. */
  open = $state(false);
  week = $state<ReportWeek>("last");
  data = $state.raw<WeekReport | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  hideProjects = $state(false);
  exported = $state<ExportState>("idle");
  /** A click on « Effacer » asks once more before the counters go. */
  confirmingClear = $state(false);

  async show(week: ReportWeek = this.week): Promise<void> {
    this.open = true;
    this.week = week;
    this.exported = "idle";
    this.confirmingClear = false;
    this.loading = true;
    try {
      this.data = await weekReport(week);
      this.error = null;
    } catch (error) {
      this.data = null;
      this.error = String(error);
    } finally {
      this.loading = false;
    }
  }

  close(): void {
    this.open = false;
    this.confirmingClear = false;
  }

  /** « Effacer mes statistiques »: first click asks, second one erases. */
  async clear(): Promise<void> {
    if (!this.confirmingClear) {
      this.confirmingClear = true;
      return;
    }
    this.confirmingClear = false;
    try {
      await clearStats();
      this.data = null;
      banners.toast("ok", texts().timeline.report.cleared.title, texts().timeline.report.cleared.sub);
    } catch (error) {
      this.error = String(error);
    }
  }

  /** Draws the image and lets the user save it where they want. */
  async export(paint: Painter): Promise<void> {
    if (!this.data || this.exported === "drawing") return;
    this.exported = "drawing";
    try {
      const png = await paint(this.data, this.hideProjects);
      this.exported = (await saveReportImage(png, imageName(this.data))) ? "saved" : "cancelled";
    } catch (error) {
      this.exported = "error";
      this.error = String(error);
    }
  }

  /** On Monday, once: the island says last week's report is ready and opens it on a click. */
  async offerMonday(): Promise<void> {
    if (!(await takeMondayReport())) return;
    this.week = "last";
    this.open = true;
    banners.toast("acc", texts().timeline.report.ready.title, texts().timeline.report.ready.sub, 6000, "today");
  }
}

export const report = new ReportStore();
