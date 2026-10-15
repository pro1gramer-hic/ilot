import { describe, expect, it } from "vitest";
import type { AgentSession } from "./agents";
import { DEFAULT_SETTINGS } from "./ipc";
import type { Track } from "./media";
import {
  MAX_EXTRAS,
  NO_PASTILLES,
  type PastilleSettings,
  activeIds,
  agentsIn,
  checkBaseUrl,
  formatBytes,
  lookOf,
  nextId,
  pullShare,
  relativeTime,
  spotifyView,
  stanceOf,
  tokenName,
  toolLine,
  toolProject,
  validModel,
  withExtra,
  withoutExtra,
} from "./pastilles";
import { DEMO_CATALOG, demoView, readPastillePreview, sceneSettings } from "./pastilles-preview";

const session = (project: string, status: AgentSession["status"]): AgentSession => ({
  id: `${project}-${status}`,
  source: "claude-code",
  project,
  task: "",
  status,
});

const extra = (id: string) => ({ id, baseUrl: "", account: "" });

describe("pastilles", () => {
  it("none is on by default", () => {
    expect(DEFAULT_SETTINGS.pastilles).toEqual(NO_PASTILLES);
    expect(activeIds(DEFAULT_SETTINGS.pastilles)).toEqual([]);
    expect(nextId([], null)).toBeNull();
  });

  it("the main tool comes first, then the extras in their order", () => {
    const settings: PastilleSettings = { tool: "cursor", extras: [extra("ollama"), extra("github")] };
    expect(activeIds(settings)).toEqual(["tool", "ollama", "github"]);
    expect(activeIds({ tool: "", extras: [extra("stripe")] })).toEqual(["stripe"]);
  });

  it("« changer de pastille » goes round", () => {
    const ids = ["tool", "ollama", "github"];
    expect(nextId(ids, null)).toBe("tool");
    expect(nextId(ids, "tool")).toBe("ollama");
    expect(nextId(ids, "github")).toBe("tool");
    expect(nextId(ids, "removed")).toBe("tool");
  });

  it("four extras at most, each once", () => {
    let settings: PastilleSettings = { tool: "", extras: [] };
    for (const id of ["ollama", "ollama", "github", "stripe", "spotify", "notion"]) settings = withExtra(settings, extra(id));
    expect(settings.extras.map((e) => e.id)).toEqual(["ollama", "github", "stripe", "spotify"]);
    expect(settings.extras).toHaveLength(MAX_EXTRAS);
    expect(withoutExtra(settings, "github").extras.map((e) => e.id)).toEqual(["ollama", "stripe", "spotify"]);
  });

  it("each pastille has its name and its little Mochi", () => {
    expect(lookOf("tool", "vscode", [])).toEqual({ id: "vscode", name: "VS Code", palette: "ciel", kind: "ecran" });
    expect(lookOf("ollama", "", []).name).toBe("Ollama");
    expect(lookOf("github", "", DEMO_CATALOG)).toEqual({ name: "GitHub", palette: "nuit", kind: "meche" });
    expect(lookOf("inconnu", "", []).name).toBe("inconnu");
  });

  it("the tool's project is the part of its title where an agent of îlot works", () => {
    const view = { open: true, focused: true, project: "lib.rs", candidates: ["lib.rs", "epilot", "Travail"] };
    const list = [session("epilot", "active"), session("ilot", "done"), session("epilot", "waiting")];
    expect(toolProject(view, list)).toBe("epilot");
    expect(toolProject(view, [])).toBe("lib.rs");
    expect(toolProject({ open: false, focused: false, candidates: [] }, list)).toBeNull();
    expect(toolProject(undefined, list)).toBeNull();
    expect(agentsIn("EPILOT", list)).toBe(2);
    expect(agentsIn("ilot", list)).toBe(0);
    expect(agentsIn(null, list)).toBe(0);
    expect(toolLine("VS Code", view, "epilot", 2)).toBe("VS Code sur epilot, 2 agents au travail");
    expect(toolLine("VS Code", view, "epilot", 0)).toBe("VS Code sur epilot");
    expect(toolLine("Cursor", { open: true, focused: false, candidates: [] }, null, 0)).toBe("Cursor est ouvert");
    expect(toolLine("Cursor", undefined, null, 0)).toBe("Cursor n'est pas ouvert");
  });

  it("Spotify's pastille comes from the media sessions", () => {
    const track = { title: "Nuit calme", artist: "Kairo", app: "Spotify", playing: true } as Track;
    expect(spotifyView(track)).toMatchObject({ id: "spotify", tone: "busy", line: "En lecture : Nuit calme, Kairo", short: "Nuit calme" });
    expect(spotifyView({ ...track, playing: false }).tone).toBe("idle");
    expect(spotifyView(null)).toMatchObject({ tone: "off", line: "Spotify ne joue rien" });
  });

  it("Mochi stays calm: a worried face on a failure, asleep when off", () => {
    expect(stanceOf("err")).toEqual({ move: "breathe", face: "worry" });
    expect(stanceOf("off")).toEqual({ move: "sleep" });
    expect(stanceOf("busy").move).toBe("watch");
    expect(stanceOf("ok").move).toBe("breathe");
  });

  it("times read simply", () => {
    const now = 1_000_000_000_000;
    expect(relativeTime(now - 20_000, now)).toBe("à l'instant");
    expect(relativeTime(now - 5 * 60_000, now)).toBe("il y a 5 min");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("il y a 3 h");
    expect(relativeTime(now - 26 * 3_600_000, now)).toBe("hier");
    expect(relativeTime(now - 4 * 86_400_000, now)).toBe("il y a 4 jours");
    expect(relativeTime(now + 20 * 60_000, now)).toBe("dans 20 min");
    expect(relativeTime(now + 2 * 3_600_000, now)).toBe("dans 2 h");
    expect(relativeTime(now + 30 * 3_600_000, now)).toBe("demain");
  });

  it("sizes and downloads read simply", () => {
    expect(formatBytes(6_591_830_464)).toBe("6,1 Go");
    expect(formatBytes(300 * 1024 ** 2)).toBe("300 Mo");
    expect(formatBytes(0)).toBe("0 Mo");
    expect(pullShare({ model: "m", step: "", completed: 500, total: 2000, done: false })).toBe(0.25);
    expect(pullShare({ model: "m", step: "", completed: 0, total: 0, done: false })).toBe(0);
    expect(pullShare({ model: "m", step: "", completed: 0, total: 0, done: true })).toBe(1);
    expect(pullShare({ model: "m", step: "", completed: 3, total: 9, done: true, error: "x" })).toBeCloseTo(1 / 3);
  });

  it("model names are checked before reaching the core", () => {
    expect(validModel("gemma4")).toBe(true);
    expect(validModel(" llama3.2:3b ")).toBe(true);
    expect(validModel("hf.co/user/model:Q4_K_M")).toBe(true);
    expect(validModel("")).toBe(false);
    expect(validModel("-x")).toBe(false);
    expect(validModel("../etc")).toBe(false);
    expect(validModel("a b")).toBe(false);
    expect(validModel("a;b")).toBe(false);
  });

  it("custom addresses are https, or http on this machine only", () => {
    expect(checkBaseUrl(" https://gitlab.example.com/ ")).toEqual({ url: "https://gitlab.example.com" });
    expect(checkBaseUrl("https://ghe.example.com/api/v3")).toEqual({ url: "https://ghe.example.com/api/v3" });
    expect(checkBaseUrl("http://localhost:3000")).toEqual({ url: "http://localhost:3000" });
    expect(checkBaseUrl("http://127.0.0.1:5678/")).toEqual({ url: "http://127.0.0.1:5678" });
    expect(checkBaseUrl("http://n8n.example.com")).toHaveProperty("error", expect.stringContaining("non chiffrée"));
    for (const bad of ["", "ftp://x.example", "https://u:p@x.example", "https://x.example/?t=1", "https://x.example/#a", "javascript:alert(1)", "pas une url"]) {
      expect(checkBaseUrl(bad), bad).toHaveProperty("error");
    }
  });

  it("tokens live in the vault under service: names", () => {
    expect(tokenName("github")).toBe("service:github");
  });

  it("the preview's readings never carry a token", () => {
    const now = Date.now();
    for (const id of ["tool", "ollama", "lmstudio", "github", "stripe", "vercel", "dokploy", "n8n"]) {
      expect(JSON.stringify(demoView(id, now)).toLowerCase()).not.toContain("token");
    }
    expect(sceneSettings("none")).toEqual({ tool: "", extras: [] });
    expect(readPastillePreview("?pastilles=pull&modes=off")).toEqual({ pastilles: "pull", playModes: false });
    expect(readPastillePreview("")).toEqual({ pastilles: "none", playModes: true });
  });
});
