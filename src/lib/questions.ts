// Questions an agent asks the user (Claude Code's `AskUserQuestion`), one at a time in
// the island: what the user picked on the question on screen, and the answer it makes.
// The core checks every answer again against the questions (questions.rs).

import { texts } from "./i18n/live.svelte";
import type { QuestionAnswer } from "./ipc";

/** One choice of a question. */
export type Choice = { label: string; description?: string };

/** A question as the core sends it (src-tauri/src/connectors/questions.rs). */
export type Question = { question: string; header: string; options: Choice[]; multiSelect: boolean; freeText: boolean };

/** Longest typed answer (`MAX_OTHER_CHARS` in questions.rs). */
export const MAX_OTHER_CHARS = 500;

/** Where the user is in a request's questions: those answered, and the one on screen. */
export type QuestionDraft = { step: number; answers: QuestionAnswer[]; picked: string[]; other: string };

export const NEW_DRAFT: QuestionDraft = { step: 0, answers: [], picked: [], other: "" };

/** A choice clicked: a simple choice replaces the other ones (and the typed answer), a
 * multiple one is added or removed. */
export function pick(draft: QuestionDraft, question: Question, label: string): QuestionDraft {
  if (!question.options.some((o) => o.label === label)) return draft;
  if (!question.multiSelect) return { ...draft, picked: [label], other: "" };
  const picked = draft.picked.includes(label) ? draft.picked.filter((l) => l !== label) : [...draft.picked, label];
  return { ...draft, picked };
}

/** Typing an answer of one's own: for a simple choice, it replaces the choice picked. */
export function type(draft: QuestionDraft, question: Question, text: string): QuestionDraft {
  if (!question.freeText) return draft;
  const other = text.slice(0, MAX_OTHER_CHARS);
  return question.multiSelect || other.trim() === "" ? { ...draft, other } : { ...draft, picked: [], other };
}

/** The answer the question on screen makes, or `null` while it has none. */
export function answerOf(draft: QuestionDraft, question: Question): QuestionAnswer | null {
  const other = question.freeText ? draft.other.trim() : "";
  const picked = question.options.map((o) => o.label).filter((label) => draft.picked.includes(label));
  const count = picked.length + (other ? 1 : 0);
  if (count === 0 || (!question.multiSelect && count > 1)) return null;
  return other ? { picked, other } : { picked };
}

/** « Question 2 sur 3 », or nothing for a single question. */
export function stepLabel(step: number, total: number): string {
  return total > 1 ? texts().agents.questionStep(step + 1, total) : "";
}
