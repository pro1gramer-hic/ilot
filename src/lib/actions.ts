// The actions a shortcut or « OK Mochi » can run (prompt 12, lot 5), each handed to the
// registry by its owner here: open the island, go to the alert, open the chat, show the
// next pastille, cut the sounds, send Mochi to the desktop, open his wardrobe, go to the
// terminal, translate the clipboard with DeepL (stores/actions.svelte.ts).

import { texts } from "./i18n/live.svelte";
import { buddyHome, buddyShow, focusIsland, inTauri } from "./ipc";
import { island } from "./island/controller.svelte";
import { banners } from "./stores/banners.svelte";
import { bubble } from "./stores/bubble.svelte";
import { chat } from "./stores/chat.svelte";
import { pastilles } from "./stores/pastilles.svelte";
import { permissions } from "./stores/permissions.svelte";
import { actions } from "./stores/actions.svelte";
import { settings } from "./stores/settings.svelte";
import { sound } from "./stores/sound.svelte";
import { translations } from "./stores/translate.svelte";
import { goToTerminal } from "./terminal";

/** « Aller à l'alerte »: the request waiting for an answer takes the keyboard (Enter
 * allows, Escape refuses, after the usual look), or the opening on screen (or the unread
 * messages) opens what it is about. Never answers anything. */
export function goToAlert(): boolean {
  if (permissions.current) {
    if (island.panelOpen) island.tab = "agents";
    void focusIsland();
    return true;
  }
  if (banners.current || island.unread > 0) {
    island.activate();
    return true;
  }
  bubble.say(texts().mochi.say.nothingWaiting);
  return false;
}

/** « Couper le son » : the same switch as the hub's sound button. */
export function toggleMute(): void {
  const current = settings.value.sound;
  void settings.update({ sound: { ...current, muted: !current.muted } });
}

/** « Envoyer Mochi sur le bureau » : he leaves the island for the desktop, as when he is
 * pulled out in the « Sur le bureau » mode. */
export function sendMochiToDesk(): boolean {
  const name = settings.value.mochi.name;
  if (island.mochiOut) {
    bubble.say(texts().mochi.say.alreadyOut(name));
    return false;
  }
  if (!inTauri) return false;
  island.mochiOut = true;
  island.mochiDesk = true;
  void buddyHome().then((home) => buddyShow(home.x, home.y, "bureau"));
  bubble.say(texts().mochi.say.leaving(name));
  sound.play("leave");
  return true;
}

export function registerActions(): void {
  actions.provide("toggle-island", () => island.toggle());
  actions.provide("go-to-alert", () => void goToAlert());
  actions.provide("open-chat", () => chat.open());
  actions.provide("next-pastille", () => void pastilles.nextPastille());
  actions.provide("mute", toggleMute);
  actions.provide("mochi-to-desktop", () => void sendMochiToDesk());
  actions.provide("open-wardrobe", () => island.openWardrobe());
  actions.provide("go-to-terminal", () => void goToTerminal());
  // Shows the clipboard first: nothing leaves before « Traduire ».
  actions.provide("translate-clipboard", () => void translations.startClipboard());
}
