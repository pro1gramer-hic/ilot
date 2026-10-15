import { describe, expect, it } from "vitest";
import { type Call, record } from "../../../tests/ui/mochi-recorder";
import { BODY_SEGMENTS, bodyOf, drawMochi, faceAnchor, outlinePoint, type FrameParams } from "./engine";
import { EYE_GAP } from "./faces";
import { KIND_IDS } from "./kinds";
import { MOVE_IDS, MOVES, OWN_MOVES } from "./moves";
import {
  type Outfit,
  PIECES,
  PIECE_IDS,
  SEASONS,
  SEASON_IDS,
  SLOTS,
  chosenOutfit,
  drawDressed,
  drawOutfit,
  halfWidthAt,
  piecesOf,
  seasonOf,
  wornOutfit,
} from "./outfits";

const day = (month: number, date: number, year = 2026): Date => new Date(year, month - 1, date, 12);

/** The calls made to draw `outfit` over a frame. */
function calls(params: FrameParams, outfit: Outfit, animated = true): Call[] {
  const { ctx, calls } = record();
  drawOutfit(ctx, params, outfit, animated);
  return calls;
}

/** Every point a drawing goes through (moveTo, lineTo, curves, arcs and ellipses' centers). */
function points(all: Call[]): [number, number][] {
  const out: [number, number][] = [];
  for (const [method, ...args] of all) {
    const n = args as number[];
    if (method === "moveTo" || method === "lineTo") out.push([n[0], n[1]]);
    else if (method === "quadraticCurveTo") out.push([n[2], n[3]]);
    else if (method === "bezierCurveTo") out.push([n[4], n[5]]);
    else if (method === "arc" || method === "ellipse" || method === "roundRect") out.push([n[0], n[1]]);
  }
  return out;
}

describe("the wardrobe", () => {
  it("has at least eight pieces, and pieces for every slot", () => {
    expect(PIECE_IDS.length).toBeGreaterThanOrEqual(8);
    expect(SLOTS.map((s) => s.label)).toEqual(["Tête", "Yeux", "Cou", "Oreilles"]);
    for (const { id } of SLOTS) expect(piecesOf(id).length, id).toBeGreaterThanOrEqual(2);
    expect(piecesOf("eyes")).toEqual(["rondes", "soleil"]);
    for (const id of PIECE_IDS) expect(PIECES[id].label, id).toMatch(/^[A-ZÉ]/);
  });

  it("keeps the pieces chosen in their own slot; « aucun » keeps a slot bare", () => {
    expect(chosenOutfit({ head: "bonnet", eyes: "soleil", neck: "", ears: "aucun" })).toEqual({ head: "bonnet", eyes: "soleil", ears: null });
    expect(chosenOutfit({ head: "soleil", eyes: "bonnet", neck: "<b>", ears: "casque" })).toEqual({ ears: "casque" });
    expect(chosenOutfit({})).toEqual({});
  });
});

describe("seasons", () => {
  it("follow the calendar", () => {
    expect(seasonOf(day(10, 11))).toBeNull();
    expect(seasonOf(day(10, 23))).toBeNull();
    expect(seasonOf(day(10, 24))).toBe("halloween");
    expect(seasonOf(day(10, 31))).toBe("halloween");
    expect(seasonOf(day(11, 1))).toBe("halloween");
    expect(seasonOf(day(11, 2))).toBeNull();
    expect(seasonOf(day(12, 1))).toBe("hiver");
    expect(seasonOf(day(12, 17))).toBe("hiver");
    expect(seasonOf(day(12, 18))).toBe("noel");
    expect(seasonOf(day(12, 25))).toBe("noel");
    expect(seasonOf(day(12, 26))).toBe("noel");
    expect(seasonOf(day(12, 27))).toBe("hiver");
    expect(seasonOf(day(12, 31))).toBe("nouvelan");
    expect(seasonOf(day(1, 1, 2027))).toBe("nouvelan");
    expect(seasonOf(day(1, 2, 2027))).toBe("hiver");
    expect(seasonOf(day(2, 28, 2027))).toBe("hiver");
    expect(seasonOf(day(2, 29, 2028))).toBe("hiver");
    expect(seasonOf(day(3, 1, 2027))).toBeNull();
    expect(seasonOf(day(3, 19))).toBeNull();
    expect(seasonOf(day(3, 20))).toBe("printemps");
    expect(seasonOf(day(5, 31))).toBe("printemps");
    expect(seasonOf(day(6, 20))).toBeNull();
    expect(seasonOf(day(6, 21))).toBe("ete");
    expect(seasonOf(day(8, 31))).toBe("ete");
    expect(seasonOf(day(9, 1))).toBeNull();
  });

  it("read the local day, at any hour", () => {
    expect(seasonOf(new Date(2026, 9, 24, 0, 0))).toBe("halloween");
    expect(seasonOf(new Date(2026, 9, 23, 23, 59))).toBeNull();
  });

  it("each dress him in their own pieces", () => {
    expect(SEASON_IDS).toEqual(["nouvelan", "noel", "halloween", "hiver", "printemps", "ete"]);
    expect(SEASONS.hiver.outfit).toEqual({ head: "bonnet", neck: "echarpe" });
    expect(SEASONS.ete.outfit).toEqual({ eyes: "soleil" });
    expect(SEASONS.halloween.outfit).toEqual({ head: "sorciere" });
    for (const id of SEASON_IDS) {
      for (const [slot, piece] of Object.entries(SEASONS[id].outfit)) expect(PIECES[piece].slot, `${id}/${piece}`).toBe(slot);
    }
  });

  it("fill only the free slots, and only while they are on", () => {
    const winter = day(1, 15);
    expect(wornOutfit({}, true, winter)).toEqual({ head: "bonnet", neck: "echarpe" });
    expect(wornOutfit({ head: "casquette" }, true, winter)).toEqual({ head: "casquette", neck: "echarpe" });
    expect(wornOutfit({ neck: null }, true, winter)).toEqual({ head: "bonnet" });
    expect(wornOutfit({ eyes: "rondes" }, false, winter)).toEqual({ eyes: "rondes" });
    expect(wornOutfit({ head: null }, false, winter)).toEqual({});
    expect(wornOutfit({}, true, day(10, 11))).toEqual({});
  });
});

describe("halfWidthAt", () => {
  it("is the body's own outline, for every silhouette", () => {
    for (const kind of KIND_IDS) {
      const body = bodyOf({ kind, rx: 7.6, ry: 6.8 });
      for (let i = 1; i < BODY_SEGMENTS / 2; i++) {
        const [x, y] = outlinePoint(bodyOf({ kind, rx: 7.6, ry: 6.8 }), i);
        expect(halfWidthAt(body, y), `${kind} point ${i}`).toBeCloseTo(Math.abs(x - body.cx), 6);
      }
      expect(halfWidthAt(body, body.top - 1)).toBe(0);
      expect(halfWidthAt(body, body.cy)).toBeCloseTo(body.rx);
    }
  });
});

describe("drawOutfit", () => {
  it("draws nothing without pieces, and keeps the canvas as it found it", () => {
    expect(calls({}, {})).toEqual([]);
    for (const piece of PIECE_IDS) {
      const all = calls({ kind: "oreilles" }, { [PIECES[piece].slot]: piece });
      expect(all.filter(([m]) => m === "save").length, piece).toBe(all.filter(([m]) => m === "restore").length);
      expect(all[0], piece).toEqual(["save"]);
      expect(all.at(-1), piece).toEqual(["restore"]);
      expect(all.some(([m]) => m === "fill" || m === "stroke"), piece).toBe(true);
    }
  });

  it("sits each hat on the head of each silhouette, and follows him up when he jumps", () => {
    for (const kind of KIND_IDS) {
      for (const piece of piecesOf("head")) {
        const ys = (params: FrameParams) => points(calls({ kind, ...params }, { head: piece })).map(([, y]) => y);
        const body = bodyOf({ kind });
        const standing = ys({});
        // Around the top of the head: from a little inside it to well above it.
        expect(Math.max(...standing), `${kind}/${piece}`).toBeLessThan(body.top + 3);
        expect(Math.min(...standing), `${kind}/${piece}`).toBeLessThan(body.top);
        const jumping = ys({ bot: 15 });
        expect(Math.max(...jumping) - Math.max(...standing), `${kind}/${piece}`).toBeCloseTo(-6, 5);
      }
    }
  });

  it("puts the glasses on the eyes, wherever the face looks", () => {
    for (const kind of KIND_IDS) {
      for (const look of [-1, 0, 1]) {
        const params: FrameParams = { kind, look, lx: 0.4, ly: -0.3 };
        const face = faceAnchor(params);
        const lenses = calls(params, { eyes: "rondes" }).filter(([m, , , r]) => m === "arc" && r === 1.75);
        expect(lenses.map(([, x, y]) => [x, y]), `${kind} look ${look}`).toEqual([
          [face.x - EYE_GAP, face.y],
          [face.x + EYE_GAP, face.y],
        ]);
      }
    }
  });

  it("slides and narrows the lenses with the eyes on a sphere", () => {
    const flat = calls({ look: 1 }, { eyes: "soleil" });
    const round = calls({ look: 1, sphere: true }, { eyes: "soleil" });
    expect(flat.filter(([m]) => m === "scale")).toHaveLength(0);
    const scales = round.filter(([m]) => m === "scale").map(([, x]) => x as number);
    expect(scales).toHaveLength(2);
    expect(scales[0]).toBeGreaterThan(scales[1]);
    expect(calls({ look: 0, sphere: true }, { eyes: "soleil" })).toEqual(calls({ look: 0 }, { eyes: "soleil" }));
  });

  it("wraps the scarf around the body below the mouth, as wide as the body there", () => {
    for (const kind of KIND_IDS) {
      const params: FrameParams = { kind, rx: 8.6, ry: 5.9 };
      const body = bodyOf(params);
      const xs = points(calls(params, { neck: "echarpe" })).map(([x]) => x);
      const row = faceAnchor(params).y + 4;
      expect(Math.max(...xs) - body.cx, kind).toBeCloseTo(halfWidthAt(body, row) + 0.25, 5);
      expect(body.cx - Math.min(...xs), kind).toBeCloseTo(halfWidthAt(body, row) + 0.25, 5);
    }
  });

  it("leans with the body when he dances", () => {
    const shear = calls({ shear: 0.12 }, { head: "bonnet", ears: "casque", eyes: "rondes" }).filter(([m]) => m === "transform");
    const bot = bodyOf({ shear: 0.12 }).bot;
    expect(shear).toEqual([
      ["transform", 1, 0, -0.12, 1, 0.12 * bot, 0],
      ["transform", 1, 0, -0.12, 1, 0.12 * bot, 0],
    ]);
  });

  it("lets the soft parts swing only when he moves", () => {
    const moving = calls({ sway: 1 }, { head: "bonnet" }, true);
    const still = calls({ sway: 1 }, { head: "bonnet" }, false);
    expect(moving).not.toEqual(still);
    expect(calls({ sway: 0 }, { head: "bonnet" }, true)).toEqual(still);
  });

  it("follows every move of every silhouette, reference and îlot's own, without failing", () => {
    const full: Outfit = { head: "sorciere", eyes: "soleil", neck: "echarpe", ears: "casque" };
    for (const kind of KIND_IDS) {
      for (const id of [...MOVE_IDS, ...OWN_MOVES]) {
        for (const t of [0, 0.37, 1.1, 2.2]) {
          const all = calls({ ...MOVES[id].pose(t), kind, sphere: true }, full);
          expect(points(all).every(([x, y]) => Number.isFinite(x) && Number.isFinite(y)), `${kind}/${id}@${t}`).toBe(true);
        }
      }
    }
  });
});

describe("drawDressed", () => {
  it("draws Mochi, then his outfit on top", () => {
    const params: FrameParams = { kind: "ecran", face: "happy" };
    const bare = record();
    drawMochi(bare.ctx, 48, params, 0);
    const dressed = record();
    drawDressed(dressed.ctx, 48, params, { neck: "noeud" }, 0, false);
    expect(dressed.calls.slice(0, bare.calls.length)).toEqual(bare.calls);
    expect(dressed.calls.length).toBeGreaterThan(bare.calls.length);
  });
});
