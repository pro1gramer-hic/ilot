// Installed by îlot: tells îlot what OpenCode does (https://opencode.ai/docs/plugins).
// ilot-plugin-revision: 1
// Removed when OpenCode is disconnected in îlot's settings. It only reports: it never
// answers a permission, never changes a tool's arguments and never waits for îlot.

import { spawn } from "node:child_process";

/** îlot's relay, as OpenCode sees it (îlot writes the path when it installs this file). */
const RELAY = "__ILOT_RELAY__";

type Payload = Record<string, unknown>;
type Session = { id: string; parentID?: string; directory?: string; title?: string };
type BusEvent = {
  type: string;
  properties?: { info?: Session; sessionID?: string; title?: string; error?: { name?: string } };
};
type Part = { type?: string; text?: string };

/** Hands one event to îlot's relay in the background: whatever happens, OpenCode goes on. */
function send(event: string, payload: Payload): void {
  try {
    const child = spawn(RELAY, ["opencode", event], { stdio: ["pipe", "ignore", "ignore"], windowsHide: true });
    child.on("error", () => {});
    child.stdin?.on("error", () => {});
    child.stdin?.end(JSON.stringify(payload));
  } catch {
    // The relay cannot start (îlot removed): nothing to report.
  }
}

export const IlotPlugin = async ({ directory }: { directory: string }) => {
  /** Subagent sessions (`parentID`) and the session they work for. */
  const parents = new Map<string, string>();
  const where = (sessionID: string | undefined): Payload => {
    const parent = sessionID === undefined ? undefined : parents.get(sessionID);
    return parent ? { session_id: parent, cwd: directory, agent_id: sessionID } : { session_id: sessionID, cwd: directory };
  };
  return {
    event: async ({ event }: { event: BusEvent }) => {
      const properties = event.properties ?? {};
      const info = properties.info;
      const id = properties.sessionID;
      switch (event.type) {
        case "session.created":
          if (info?.parentID) {
            parents.set(info.id, info.parentID);
            send("SubagentStart", { session_id: info.parentID, cwd: directory, agent_id: info.id, agent_type: info.title });
          } else if (info) {
            send("SessionStart", { session_id: info.id, cwd: info.directory ?? directory });
          }
          break;
        case "session.idle":
          if (id !== undefined && parents.has(id)) {
            send("SubagentStop", { ...where(id) });
            parents.delete(id);
          } else {
            send("Stop", where(id));
          }
          break;
        case "session.error":
          if (id !== undefined && !parents.has(id)) send("StopFailure", { ...where(id), error: properties.error?.name ?? "error" });
          break;
        case "session.deleted":
          if (info && !info.parentID) send("SessionEnd", { session_id: info.id, cwd: directory });
          break;
        case "permission.updated":
        case "permission.asked":
          send("Notification", { ...where(id), notification_type: "permission_prompt", message: properties.title ?? "" });
          break;
      }
    },
    "chat.message": async (input: { sessionID: string }, output: { parts?: Part[] }) => {
      if (parents.has(input.sessionID)) return;
      const prompt = output.parts?.find((part) => part.type === "text")?.text;
      send("UserPromptSubmit", { ...where(input.sessionID), prompt });
    },
    "tool.execute.before": async (input: { tool: string; sessionID: string }, output: { args: unknown }) => {
      send("PreToolUse", { ...where(input.sessionID), tool_name: input.tool, tool_input: output.args });
    },
    "tool.execute.after": async (input: { tool: string; sessionID: string; args?: unknown }) => {
      send("PostToolUse", { ...where(input.sessionID), tool_name: input.tool, tool_input: input.args });
    },
  };
};
