// User settings, owned by the Rust core and mirrored here.

import { i18n } from "../i18n/i18n.svelte";
import { DEFAULT_SETTINGS, type Settings, getSettings, saveSettings, setShortcut } from "../ipc";
import type { ActionId } from "../shortcuts";

class SettingsStore {
  value = $state<Settings>({ ...DEFAULT_SETTINGS });
  error = $state<string | null>(null);

  async load(): Promise<void> {
    this.apply(await getSettings());
  }

  /** Takes settings coming from the core (load, tray menu, another window). */
  apply(settings: Settings): void {
    this.value = settings;
    i18n.language = settings.language;
    document.documentElement.dataset.theme = settings.theme;
  }

  /** Gives `action` a new shortcut (empty: none). Returns why it was refused (the core
   * keeps the previous one), or null. */
  async setShortcut(action: ActionId, shortcut: string): Promise<string | null> {
    try {
      this.apply(await setShortcut(action, shortcut, this.value));
      return null;
    } catch (error) {
      return error instanceof Error ? error.message : String(error);
    }
  }

  async update(patch: Partial<Settings>): Promise<void> {
    try {
      this.apply(await saveSettings({ ...this.value, ...patch }));
      this.error = null;
    } catch (error) {
      this.error = String(error);
    }
  }
}

export const settings = new SettingsStore();
