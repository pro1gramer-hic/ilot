// The island at rest (rules in island/rest.ts): with « Transparente au repos » it fades
// after a few quiet seconds and comes back on hover or on the first event; in any case,
// quiet a little longer, its webview is asked for little memory, and for its normal
// memory again as soon as something goes on.

import { untrack } from "svelte";
import { islandMemory } from "../ipc";
import { island } from "../island/controller.svelte";
import { MEMORY_AFTER_MS, REST_AFTER_MS, isQuiet } from "../island/rest";
import { media } from "./media.svelte";
import { permissions } from "./permissions.svelte";
import { sessions } from "./sessions.svelte";
import { settings } from "./settings.svelte";

class RestStore {
  /** The webview was asked for little memory (the preview shows it for its tests). */
  memoryLow = $state(false);
  #fadeTimer: ReturnType<typeof setTimeout> | undefined;
  #memoryTimer: ReturnType<typeof setTimeout> | undefined;

  /** Starts following what goes on (startup.ts); returns what stops it. */
  follow(): () => void {
    return $effect.root(() => {
      $effect(() => {
        const quiet = isQuiet({
          mode: island.mode,
          hubOpen: island.hubOpen,
          panelOpen: island.panelOpen,
          working: sessions.list.some((s) => s.status === "active" || s.status === "waiting"),
          playing: media.shown?.playing === true,
          waiting: permissions.current !== undefined,
          handled: island.dragging || island.pull > 0 || island.dropping,
          roaming: island.mochiOut && !island.mochiDesk,
          near: island.pointer.near || island.pointer.inside || island.hovered,
        });
        const fade = settings.value.display.fadeWhenIdle;
        untrack(() => this.#follow(quiet, fade));
      });
    });
  }

  #follow(quiet: boolean, fade: boolean): void {
    clearTimeout(this.#fadeTimer);
    clearTimeout(this.#memoryTimer);
    if (!quiet) {
      island.resting = false;
      if (this.memoryLow) this.#memory(false);
      return;
    }
    if (!fade) island.resting = false;
    else if (!island.resting) this.#fadeTimer = setTimeout(() => (island.resting = true), REST_AFTER_MS);
    if (!this.memoryLow) this.#memoryTimer = setTimeout(() => this.#memory(true), MEMORY_AFTER_MS);
  }

  #memory(low: boolean): void {
    this.memoryLow = low;
    void islandMemory(low);
  }
}

export const rest = new RestStore();
