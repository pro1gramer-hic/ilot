<!--
  Mochi out of the island, in his own small window (src-tauri/src/buddy.rs). Ported from
  `roamOut`, `wander`, `pickSpot`, `hopTo`, `heldMove` and `buddyHome` in
  reference/ilot-demo-finale.html. Positions are Mochi's center on the screen, in the
  core's logical pixels; the core moves the window, this page decides where it goes.
  « Sur le bureau » (prompt 12, lot 1): he stays where he is put down, can be picked up
  again, goes home on a double click, and the core saves his place for the next launch.
  He wears his outfit everywhere; a right click on him opens his wardrobe in the island.
-->
<script module lang="ts">
  import type { BuddyMode, BuddySpot, BuddySpotKind } from "./lib/ipc";
  import type { FaceId } from "./lib/mochi/faces";
  import type { MoveId } from "./lib/mochi/moves";

  export type Point = { x: number; y: number };
  /** What Mochi does once he reached a place. */
  export type Act = "look" | "dance" | "wave" | "peek" | "sleep";
  /** Out of the island, held under the cursor, hopping, stopped, following, flying home,
   * resting on the desktop. */
  export type Phase = "out" | "held" | "hop" | "stop" | "follow" | "home" | "rest";
  /** A move of the drawing engine and, sometimes, a face. */
  export type Look = { move: MoveId; face?: FaceId };
  /** The cursor as the core reports it, with the primary button. */
  export type CursorReport = { x: number; y: number; down?: boolean };
  type Sample = Point & { t: number };

  /** Outing length in « Se balade » mode before Mochi heads home on his own. */
  export const ROAM_MS = 15_000;
  /** Released this close to his place in the island, Mochi goes straight home. */
  export const HOME_REACH = 90;
  /** « Te suit »: where Mochi trails the cursor, and the demo's spring. */
  export const FOLLOW_OFFSET: Point = { x: -64, y: 30 };
  const FOLLOW_STIFFNESS = 28;
  const FOLLOW_FRICTION = 8.5;
  /** Faster than this, a following Mochi bounces. */
  export const BOUNCY_SPEED = 140;
  const HOP_LENGTH = 130;
  /** A new place is at least this far, when there is a choice. */
  const MIN_TRIP = 60;
  /** The release speed comes from the moves of this last stretch. */
  export const TRAIL_MS = 100;
  /** Dropped on a window offered to the chat, he looks at it this long. */
  const LOOK_MS = 2400;

  /** The action at each kind of place (docs/09-DEMO-FINALE.md, section 6). */
  export const ACT_AT: Readonly<Record<BuddySpotKind, Act>> = {
    bas: "look",
    fenetre: "dance",
    curseur: "wave",
    bord: "peek",
    milieu: "sleep",
  };

  /** How often each kind of place is picked (the demo's `pickSpot`, the title merged in). */
  export const WEIGHTS: Readonly<Record<BuddySpotKind, number>> = {
    bas: 0.24,
    fenetre: 0.24,
    curseur: 0.18,
    bord: 0.16,
    milieu: 0.18,
  };

  const ACT_LOOK: Readonly<Record<Act, Look>> = {
    look: { move: "watch" },
    dance: { move: "party" },
    wave: { move: "rock", face: "wink" },
    peek: { move: "peek", face: "wow" },
    sleep: { move: "sleep" },
  };

  const clamp = (v: number, low: number, high: number): number => Math.min(high, Math.max(low, v));
  const lerp = (a: Point, b: Point, p: number): Point => ({ x: a.x + (b.x - a.x) * p, y: a.y + (b.y - a.y) * p });
  export const easeInOut = (p: number): number => (p < 0.5 ? 2 * p * p : 1 - (-2 * p + 2) ** 2 / 2);
  export const easeOut = (p: number): number => 1 - (1 - p) ** 3;

  /** Where Mochi starts and how he behaves, from the page's address. */
  export function readStart(search: string): { start: Point; mode: BuddyMode } {
    const query = new URLSearchParams(search);
    const number = (key: string): number => {
      const value = Number(query.get(key));
      return Number.isFinite(value) ? value : 0;
    };
    const asked = query.get("mode");
    return { start: { x: number("x"), y: number("y") }, mode: asked === "suit" || asked === "bureau" ? asked : "balade" };
  }

  /** At launch, Mochi is put back where he was left on the desktop: no jump out of the island. */
  export function isRestore(search: string): boolean {
    return new URLSearchParams(search).get("restore") === "1";
  }

  /** On the desktop, he breathes this long after the last touch, then holds still. */
  export const DESK_AWAKE_MS = 60_000;

  /** Mochi's look for the moment. */
  export function lookOf(phase: Phase, act: Act | null, speed: number): Look {
    switch (phase) {
      case "held":
        return { move: "breathe", face: "wow" };
      case "home":
        return { move: "fly", face: "wow" };
      case "follow":
        return speed > BOUNCY_SPEED ? { move: "bounce", face: "happy" } : { move: "breathe" };
      case "rest":
        return { move: "breathe" };
      case "stop":
        return act ? ACT_LOOK[act] : { move: "breathe" };
      default:
        return { move: "breathe", face: "happy" };
    }
  }

  /** A place to go: a kind drawn by its weight among the kinds on offer, then one of its
   * places, away from where Mochi stands when he can. */
  export function pickSpot(spots: readonly BuddySpot[], random: () => number, from?: Point): BuddySpot | undefined {
    const away = from ? spots.filter((s) => Math.hypot(s.x - from.x, s.y - from.y) >= MIN_TRIP) : [];
    const pool = away.length > 0 ? away : spots;
    const kinds = [...new Set(pool.map((s) => s.kind))];
    if (kinds.length === 0) return undefined;
    let draw = random() * kinds.reduce((sum, kind) => sum + WEIGHTS[kind], 0);
    const kind = kinds.find((k) => (draw -= WEIGHTS[k]) < 0) ?? kinds[kinds.length - 1];
    const choices = pool.filter((s) => s.kind === kind);
    return choices[Math.min(choices.length - 1, Math.floor(random() * choices.length))];
  }

  export type Hop = { from: Point; to: Point; height: number; ms: number };

  /** Small hops of about 130 px, higher when they climb, as in the demo's `hopTo`. */
  export function hops(from: Point, to: Point): Hop[] {
    const distance = Math.hypot(to.x - from.x, to.y - from.y);
    const count = Math.max(1, Math.round(distance / HOP_LENGTH));
    const ms = clamp((distance / count) * 2.2, 260, 520);
    return Array.from({ length: count }, (_, i) => {
      const a = lerp(from, to, i / count);
      const b = lerp(from, to, (i + 1) / count);
      return { from: a, to: b, height: 26 + Math.min(60, Math.abs(b.y - a.y) * 0.4), ms };
    });
  }

  export function along(hop: Hop, p: number): Point {
    const at = lerp(hop.from, hop.to, p);
    return { x: at.x, y: at.y - Math.sin(Math.PI * p) * hop.height };
  }

  /** Leaving the island: a small jump away from it (down, or up when the island sits low). */
  export function dropAt(start: Point, direction: 1 | -1, p: number): Point {
    return { x: start.x - 20 * p, y: start.y + direction * 90 * easeOut(p) - Math.sin(Math.PI * p) * 30 };
  }

  /** Away from the island: toward the middle of the screen. */
  export function dropDirection(start: Point, spots: readonly BuddySpot[]): 1 | -1 {
    const middle = spots.filter((s) => s.kind === "milieu");
    if (middle.length === 0) return 1;
    const y = middle.reduce((sum, s) => sum + s.y, 0) / middle.length;
    return y >= start.y ? 1 : -1;
  }

  /** Flying home in an arc. */
  export function flightAt(from: Point, to: Point, p: number): Point {
    const at = lerp(from, to, easeInOut(p));
    return { x: at.x, y: at.y - Math.sin(Math.PI * p) * 110 };
  }

  /** Speed at release, in pixels per second. */
  export function releaseSpeed(trail: readonly Sample[]): Point {
    if (trail.length < 2) return { x: 0, y: 0 };
    const a = trail[0];
    const b = trail[trail.length - 1];
    const seconds = (b.t - a.t) / 1000 || 0.016;
    return { x: (b.x - a.x) / seconds, y: (b.y - a.y) / seconds };
  }

  /** One step of « Te suit »: a spring toward the cursor that keeps its momentum. */
  export function followStep(pos: Point, speed: Point, target: Point, seconds: number): { pos: Point; speed: Point } {
    const ax = (target.x - pos.x) * FOLLOW_STIFFNESS - speed.x * FOLLOW_FRICTION;
    const ay = (target.y - pos.y) * FOLLOW_STIFFNESS - speed.y * FOLLOW_FRICTION;
    const next = { x: speed.x + ax * seconds, y: speed.y + ay * seconds };
    return { pos: { x: pos.x + next.x * seconds, y: pos.y + next.y * seconds }, speed: next };
  }
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { prefersReducedMotion } from "svelte/motion";
  import Mochi from "./lib/mochi/Mochi.svelte";
  import { i18n } from "./lib/i18n/i18n.svelte";
  import { buddyDropWindow, buddyGrab, buddyHide, buddyHome, buddyMove, buddySettle, buddySpots, onCoreEvent, openWardrobe } from "./lib/ipc";
  import { settings } from "./lib/stores/settings.svelte";

  let phase = $state<Phase>("out");
  let act = $state<Act | null>(null);
  let speed = $state(0);
  /** Small while he leaves the island, smaller again as he flies home. */
  let size = $state(0.66);
  let landed = $state(false);
  /** Resting on the desktop, he holds still a minute after the last touch (no CPU). */
  let awake = $state(true);
  let awakeTimer: ReturnType<typeof setTimeout> | undefined;
  const look = $derived(lookOf(phase, act, speed));

  // The window speaks the interface's language (screen readers read Mochi's name in it).
  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });

  // Motion bookkeeping, not state: read and written every frame.
  let mode: BuddyMode = "balade";
  let pos: Point = { x: 0, y: 0 };
  let sent: Point | null = null;
  let home: Point | null = null;
  let cursor: Point | null = null;
  let velocity: Point = { x: 0, y: 0 };
  let trail: Sample[] = [];
  /** Bumped by every new plan: an older one stops at its next step. */
  let run = 0;
  let frame = 0;
  let last = 0;
  let waiting: ((report: CursorReport | null) => void) | null = null;

  const still = (): boolean => prefersReducedMotion.current;
  const wait = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));

  /** Puts Mochi there: the core moves the window (once per frame at most). */
  function place(p: Point): void {
    pos = p;
    if (sent && Math.abs(p.x - sent.x) < 0.5 && Math.abs(p.y - sent.y) < 0.5) return;
    sent = { ...p };
    void buddyMove(p.x, p.y);
  }

  /** Plays `draw` from 0 to 1 over `ms`; at once with reduced motion. */
  function tween(ms: number, draw: (p: number) => void, token: number): Promise<void> {
    if (still()) {
      draw(1);
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      const began = performance.now();
      const step = (now: number): void => {
        if (token !== run) return resolve();
        const p = Math.min(1, (now - began) / ms);
        draw(p);
        if (p < 1) requestAnimationFrame(step);
        else resolve();
      };
      requestAnimationFrame(step);
    });
  }

  /** The first cursor report, or nothing after `ms` (browser preview). */
  function nextCursor(ms: number): Promise<CursorReport | null> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        waiting = null;
        resolve(null);
      }, ms);
      waiting = (report) => {
        clearTimeout(timer);
        resolve(report);
      };
    });
  }

  function onCursor(report: CursorReport): void {
    cursor = { x: report.x, y: report.y };
    waiting?.(report);
    waiting = null;
    if (phase === "held") {
      holdAt(report);
      if (!report.down) release();
    } else if (phase === "follow") {
      wake();
    }
  }

  function holdAt(report: CursorReport): void {
    const now = performance.now();
    const p = { x: report.x, y: report.y + 10 };
    trail = [...trail, { ...p, t: now }].filter((s) => now - s.t < TRAIL_MS);
    place(p);
  }

  /** Let go: home if dropped by the island, put down where he is « Sur le bureau »; else
   * the window under him is offered to the chat as context (only its title is looked at:
   * prompt 12, lot 3), he looks at it a moment, then goes on his outing with the throw. */
  function release(): void {
    const throwSpeed = releaseSpeed(trail);
    trail = [];
    if (home && Math.hypot(pos.x - home.x, pos.y - home.y) < HOME_REACH) {
      void goHome();
      return;
    }
    if (mode === "bureau") {
      settle();
      return;
    }
    // Out of the user's hand at once: later cursor reports no longer count as a release.
    const token = ++run;
    phase = "stop";
    act = null;
    void buddyDropWindow(pos.x, pos.y).then(async (offered) => {
      if (token !== run) return;
      if (offered) {
        // He peers at the window offered to the chat, then goes on.
        act = "look";
        await wait(LOOK_MS);
        if (token !== run) return;
      }
      if (mode === "suit") follow(throwSpeed);
      else void wander();
    });
  }

  /** « Sur le bureau »: he stays here; the core keeps the place for the next launch. */
  function settle(): void {
    ++run;
    phase = "rest";
    act = null;
    squash();
    wakeUp();
    void buddySettle(pos.x, pos.y);
  }

  /** On the desktop he breathes a minute after being touched, then holds still. */
  function wakeUp(): void {
    awake = true;
    clearTimeout(awakeTimer);
    awakeTimer = setTimeout(() => (awake = false), DESK_AWAKE_MS);
  }

  /** On the desktop, a press picks him up again. */
  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0 || mode !== "bureau" || phase !== "rest") return;
    phase = "held";
    trail = [];
    void buddyGrab();
  }

  /** A right click opens his wardrobe in the island. */
  function oncontextmenu(event: MouseEvent): void {
    event.preventDefault();
    void openWardrobe();
  }

  /** On the desktop, a double click sends him home. */
  function ondblclick(): void {
    if (mode === "bureau" && phase === "rest") void goHome();
  }

  function squash(): void {
    if (still()) return;
    landed = true;
    setTimeout(() => (landed = false), 170);
  }

  async function hopTo(spot: Point, token: number): Promise<void> {
    for (const hop of hops(pos, spot)) {
      if (token !== run) return;
      phase = "hop";
      act = null;
      await tween(hop.ms, (p) => place(along(hop, p)), token);
      squash();
    }
  }

  /** « Se balade »: hops from place to place and acts at each stop, then heads home. */
  async function wander(): Promise<void> {
    const token = ++run;
    const began = performance.now();
    while (token === run && performance.now() - began < ROAM_MS) {
      const spot = pickSpot(await buddySpots(), Math.random, pos);
      if (token !== run) return;
      if (!spot) break;
      await hopTo(spot, token);
      if (token !== run) return;
      phase = "stop";
      act = ACT_AT[spot.kind];
      await wait(act === "sleep" ? 2000 : 900 + Math.random() * 1000);
    }
    if (token === run) void goHome();
  }

  /** « Te suit »: trails the cursor on a spring, starting with the throw. */
  function follow(throwSpeed: Point): void {
    ++run;
    phase = "follow";
    act = null;
    velocity = throwSpeed;
    wake();
  }

  function wake(): void {
    if (frame !== 0) return;
    last = performance.now();
    frame = requestAnimationFrame(step);
  }

  function step(now: number): void {
    frame = 0;
    if (phase !== "follow" || !cursor) return;
    const target = { x: cursor.x + FOLLOW_OFFSET.x, y: cursor.y + FOLLOW_OFFSET.y };
    if (still()) {
      speed = 0;
      place(target);
      return;
    }
    const seconds = Math.min(0.05, (now - last) / 1000);
    last = now;
    const next = followStep(pos, velocity, target, seconds);
    velocity = next.speed;
    speed = Math.hypot(velocity.x, velocity.y);
    place(next.pos);
    const resting = speed < 1 && Math.hypot(target.x - next.pos.x, target.y - next.pos.y) < 0.5;
    if (!resting) frame = requestAnimationFrame(step);
  }

  /** Back into the island: a flight in an arc, or straight in with reduced motion. */
  async function goHome(): Promise<void> {
    if (phase === "home") return;
    const token = ++run;
    phase = "home";
    act = null;
    if (!still()) {
      const from = { ...pos };
      const to = await buddyHome().catch(() => home ?? from);
      await tween(620, (p) => {
        place(flightAt(from, to, p));
        size = 1 - 0.33 * easeInOut(p);
      }, token);
    }
    if (token === run) await buddyHide();
  }

  /** Held under the cursor if the user pulled him out, else he jumps out on his own. */
  async function begin(start: Point): Promise<void> {
    const token = ++run;
    const first = await nextCursor(400);
    if (token !== run) return;
    if (first?.down) {
      phase = "held";
      size = 1;
      holdAt(first);
      return;
    }
    const direction = dropDirection(start, await buddySpots());
    if (token !== run) return;
    await tween(480, (p) => {
      place(dropAt(start, direction, p));
      size = 0.66 + 0.34 * p;
    }, token);
    if (token !== run) return;
    size = 1;
    if (mode === "bureau") settle();
    else if (mode === "suit") follow({ x: 0, y: 0 });
    else void wander();
  }

  onMount(() => {
    const start = readStart(location.search);
    mode = start.mode;
    place(start.start);
    const restoring = mode === "bureau" && isRestore(location.search);
    const stops = [
      onCoreEvent("buddy:cursor", (report) => onCursor(report as CursorReport)),
      onCoreEvent("buddy:recall", () => void goHome()),
      onCoreEvent("settings:changed", (value) => settings.apply(value)),
    ];
    void settings.load();
    buddyHome().then(
      (point) => (home = point),
      () => (home = null),
    );
    if (restoring) {
      // Back where he was left: he is already resting there.
      size = 1;
      phase = "rest";
      wakeUp();
    } else {
      void begin(start.start);
    }
    return () => {
      run++;
      cancelAnimationFrame(frame);
      clearTimeout(awakeTimer);
      for (const stop of stops) stop();
    };
  });
</script>

<div class="buddy" class:landed style:--size={size}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="body" {onpointerdown} {oncontextmenu} {ondblclick} onpointerenter={() => phase === "rest" && wakeUp()}>
    <Mochi
      palette={settings.value.mochi.palette}
      move={look.move}
      face={look.face}
      scale={3}
      animated={settings.value.mochi.animated && (phase !== "rest" || awake)}
      label={settings.value.mochi.name}
      dressed
    />
  </span>
</div>

<style>
  /* The whole window is Mochi: transparent around him, never a target for the mouse. */
  .buddy {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
    transform: scale(var(--size));
    transform-origin: 50% 50%;
  }

  /* The core lets the clicks reach the window only on Mochi himself (buddy.rs). */
  .body {
    pointer-events: auto;
  }

  /* Landing from a hop: a quick squash on his feet. */
  .buddy.landed {
    transform: scale(calc(var(--size) * 1.12), calc(var(--size) * 0.86));
    transform-origin: 50% 84%;
  }
</style>
