// The mini-player's own window: where it sits on the screen, moved at most once a frame,
// and resized when the player turns into a video. Outside Tauri (browser preview, tests)
// the page stands for the window and nothing moves.

import { LogicalPosition, LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri } from "../../lib/ipc";
import type { Bounds, Point } from "./fling";

/** Top-left corner of the window on the screen, in logical pixels. */
export function windowAt(): Point {
  return { x: window.screenX, y: window.screenY };
}

/** Center of the window on the screen. */
function windowCenter(): Point {
  return { x: window.screenX + window.innerWidth / 2, y: window.screenY + window.innerHeight / 2 };
}

/** The part of the screen the window may use (the taskbar left out). */
export function screenBounds(): Bounds {
  const s = window.screen as Screen & { availLeft?: number; availTop?: number };
  const left = s.availLeft ?? 0;
  const top = s.availTop ?? 0;
  return { left, top, right: left + s.availWidth, bottom: top + s.availHeight };
}

let wanted: Point | null = null;
let frame = 0;

/** Moves the window; several moves in one frame make one call to the core. */
export function moveWindow(to: Point): void {
  if (!inTauri) return;
  wanted = to;
  if (frame !== 0) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    const at = wanted;
    wanted = null;
    // A refused move leaves the window where it is: nothing else to do.
    if (at) getCurrentWindow().setPosition(new LogicalPosition(Math.round(at.x), Math.round(at.y))).catch(() => {});
  });
}

/** Gives the window a new size around the same center (refused, it keeps the old one). */
export async function resizeWindow(width: number, height: number): Promise<void> {
  if (!inTauri || (window.innerWidth === width && window.innerHeight === height)) return;
  const center = windowCenter();
  const win = getCurrentWindow();
  try {
    await win.setSize(new LogicalSize(width, height));
    await win.setPosition(new LogicalPosition(Math.round(center.x - width / 2), Math.round(center.y - height / 2)));
  } catch {
    // The page keeps the player centered in whatever size the window has.
  }
}
