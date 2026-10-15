import { describe, expect, it } from "vitest";
import { record } from "../../../tests/ui/mochi-recorder";
import { KIND_IDS } from "./kinds";
import { MOVES } from "./moves";
import { INK } from "./palettes";
import { BOX_BEATS, POSES, boxAt, drawBox, drawThought, framePoseAt, isPose, stillFramePose } from "./poses";

describe("the box that swallows a dropped file", () => {
  it("plays its story once: Mochi becomes a box, opens, a sheet falls in, it closes and gulps", () => {
    expect(boxAt(0).box).toMatchObject({ morph: 0, open: 0, sheet: -1, squash: 1 });
    expect(boxAt(0).face).toBe("wow");
    const opened = boxAt(BOX_BEATS.open);
    expect(opened.box.morph).toBeCloseTo(1, 2);
    expect(opened.box.open).toBeGreaterThan(0.9);
    expect(opened.face).toBe("eat");
    const falling = boxAt((BOX_BEATS.open + BOX_BEATS.fall) / 2);
    expect(falling.box.sheet).toBeGreaterThan(0);
    expect(falling.box.sheet).toBeLessThan(1);
    const gulping = boxAt((BOX_BEATS.close + BOX_BEATS.gulp) / 2);
    expect(gulping.box.open).toBeLessThan(0.05);
    expect(gulping.box.squash).toBeLessThan(0.95);
    expect(gulping.face).toBe("happy");
  });

  it("then rests closed, breathing a little, and blinks now and then", () => {
    for (const t of [2, 3.5, 7, 30]) {
      const { box } = boxAt(t);
      expect(box.open).toBeLessThan(0.01);
      expect(box.sheet).toBe(1);
      expect(Math.abs(box.squash - 1)).toBeLessThan(0.02);
    }
    const faces = new Set(Array.from({ length: 120 }, (_, i) => boxAt(2 + i / 30).face));
    expect(faces).toEqual(new Set(["happy", "blink"]));
  });

  it("is drawn for the three silhouettes, with the face on its front and the sheet only while it falls", () => {
    for (const kind of KIND_IDS) {
      const { ctx, calls } = record();
      drawBox(ctx, 48, { ...POSES.box.pose(0.8), kind, palette: "menthe" });
      expect(calls.length).toBeGreaterThan(40);
      expect(calls.filter(([method, value]) => method === "fillStyle =" && value === INK.glint).length).toBeGreaterThan(0);
      const rest = record();
      drawBox(rest.ctx, 48, { ...POSES.box.pose(5), kind });
      const sheets = rest.calls.filter(([method]) => method === "lineTo").length;
      expect(sheets).toBeLessThan(calls.filter(([method]) => method === "lineTo").length);
    }
  });

  it("keeps its own faces: the island's expressions never reach it", () => {
    const frame = framePoseAt("box", 0.4, { face: "worry", big: true });
    expect(frame.face).toBe("eat");
    expect(stillFramePose("box").box).toEqual({ morph: 1, open: 0, sheet: -1, squash: 1 });
  });
});

describe("Mochi thinking while a model answers", () => {
  it("looks up toward three dots that light one after the other", () => {
    const lit = Array.from({ length: 16 }, (_, i) => POSES.think.pose(i * 0.1).thought);
    expect(new Set(lit)).toEqual(new Set([0, 1, 2, 3]));
    const frame = POSES.think.pose(0.5);
    expect(frame.look).toBe(1);
    expect(frame.ly).toBeLessThan(0);
  });

  it("draws the lit dots only, the newest the brightest", () => {
    const { ctx, calls } = record();
    drawThought(ctx, 2);
    expect(calls.filter(([method]) => method === "arc").length).toBe(2);
    const colors = calls.filter(([method]) => method === "fillStyle =").map(([, value]) => value);
    expect(colors.at(-1)).toBe(INK.glint);
  });

  it("takes an imposed face, and still dots with reduced motion", () => {
    expect(framePoseAt("think", 0.5, { face: "wow" }).face).toBe("wow");
    expect(stillFramePose("think").thought).toBe(3);
  });
});

describe("any move", () => {
  it("the reference's moves go through untouched", () => {
    expect(isPose("breathe")).toBe(false);
    expect(isPose("think")).toBe(true);
    expect(framePoseAt("bounce", 0.3)).toEqual(MOVES.bounce.pose(0.3));
    expect(stillFramePose("sleep")).toEqual({ face: "sleep" });
  });
});
