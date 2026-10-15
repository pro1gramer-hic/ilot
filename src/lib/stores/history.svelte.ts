// Today's work, in memory only: finished tasks and failures, for Mochi's summary.

import type { DayLog } from "../assistant";
import type { Finished } from "../island/banners";

class HistoryStore {
  log = $state<DayLog>({ finished: [], failures: 0 });
  #day = new Date().toDateString();

  recordFinished(finished: Finished, at = Date.now()): void {
    this.#rollOver(at);
    this.log.finished.push({ ...finished, at });
  }

  recordFailure(at = Date.now()): void {
    this.#rollOver(at);
    this.log.failures += 1;
  }

  /** A new day starts an empty log. */
  #rollOver(at: number): void {
    const day = new Date(at).toDateString();
    if (day === this.#day) return;
    this.#day = day;
    this.log = { finished: [], failures: 0 };
  }
}

export const history = new HistoryStore();
