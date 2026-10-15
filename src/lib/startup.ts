// Wires the interface to the Rust core (or to the browser preview) and to the keyboard.

import { toStore } from "svelte/store";
import { registerActions } from "./actions";
import { isTextField } from "./keyboard";
import { dotKey, newlyFailed } from "./agents";
import { texts, textsLocale } from "./i18n/live.svelte";
import { agentsSnapshot, buddyRestore, copyText, fileAttach, focusIsland, inTauri, islandPlacement, onCoreEvent, takingFocus } from "./ipc";
import { dropToast } from "./island/banners";
import { island } from "./island/controller.svelte";
import { distanceTo, islandBox } from "./island/model";
import { reactionTo } from "./mood";
import { stillness } from "./motion.svelte";
import { bubble } from "./stores/bubble.svelte";
import { changes } from "./stores/changes.svelte";
import { history } from "./stores/history.svelte";
import { WorkClock } from "./worktime";
import { startPreview } from "./preview";
import { banners } from "./stores/banners.svelte";
import { chat } from "./stores/chat.svelte";
import { decisions } from "./stores/decisions.svelte";
import { media } from "./stores/media.svelte";
import { mood } from "./stores/mood.svelte";
import { notifications } from "./stores/notifications.svelte";
import { pastilles } from "./stores/pastilles.svelte";
import { asksQuestions, permissions } from "./stores/permissions.svelte";
import { rest } from "./stores/rest.svelte";
import { sessions } from "./stores/sessions.svelte";
import { settings } from "./stores/settings.svelte";
import { sound } from "./stores/sound.svelte";
import { timeline } from "./stores/timeline.svelte";
import { actions } from "./stores/actions.svelte";
import { report } from "./stores/report.svelte";
import { voice } from "./stores/voice.svelte";

/** True when the keystroke targets a control that handles Enter itself. */
function inControl(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && target.closest("button, input, textarea, select, a") !== null;
}

/** Enter allows and Escape refuses the request on screen; Escape also closes the panel,
 * the quick menu and the hub. A held key repeats: only a fresh press answers, and the
 * permission store ignores answers given too soon or to a request not shown in full. */
function onKeydown(event: KeyboardEvent): void {
  const request = island.answering;
  if (event.key === "Escape") {
    if (island.panelOpen) island.closePanel();
    else if (island.mode === "menu") island.closeMenu();
    else if (island.mode === "wardrobe") island.closeWardrobe();
    else if (island.hubOpen) island.closeHub();
    else if (request?.surface === "island" && !event.repeat) permissions.respond(request.id, "deny");
  } else if (event.key === "Enter" && request && !event.repeat && !inControl(event.target)) {
    event.preventDefault();
    permissions.respond(request.id, "allow");
  }
}

/** Loops (comet, record, breath, hints) stop while the island cannot be seen. */
function followVisibility(): void {
  const update = (): void => {
    stillness.current = document.hidden;
    document.documentElement.toggleAttribute("data-still", document.hidden);
  };
  document.addEventListener("visibilitychange", update);
  update();
}

/** The page says the language it speaks (`<html lang>`: screen readers, hyphenation), and
 * follows it when it changes in Réglages. */
function followLanguage(): void {
  toStore(() => textsLocale()).subscribe((locale) => {
    document.documentElement.lang = locale;
  });
}

/** A file dragged this close to the island opens the drop zone. */
const DROP_REACH_PX = 80;

/** A file dropped on the island: its path is copied for the agent, and Mochi turns into a
 * box that swallows it, then offers to ask about it or to mail it. Nothing reads the file
 * here: it is opened only if the user sends a question about it (src-tauri/src/attachments.rs). */
async function dropped(path: string): Promise<void> {
  void copyText(path);
  sound.play("swallow");
  island.kick(-100);
  const attachment = await fileAttach(path).catch(() => undefined);
  if (attachment) {
    banners.queue.push({ kind: "file", attachment });
  } else {
    mood.playOnce("swallow");
    mood.flash("eat", 1400);
    banners.queue.push(dropToast(path));
  }
}

/** Files dragged from Windows (Tauri: the webview's drag and drop events, positions in
 * physical pixels) or, in the browser preview, from the page. */
async function watchDrops(): Promise<void> {
  const near = (x: number, y: number): boolean =>
    island.mode !== "permission" && distanceTo(islandBox(island.size, island.placement), x, y) < DROP_REACH_PX;
  if (!inTauri) {
    window.addEventListener("dragover", (event) => {
      if (!event.dataTransfer?.types.includes("Files")) return;
      event.preventDefault();
      island.dropping = near(event.clientX, event.clientY);
    });
    window.addEventListener("dragleave", (event) => {
      if (event.relatedTarget === null) island.dropping = false;
    });
    window.addEventListener("drop", (event) => {
      event.preventDefault();
      const file = event.dataTransfer?.files[0];
      if (island.dropping && file) void dropped(file.name);
      island.dropping = false;
    });
    return;
  }
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  await getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === "leave") {
      island.dropping = false;
      return;
    }
    const at = payload.position.toLogical(window.devicePixelRatio);
    const over = near(at.x, at.y);
    if (payload.type === "drop") {
      if (island.dropping && over && payload.paths[0]) void dropped(payload.paths[0]);
      island.dropping = false;
    } else {
      island.dropping = over;
    }
  });
}

/** After 2 h of agents at work, Mochi suggests a break once ("Fond" in the panel). */
function suggestBreaks(): void {
  const clock = new WorkClock();
  setInterval(() => {
    if (!clock.update(Date.now(), sessions.liveCount > 0)) return;
    bubble.say(texts().mochi.say.takeBreak, 8000);
    if (island.panelOpen) mood.playOnce("melt");
    else mood.flash("sleep", 4000);
  }, 60_000);
}

/** Last week's report is offered once on Monday: checked at launch, then every hour. */
const MONDAY_CHECK_MS = 60 * 60 * 1000;

export async function start(): Promise<void> {
  media.ondetach = () => sound.play("detach");
  window.addEventListener("keydown", onKeydown);
  followLanguage();
  followVisibility();
  registerActions();
  voice.follow();
  // The « Enregistrer sous » dialog of the report image takes the focus: the panel stays.
  window.addEventListener("blur", () => {
    if (takingFocus()) return;
    if (report.exported !== "drawing") island.blur();
  });
  // A click in one of the island's text fields gives its window the keyboard.
  window.addEventListener(
    "pointerdown",
    (event) => {
      const field = event.target;
      if (!isTextField(field)) return;
      void focusIsland(true).then(() => (field as HTMLElement).focus());
    },
    true,
  );
  window.addEventListener("focus", () => (island.focused = true));
  island.focused = document.hasFocus();
  void watchDrops();
  // Streamed answers, windows Mochi lands on, dictation (in the preview too: emitPreview).
  chat.connect();
  // « Transparente au repos » (in the preview too).
  rest.follow();
  if (!inTauri) {
    startPreview(location.search);
    return;
  }
  onCoreEvent("island:pointer", (pointer) => (island.pointer = pointer));
  onCoreEvent("island:placed", (placement) => (island.placement = placement));
  onCoreEvent("island:drag", ({ dragging }) => (island.dragging = dragging));
  onCoreEvent("buddy:returned", () => {
    island.mochiOut = false;
    island.mochiDesk = false;
    island.kick(-110);
    mood.flash("annoyed", 450);
    sound.play("back");
  });
  onCoreEvent("island:wardrobe", () => island.openWardrobe());
  onCoreEvent("island:toggle", () => island.toggle());
  onCoreEvent("shortcut:action", (action) => actions.run(action));
  onCoreEvent("voice:heard", (heard) => voice.heard(heard));
  onCoreEvent("voice:state", ({ listening }) => (voice.listening = listening));
  onCoreEvent("island:open-settings", () => island.openPanel("modes"));
  onCoreEvent("settings:changed", (value) => settings.apply(value));
  onCoreEvent("sessions:changed", (list) => {
    const reaction = reactionTo(sessions.list, list);
    for (const failed of newlyFailed(sessions.list, list)) {
      island.flash(dotKey(failed), "err");
      banners.queue.push({
        kind: "failed",
        failed: {
          source: failed.source,
          ...(failed.name ? { name: failed.name } : {}),
          ...(failed.agent ? { agent: failed.agent } : {}),
          project: failed.project,
          detail: failed.task,
        },
      });
    }
    timeline.record(sessions.list, list);
    sessions.list = list;
    if (reaction?.face === "worry") history.recordFailure();
    if (reaction?.face) mood.flash(reaction.face, 4000);
    // Expressive moves only play where there is room: the panel (the pill stays calm).
    if (reaction?.move && island.panelOpen) mood.playOnce(reaction.move);
  });
  onCoreEvent("permission:added", (request) => {
    permissions.add(request);
    sound.play("approval");
    // A question is no permission: no mark on the timeline.
    if (!asksQuestions(request)) timeline.mark(request.source, "permission", request.detail);
  });
  onCoreEvent("decision:added", (decision) => {
    decisions.add(decision);
    timeline.decide(decision);
  });
  onCoreEvent("agents:suspended", ({ on }) => (decisions.suspended = on));
  onCoreEvent("permission:removed", ({ id }) => permissions.remove(id));
  onCoreEvent("diff:changed", ({ id }) => changes.changed(id));
  onCoreEvent("agent:finished", (finished) => {
    history.recordFinished(finished);
    island.flash(dotKey(finished), "ok");
    timeline.mark(finished.source, "done", `${finished.project}, +${finished.added} −${finished.removed}`);
    if (settings.value.modes.done === "discret" || !settings.value.announceFinished) {
      // « Discret »: a green dot and a jump, no opening.
      mood.flash("star", 1600);
      mood.hop();
      island.kick(-150);
      sound.play("jump");
    } else {
      banners.queue.push({ kind: "finished", items: [finished] });
    }
  });
  onCoreEvent("notifications:incoming", (incoming) => {
    notifications.receive(incoming);
    // « Pastille »: no opening, a quiet tick for the badge.
    if (settings.value.modes.messages === "discret") sound.play("masked");
  });
  onCoreEvent("notifications:status", (status) => (notifications.status = status));
  const placement = await islandPlacement();
  if (placement) island.placement = placement;
  await Promise.all([
    settings.load(),
    media.connect(),
    notifications.refreshStatus(),
    decisions.load(),
    sound.refresh(),
    pastilles.connect(),
  ]);
  const snapshot = await agentsSnapshot();
  timeline.record(sessions.list, snapshot.sessions);
  sessions.list = snapshot.sessions;
  for (const request of snapshot.permissions) permissions.add(request);
  decisions.suspended = snapshot.suspended;
  suggestBreaks();
  // Left on the desktop last time, Mochi is back there (prompt 12, « Sur le bureau »).
  if (await buddyRestore().catch(() => false)) {
    island.mochiOut = true;
    island.mochiDesk = true;
  }
  void report.offerMonday();
  setInterval(() => void report.offerMonday(), MONDAY_CHECK_MS);
  // Mochi wakes up in the hub and waves, then the island folds back unless it is used
  // meanwhile; with a permission on screen, he waves in the pill once it is answered.
  if (island.mode !== "permission") island.openHub({ intro: "launch" });
  else island.greeting = true;
}
