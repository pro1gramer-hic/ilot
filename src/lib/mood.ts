// Which reaction Mochi shows when the sessions change (docs/03-DESIGN.md, "Mouvements
// et quand les jouer", "Têtes"). Pure: the mood store plays what this returns.

import type { AgentSession } from "./agents";
import { texts } from "./i18n/live.svelte";
import type { FaceId } from "./mochi/faces";
import type { MoveId } from "./mochi/moves";

export type Reaction = { move?: MoveId; face?: FaceId };

/** An agent starts → Rebondit; an agent fails → Inquiet. Nothing else changes his mood here. */
export function reactionTo(previous: readonly AgentSession[], next: readonly AgentSession[]): Reaction | null {
  const before = new Map(previous.map((s) => [s.id, s.status]));
  if (next.some((s) => s.status === "failed" && before.get(s.id) !== "failed")) return { face: "worry" };
  const started = next.some((s) => s.status === "active" && before.get(s.id) !== "active" && before.get(s.id) !== "waiting");
  return started ? { move: "bounce" } : null;
}

/** The faces of the pokes, in turn: each one says the line of the same rank. */
const POKE_FACES: readonly FaceId[] = ["happy", "wow", "love", "wink", "star", "worry", "sleep"];

/** What Mochi says (in the language of the moment) and how he looks when clicked, in turn. */
export const POKES: readonly { face: FaceId; text: string }[] = POKE_FACES.map((face, index) => ({
  face,
  get text() {
    return texts().mochi.pokes[index];
  },
}));
