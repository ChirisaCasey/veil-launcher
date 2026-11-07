import { api, type AppInfo, type Settings } from "$lib/api";
import { toasts } from "./toasts.svelte";

class SettingsStore {
  value = $state<Settings | null>(null);
  app = $state<AppInfo | null>(null);
  loading = $state(true);

  #pending: Partial<Settings> = {};
  #timer: ReturnType<typeof setTimeout> | null = null;

  async load() {
    try {
      const [s, app] = await Promise.all([api.getSettings(), api.getAppInfo()]);
      this.value = s;
      this.app = app;
    } catch (e) {
      toasts.error(e, "Couldn't load settings");
    } finally {
      this.loading = false;
    }
  }

  /** Optimistic update: applies locally, persists, rolls back on failure. */
  async update(patch: Partial<Settings>): Promise<boolean> {
    if (!this.value) return false;
    const prev = $state.snapshot(this.value) as Settings;
    this.value = { ...this.value, ...patch };
    try {
      this.value = await api.updateSettings(patch);
      return true;
    } catch (e) {
      this.value = prev;
      toasts.error(e, "Couldn't save setting");
      return false;
    }
  }

  /** For continuous inputs (sliders, color picker): apply locally now, persist after a pause. */
  updateDebounced(patch: Partial<Settings>, ms = 350) {
    if (!this.value) return;
    this.value = { ...this.value, ...patch };
    this.#pending = { ...this.#pending, ...patch };
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => this.flush(), ms);
  }

  async flush() {
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = null;
    const patch = this.#pending;
    this.#pending = {};
    if (Object.keys(patch).length === 0) return;
    try {
      this.value = await api.updateSettings(patch);
    } catch (e) {
      toasts.error(e, "Couldn't save setting");
      try {
        this.value = await api.getSettings();
      } catch {
        /* keep local value */
      }
    }
  }

  /** Replace from a command that returns the full settings object (e.g. selectVideo). */
  set(s: Settings) {
    this.value = s;
  }
}

export const settings = new SettingsStore();
