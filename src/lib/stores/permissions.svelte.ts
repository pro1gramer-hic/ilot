// Permission requests waiting for the user. The island never answers on its own: a
// request leaves the queue only on the user's click or key, or when it expires (the
// agent then asks in its own terminal).

import { AI, type AiSource } from "../agents";
import { answerPermission, answerQuestions } from "../ipc";
import { answerToast, questionToast } from "../island/banners";
import { AnswerGuard, type PermissionAnswer } from "../island/guard";
import { NEW_DRAFT, type Question, type QuestionDraft, answerOf, pick, type } from "../questions";
import { banners } from "./banners.svelte";
import { mood } from "./mood.svelte";
import { sound } from "./sound.svelte";

export type { PermissionAnswer };

/** Answers stop a little before the relay gives up, so that a late click is never shown
 * as accepted while the agent already asks in its terminal. */
export const ANSWER_MARGIN_MS = 5_000;

/** How long the island shows an answer (the button lights up, or the card shakes) before
 * it folds. Nothing can be answered during that time: the request is already gone. */
export const ANSWER_SHOWN_MS = 420;

export type PermissionRequest = {
  id: string;
  source: AiSource;
  project: string;
  /** Short French title: "Claude veut exécuter une commande". */
  title: string;
  /** What will run or change: the exact command, a file path, a URL. */
  detail: string;
  /** What a file tool will write: lines starting with "-" (removed) or "+" (added). */
  content?: string | null;
  /** Other fields of the request that change what happens ("dangerouslyDisableSandbox: true"). */
  extras: string[];
  reason?: string | null;
  /** The agent's payload was cut because it was too long. */
  truncated: boolean;
  /** "Toujours" is offered only when a rule can match this exact request. */
  canAlways: boolean;
  /** Epoch milliseconds. */
  receivedAt: number;
  timeoutMs: number;
  /** « Risque élevé » for deletions, git push, network and system commands. */
  risk?: "low" | "high";
  /** What makes it risky, in French (« Supprime des fichiers de façon définitive. »). */
  warnings?: string[];
  /** The folder it runs in, as shown (« ~/dev/ilot »). */
  folder?: string;
  /** The agent asks questions instead (Claude's `AskUserQuestion`): one card per
   * question, answered one after the other. */
  questions?: Question[];
};

/** A request that asks questions rather than a permission. */
export function asksQuestions(request: PermissionRequest): request is PermissionRequest & { questions: Question[] } {
  return (request.questions?.length ?? 0) > 0;
}

class PermissionsStore {
  /** Oldest first; only the first one is shown. */
  queue = $state<PermissionRequest[]>([]);
  current = $derived(this.queue[0]);
  /** Requests too close to their end to be answered here (answer in the terminal). */
  late = $state<string[]>([]);
  /** The request answered a moment ago, while its answer plays on screen. */
  answered = $state<{ request: PermissionRequest; answer: PermissionAnswer } | null>(null);
  readonly guard = new AnswerGuard();
  #lateTimers = new Map<string, ReturnType<typeof setTimeout>>();
  #answeredTimer: ReturnType<typeof setTimeout> | undefined;

  add(request: PermissionRequest): void {
    if (this.queue.some((r) => r.id === request.id)) return;
    this.queue.push(request);
    const left = request.receivedAt + request.timeoutMs - ANSWER_MARGIN_MS - Date.now();
    this.#lateTimers.set(
      request.id,
      setTimeout(() => this.late.push(request.id), Math.max(0, left)),
    );
  }

  isLate(id: string): boolean {
    return this.late.includes(id);
  }

  /** The request disappears: answered, expired, or withdrawn by the agent. */
  remove(id: string): void {
    this.queue = this.queue.filter((r) => r.id !== id);
    delete this.drafts[id];
    this.guard.forget(id);
    clearTimeout(this.#lateTimers.get(id));
    this.#lateTimers.delete(id);
    this.late = this.late.filter((late) => late !== id);
  }

  /** Where the user is in each request's questions (memory only, gone with the request). */
  drafts = $state<Record<string, QuestionDraft>>({});

  draft(id: string): QuestionDraft {
    return this.drafts[id] ?? NEW_DRAFT;
  }

  /** A choice clicked on the question on screen. */
  pick(id: string, label: string): void {
    const request = this.queue.find((r) => r.id === id);
    if (!request || !asksQuestions(request)) return;
    const draft = this.draft(id);
    this.drafts[id] = pick(draft, request.questions[draft.step], label);
  }

  /** The user types an answer of their own to the question on screen. */
  type(id: string, text: string): void {
    const request = this.queue.find((r) => r.id === id);
    if (!request || !asksQuestions(request)) return;
    const draft = this.draft(id);
    this.drafts[id] = type(draft, request.questions[draft.step], text);
  }

  /** The question on screen is answered (a click on « Suivant » or « Envoyer », or Enter):
   * the next one comes and needs its own look; after the last one, every answer goes to
   * the agent through the core. Ignored (false) when it came too soon, the question was
   * not shown whole, or it has no answer yet. */
  submitQuestion(id: string): boolean {
    const request = this.queue.find((r) => r.id === id);
    if (!request || !asksQuestions(request) || request.truncated || this.isLate(id)) return false;
    if (!this.guard.ready(id, "allow", performance.now())) return false;
    const draft = this.draft(id);
    const answer = answerOf(draft, request.questions[draft.step]);
    if (!answer) return false;
    const answers = [...draft.answers, answer];
    if (answers.length < request.questions.length) {
      this.drafts[id] = { step: draft.step + 1, answers, picked: [], other: "" };
      this.guard.restart(id, performance.now());
      return true;
    }
    mood.react("allow");
    clearTimeout(this.#answeredTimer);
    this.answered = { request, answer: "allow" };
    this.#answeredTimer = setTimeout(() => (this.answered = null), ANSWER_SHOWN_MS);
    this.remove(id);
    void answerQuestions(id, answers);
    banners.queue.push(questionToast(AI[request.source].tool));
    return true;
  }

  /** The user's answer, sent to the waiting agent through the core. Ignored (false) unless
   * the request was on screen long enough, and, to allow it, shown in full. A question
   * is answered through `submitQuestion` (Enter does it), or left to the terminal
   * (Escape does it); « Toujours » means nothing for it. */
  respond(id: string, answer: PermissionAnswer): boolean {
    const request = this.queue.find((r) => r.id === id);
    if (request && asksQuestions(request)) {
      if (answer === "allow") return this.submitQuestion(id);
      if (answer === "always") return false;
      answer = "terminal";
    }
    if (!request || this.isLate(id) || !this.guard.ready(id, answer, performance.now())) return false;
    if (answer !== "terminal") {
      mood.react(answer);
      sound.play(answer === "deny" ? "deny" : "allow");
    }
    clearTimeout(this.#answeredTimer);
    this.answered = { request, answer };
    this.#answeredTimer = setTimeout(() => (this.answered = null), ANSWER_SHOWN_MS);
    this.remove(id);
    void answerPermission(id, answer);
    banners.queue.push(answerToast(answer, AI[request.source].tool, request.detail));
    return true;
  }

  clear(): void {
    for (const request of this.queue) this.remove(request.id);
    clearTimeout(this.#answeredTimer);
    this.answered = null;
  }
}

export const permissions = new PermissionsStore();
