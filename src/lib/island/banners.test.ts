import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  BANNER_MS,
  type Banner,
  BannerQueue,
  type Finished,
  answerToast,
  dropToast,
  durationOf,
  fileName,
  finishedText,
  formatDuration,
} from "./banners";

const done = (project: string, source: Finished["source"] = "claude-code"): Banner => ({
  kind: "finished",
  items: [{ source, project, durationMs: 240_000, added: 12, removed: 3 }],
});
const message: Banner = { kind: "message", id: "m1" };

describe("BannerQueue", () => {
  let shown: (Banner | undefined)[];
  let queue: BannerQueue;

  beforeEach(() => {
    vi.useFakeTimers();
    shown = [];
    queue = new BannerQueue((b) => shown.push(b));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows a message for 7 s then folds back", () => {
    queue.push(message);
    expect(queue.current).toBe(message);
    vi.advanceTimersByTime(BANNER_MS.message - 1);
    expect(queue.current).toBe(message);
    vi.advanceTimersByTime(1);
    expect(queue.current).toBeUndefined();
  });

  it("merges agents that finish together into one banner", () => {
    queue.push(done("epilot"));
    queue.push(done("site", "gemini-cli"));
    const current = queue.current;
    expect(current?.kind).toBe("finished");
    if (current?.kind === "finished") expect(finishedText(current.items)).toBe("2 agents ont terminé");
  });

  it("pauses while hovered and resumes with the time it had left", () => {
    queue.push(message);
    vi.advanceTimersByTime(4000);
    queue.hover(true);
    expect(queue.paused).toBe(true);
    vi.advanceTimersByTime(20_000);
    expect(queue.current).toBe(message);
    queue.hover(false);
    vi.advanceTimersByTime(BANNER_MS.message - 4000 - 1);
    expect(queue.current).toBe(message);
    vi.advanceTimersByTime(1);
    expect(queue.current).toBeUndefined();
  });

  it("waits while another shape covers it, whatever the hover does", () => {
    queue.push(message);
    queue.hold("cover", true);
    queue.hover(true);
    queue.hover(false);
    vi.advanceTimersByTime(60_000);
    expect(queue.current).toBe(message);
    queue.hold("cover", false);
    vi.advanceTimersByTime(BANNER_MS.message);
    expect(queue.current).toBeUndefined();
  });

  it("gives each new banner its full time", () => {
    queue.push(done("epilot"));
    queue.push(message);
    vi.advanceTimersByTime(3000);
    queue.dismissCurrent();
    vi.advanceTimersByTime(BANNER_MS.message - 1);
    expect(queue.current).toBe(message);
  });

  it("shows queued banners one after the other, each for its own time", () => {
    queue.push(done("epilot"));
    queue.push(message);
    vi.advanceTimersByTime(BANNER_MS.finished);
    expect(queue.current).toBe(message);
    vi.advanceTimersByTime(BANNER_MS.message);
    expect(queue.current).toBeUndefined();
    expect(shown.filter((b) => b === undefined)).toHaveLength(1);
  });

  it("gives each kind of opening its time, and a toast the time it asks for", () => {
    expect(durationOf(done("epilot"))).toBe(5000);
    expect(durationOf({ kind: "failed", failed: { source: "gemini-cli", project: "notes", detail: "npm test" } })).toBe(7000);
    expect(durationOf({ kind: "track" })).toBe(4000);
    expect(durationOf({ kind: "music" })).toBe(6000);
    expect(durationOf({ kind: "toast", tone: "ok", title: "Commande autorisée" })).toBe(2800);
    expect(durationOf({ kind: "toast", tone: "acc", title: "Chemin copié", ms: 3400 })).toBe(3400);
  });

  it("waits while the user types a reply", () => {
    queue.push(message);
    queue.hold("reply", true);
    vi.advanceTimersByTime(60_000);
    expect(queue.current).toBe(message);
    queue.hold("reply", false);
    vi.advanceTimersByTime(BANNER_MS.message);
    expect(queue.current).toBeUndefined();
  });

  it("ignores the removal of an unknown message", () => {
    queue.push(message);
    queue.removeMessage("nope");
    expect(queue.current).toBe(message);
  });

  it("drops a message whose notification went away", () => {
    queue.push(message);
    queue.removeMessage("m1");
    expect(queue.current).toBeUndefined();
  });

  it("reports every banner that leaves the screen", () => {
    const gone: Banner[] = [];
    const reporting = new BannerQueue(() => {}, (b) => gone.push(b));
    reporting.push(message);
    vi.advanceTimersByTime(BANNER_MS.message);
    reporting.push(done("epilot"));
    reporting.dismissCurrent();
    expect(gone).toEqual([message, done("epilot")]);
  });
});

describe("texts", () => {
  it("formats durations in French", () => {
    expect(formatDuration(20_000)).toBe("moins d'une minute");
    expect(formatDuration(4 * 60_000 + 59_000)).toBe("4 min");
    expect(formatDuration(65 * 60_000)).toBe("1 h 05");
  });

  it("describes one finished agent", () => {
    const banner = done("epilot");
    if (banner.kind === "finished") expect(finishedText(banner.items)).toBe("Claude a terminé · epilot · 4 min");
    expect(finishedText([{ source: "generic", name: "ci", project: "site", durationMs: 30_000, added: 0, removed: 0 }])).toBe(
      "ci a terminé · site · moins d'une minute",
    );
  });

});

describe("toasts", () => {
  it("answers a decision with what was decided, the command only for « Toujours »", () => {
    expect(answerToast("allow", "Claude Code", "npm test")).toMatchObject({ tone: "ok", title: "Commande autorisée" });
    expect(answerToast("deny", "Codex", "rm -rf build")).toMatchObject({ tone: "err", sub: "Codex est prévenu." });
    expect(answerToast("always", "Claude Code", "npm test").sub).toBe("Seulement pour : npm test");
    expect(answerToast("terminal", "Gemini CLI", "x").tone).toBe("acc");
  });

  it("names a dropped file by its last part, Windows or Linux path", () => {
    expect(fileName("C:\\Users\\HP 240\\Documents\\rapport-T3.pdf")).toBe("rapport-T3.pdf");
    expect(fileName("/home/hic/notes.md")).toBe("notes.md");
    expect(dropToast("C:\\a\\b.png").title).toBe("b.png : chemin copié");
    expect(durationOf(dropToast("x"))).toBe(3400);
  });
});
