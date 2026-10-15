import { describe, expect, it } from "vitest";
import { hasHiddenCharacters, looksSpoofed, reveal } from "./reveal";

describe("reveal", () => {
  it("leaves a plain command untouched", () => {
    expect(reveal("npm run build")).toEqual([{ kind: "text", value: "npm run build" }]);
    expect(hasHiddenCharacters("rm -rf ./dist && echo «ok» é")).toBe(false);
  });

  it("shows line breaks", () => {
    expect(reveal("echo a\nrm -rf ~")).toEqual([
      { kind: "text", value: "echo a" },
      { kind: "newline" },
      { kind: "text", value: "rm -rf ~" },
    ]);
  });

  it("names bidi overrides that reorder what the eye reads", () => {
    expect(reveal("ls ‮txt.exe")).toEqual([
      { kind: "text", value: "ls " },
      { kind: "hidden", label: "RLO U+202E" },
      { kind: "text", value: "txt.exe" },
    ]);
  });

  it("flags zero-width, control and look-alike space characters", () => {
    expect(reveal("a​b").map((s) => s.kind)).toEqual(["text", "hidden", "text"]);
    expect(reveal("\u001b[2J")[0]).toEqual({ kind: "hidden", label: "ESC U+001B" });
    expect(reveal("rm -rf")[1]).toEqual({ kind: "hidden", label: "NBSP U+00A0" });
    expect(reveal("x\rY")[1]).toEqual({ kind: "hidden", label: "CR U+000D" });
    expect(reveal("\t")[0]).toEqual({ kind: "hidden", label: "TAB U+0009" });
  });

  it("flags fillers and blank look-alikes that pass for nothing", () => {
    for (const char of ["\u3164", "\u2800", "\u034f", "\u180b", "\u{1D173}", "\u115f", "\uffa0", "\u17b4"]) {
      expect(hasHiddenCharacters(`rm${char}x`), char).toBe(true);
    }
  });

  it("spots words that mix alphabets, and stacked marks", () => {
    expect(looksSpoofed("curl https://gіthub.com/x")).toBe(true);
    expect(looksSpoofed("rm fi\u0301\u0301le")).toBe(true);
    expect(looksSpoofed("echo café")).toBe(false);
    expect(looksSpoofed("echo привет world")).toBe(false);
  });

  it("flags Unicode tag characters used to smuggle text", () => {
    expect(reveal("ok\u{E0041}")[1]).toEqual({ kind: "hidden", label: "U+E0041" });
    expect(hasHiddenCharacters("ok\u{E0041}")).toBe(true);
  });
});
