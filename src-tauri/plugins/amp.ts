// Installed by îlot: tells îlot what Amp does (https://ampcode.com/docs/customize/plugins).
// ilot-plugin-revision: 1
// Removed when Amp is disconnected in îlot's settings. It only reports: it listens to no
// `tool.call` (where a plugin decides whether a tool runs) and never waits for îlot.

import { spawn } from "node:child_process";

/** îlot's relay, as Amp sees it (îlot writes the path when it installs this file). */
const RELAY = "__ILOT_RELAY__";

type Payload = Record<string, unknown>;
type Thread = { thread: { id: string } };
type ToolResult = Thread & { tool: string; input?: unknown; status: "done" | "error" | "cancelled"; error?: string };
type Uri = { fsPath?: string; path?: string };

/** The part of Amp's plugin API îlot uses (https://ampcode.com/docs/plugin-api). */
interface PluginAPI {
  on(event: "session.start", handler: (event: Thread) => Promise<void>): void;
  on(event: "agent.start", handler: (event: Thread & { message: string }) => Promise<void>): void;
  on(event: "tool.result", handler: (event: ToolResult) => Promise<void>): void;
  on(event: "agent.end", handler: (event: Thread & { status: "done" | "error" | "cancelled" }) => Promise<void>): void;
  system?: { workspaceRoot?: Uri | null };
}

/** Hands one event to îlot's relay in the background: whatever happens, Amp goes on. */
function send(event: string, payload: Payload): void {
  try {
    const child = spawn(RELAY, ["amp", event], { stdio: ["pipe", "ignore", "ignore"], windowsHide: true });
    child.on("error", () => {});
    child.stdin?.on("error", () => {});
    child.stdin?.end(JSON.stringify(payload));
  } catch {
    // The relay cannot start (îlot removed): nothing to report.
  }
}

export default function (amp: PluginAPI): void {
  const where = (event: Thread): Payload => {
    const root = amp.system?.workspaceRoot;
    return { session_id: event.thread.id, cwd: root?.fsPath ?? root?.path ?? "" };
  };
  amp.on("session.start", async (event) => send("SessionStart", where(event)));
  amp.on("agent.start", async (event) => send("UserPromptSubmit", { ...where(event), prompt: event.message }));
  amp.on("tool.result", async (event) => {
    if (event.status === "cancelled") return;
    const name = event.status === "error" ? "PostToolUseFailure" : "PostToolUse";
    send(name, { ...where(event), tool_name: event.tool, tool_input: event.input, error: event.error });
  });
  amp.on("agent.end", async (event) => {
    send(event.status === "done" ? "Stop" : event.status === "error" ? "StopFailure" : "Interrupt", where(event));
  });
}
