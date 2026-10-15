// The opening on screen, if any (rules in island/banners.ts). Messages come from the
// notifications store: the opening shows its current item and tells it when it closes;
// in the « Pastille » mode they never open the island, a badge on the pill counts them.

import { untrack } from "svelte";
import { type Banner, BannerQueue, type Hold, type ToastTone } from "../island/banners";
import type { Tab } from "../island/model";
import { mood } from "./mood.svelte";
import { notifications } from "./notifications.svelte";
import { settings } from "./settings.svelte";
import { sound } from "./sound.svelte";

class BannersStore {
  current = $state<Banner | undefined>(undefined);
  /** Counts the banners shown: each one replays its entrance and its time line. */
  serial = $state(0);
  /** The time line stands still (hover, or a shape covers the banner). */
  paused = $state(false);
  readonly queue = new BannerQueue(
    (banner) => {
      // A copy: one more finished agent in the same banner still updates the screen.
      this.current = banner && { ...banner };
      this.serial += 1;
      // Each new opening plays Mochi's expressive move once, then Mochi calms down; the
      // events that come on their own have their sound (prompt 12).
      if (banner?.kind === "finished") {
        mood.playOnce("party");
        mood.hop();
        sound.play("finish");
      } else if (banner?.kind === "failed") {
        mood.playOnce("shake");
        mood.flash("worry", 1600);
        sound.play("fail");
      } else if (banner?.kind === "message") {
        mood.playOnce("peek");
        sound.play(notifications.current?.masked ? "masked" : "message");
      } else if (banner?.kind === "music" || banner?.kind === "track") {
        mood.playOnce("rock");
        if (banner.kind === "track") sound.play("track");
      }
    },
    (banner) => {
      if (banner.kind === "message") notifications.dismiss(banner.id);
    },
  );

  /** A short answer to what the user just did (« Commande autorisée »…); with `tab`, a
   * click on the island opens that tab of the panel. */
  toast(tone: ToastTone, title: string, sub?: string, ms?: number, tab?: Tab): void {
    this.queue.push({ kind: "toast", tone, title, ...(sub ? { sub } : {}), ...(ms ? { ms } : {}), ...(tab ? { tab } : {}) });
    sound.play("toast");
  }

  hold(reason: Hold, on: boolean): void {
    this.queue.hold(reason, on);
    this.paused = this.queue.paused;
  }

  constructor() {
    let shown: string | undefined;
    $effect.root(() => {
      $effect(() => {
        const id = settings.value.modes.messages === "discret" ? undefined : notifications.current?.id;
        untrack(() => {
          if (shown && shown !== id) this.queue.removeMessage(shown);
          if (id && id !== shown) this.queue.push({ kind: "message", id });
          shown = id;
        });
      });
    });
  }
}

export const banners = new BannersStore();
