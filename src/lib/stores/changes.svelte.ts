// Which sessions have a new latest change (`diff:changed`). Only a counter per session:
// the change itself stays in the core's memory and is fetched while the live session
// shows it (`sessionDiff`).

class ChangesStore {
  /** Goes up each time the session's latest change is replaced. */
  versions = $state<Record<string, number>>({});

  changed(id: string): void {
    this.versions[id] = (this.versions[id] ?? 0) + 1;
  }
}

export const changes = new ChangesStore();
