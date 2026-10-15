// « Aller au terminal »: the core brings the window where a session runs to the
// foreground (src-tauri/src/terminal.rs) and says how close it got; the island says it
// in a toast. Without a session, the one that matters most (waiting, then at work).

import { type TerminalOutcome, goToTerminal as askCore } from "./ipc";
import type { Toast } from "./island/banners";
import { banners } from "./stores/banners.svelte";

/** The toast that tells what happened, honestly (« onglet non identifié »). */
export function terminalToast(outcome: TerminalOutcome): Toast {
  const tone = !outcome.found ? "err" : outcome.exact ? "ok" : "acc";
  return { kind: "toast", tone, title: outcome.message, ms: 3400 };
}

/** Brings the session's window forward (the shortcut of « Aller au terminal » calls it too). */
export async function goToTerminal(sessionId?: string): Promise<TerminalOutcome> {
  const outcome = await askCore(sessionId);
  banners.queue.push(terminalToast(outcome));
  return outcome;
}
