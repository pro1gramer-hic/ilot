// Whether a looping element (the record, the wave) is on screen. The island keeps its
// three layers and its four tabs mounted and marks the hidden ones `inert`: a loop inside
// an inert subtree is invisible and must not run. One observer for the whole page, alive
// while at least one loop listens.

const changes = $state({ count: 0 });
let observer: MutationObserver | undefined;
let listeners = 0;

/** True while `node` sits outside every inert subtree. Reactive inside effects and
 * deriveds, as long as `watchInert` runs. */
export function onScreen(node: Element): boolean {
  void changes.count;
  return node.closest("[inert]") === null;
}

/** Follows the `inert` attributes of the page; returns the function that stops following. */
export function watchInert(): () => void {
  if (listeners++ === 0) {
    observer = new MutationObserver(() => changes.count++);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["inert"], subtree: true });
  }
  return () => {
    if (--listeners > 0) return;
    observer?.disconnect();
    observer = undefined;
  };
}
