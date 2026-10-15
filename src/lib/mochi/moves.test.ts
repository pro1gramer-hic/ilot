import { describe, expect, it } from "vitest";
import { FPS } from "./clock";
import { BODY_SEGMENTS, GRID_SIZE, bodyOf, outlinePoint } from "./engine";
import { FACE_IDS } from "./faces";
import { KIND_IDS } from "./kinds";
import { ANIMATIONS, express, MOVE_IDS, MOVES, OWN_MOVES, poseAt, stillPose } from "./moves";

describe("MOVES", () => {
  it("has the ten animations of the onglet Mochi, then the island's other moves", () => {
    expect(MOVE_IDS.map((id) => MOVES[id].label)).toEqual([
      "Immobile",
      "Respire",
      "Rebondit",
      "Se balance",
      "Clic",
      "Tournis",
      "Avale",
      "Regarde",
      "Vole",
      "Tremble",
      "Dort",
      "Roule",
      "Coucou",
      "Fait la fête",
      "Fond",
    ]);
    // îlot's own moves (prompt 12) come on top of the reference's.
    expect(Object.keys(MOVES).sort()).toEqual([...MOVE_IDS, ...OWN_MOVES].sort());
    expect(ANIMATIONS.map(({ move }) => move)).toEqual(MOVE_IDS.slice(0, 10));
  });

  it("every move actually moves: its 30 frames/s cycle shows distinct frames", () => {
    for (const id of MOVE_IDS) {
      const frames = new Set<string>();
      for (let frame = 0; frame < FPS * 5; frame++) frames.add(JSON.stringify(MOVES[id].pose(frame / FPS)));
      expect(frames.size, id).toBeGreaterThan(1);
    }
  });
});

describe("express", () => {
  const pose = { rx: 9, face: "idle" as const, look: 1 };

  it("leaves the pose alone without an expression", () => {
    expect(express(pose)).toEqual(pose);
    expect(express(pose, {})).toEqual(pose);
  });

  it("an imposed face wins over big eyes and the face at rest", () => {
    expect(express(pose, { face: "wink", big: true, rest: "love" }).face).toBe("wink");
  });

  it("big eyes win over the face at rest and the move's face", () => {
    expect(express(pose, { big: true, rest: "love" }).face).toBe("big");
    expect(express({ face: "happy" }, { big: true }).face).toBe("big");
  });

  it("the face at rest replaces only the plain idle face", () => {
    expect(express(pose, { rest: "star" }).face).toBe("star");
    expect(express({}, { rest: "star" }).face).toBe("star");
    expect(express({ face: "blink" }, { rest: "star" }).face).toBe("blink");
    expect(express({ face: "happy" }, { rest: "star" }).face).toBe("happy");
  });

  it("a gaze replaces the move's own, the rest of the pose is kept", () => {
    expect(express(pose, { look: -1 })).toEqual({ ...pose, look: -1 });
    expect(express(pose, { look: 0 }).look).toBe(0);
  });
});

describe("poseAt", () => {
  it("returns the move's own pose when nothing is laid over", () => {
    expect(poseAt("bounce", 0.3)).toEqual(MOVES.bounce.pose(0.3));
  });

  it("replaces only the face when one is imposed", () => {
    for (const id of MOVE_IDS) {
      for (const face of FACE_IDS) {
        const t = 1.75;
        expect(poseAt(id, t, { face })).toEqual({ ...MOVES[id].pose(t), face });
      }
    }
  });

  it("an imposed face also replaces the move's blinks, the face at rest does not", () => {
    expect(MOVES.breathe.pose(0).face).toBe("blink");
    expect(poseAt("breathe", 0, { face: "love" }).face).toBe("love");
    expect(poseAt("breathe", 0, { rest: "love" }).face).toBe("blink");
  });
});

describe("stillPose", () => {
  it("uses the move's own face at rest", () => {
    expect(stillPose("shake")).toEqual({ face: "worry" });
    expect(stillPose("party")).toEqual({ face: "star" });
    expect(stillPose("sleep")).toEqual({ face: "sleep" });
    expect(stillPose("dizzy")).toEqual({ face: "dizzy" });
  });

  it("lays the expression over it", () => {
    expect(stillPose("shake", { face: "wink" })).toEqual({ face: "wink" });
    expect(stillPose("breathe", { rest: "love" })).toEqual({ face: "love" });
    expect(stillPose("breathe", { big: true, look: 1 })).toEqual({ face: "big", look: 1 });
  });

  it("keeps Mochi fully on screen for every move and silhouette", () => {
    for (const kind of KIND_IDS) {
      for (const id of MOVE_IDS) {
        const body = bodyOf({ ...stillPose(id), kind });
        for (let i = 0; i <= BODY_SEGMENTS; i++) {
          const [x, y] = outlinePoint(body, i);
          expect(x, `${kind}/${id}`).toBeGreaterThan(0);
          expect(x, `${kind}/${id}`).toBeLessThan(GRID_SIZE);
          expect(y, `${kind}/${id}`).toBeGreaterThan(0);
        }
        // His feet (0.85 unit tall, centered 0.35 below the body) stay above the grid's edge.
        expect(body.bot + 0.35 + 0.85, `${kind}/${id}`).toBeLessThan(GRID_SIZE);
      }
    }
  });
});
