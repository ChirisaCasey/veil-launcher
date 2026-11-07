import { api, EVENTS, on, type Status } from "$lib/api";
import { toasts } from "./toasts.svelte";

class StatusStore {
  value = $state<Status | null>(null);
  loading = $state(true);
  launching = $state(false);
  applying = $state(false);
  restoring = $state(false);

  async load() {
    try {
      this.value = await api.getStatus();
    } catch (e) {
      toasts.error(e, "Couldn't read status");
    } finally {
      this.loading = false;
    }
  }

  listen() {
    return on<Status>(EVENTS.status, (s) => {
      this.value = s;
    });
  }

  async launch() {
    if (this.launching) return;
    this.launching = true;
    try {
      await api.launchGame();
      toasts.info("Starting VALORANT through the Riot Client.", "Launching");
    } catch (e) {
      toasts.error(e, "Couldn't launch");
    } finally {
      // Keep the button busy briefly so a double click doesn't fire twice.
      setTimeout(() => (this.launching = false), 800);
    }
  }

  async applyNow() {
    if (this.applying) return;
    this.applying = true;
    try {
      this.value = await api.applyNow();
      toasts.success("Your video is in place.", "Applied");
    } catch (e) {
      toasts.error(e, "Couldn't apply");
    } finally {
      this.applying = false;
    }
  }

  async restoreNow() {
    if (this.restoring) return;
    this.restoring = true;
    try {
      this.value = await api.restoreNow();
      toasts.success("The original background is back.", "Restored");
    } catch (e) {
      toasts.error(e, "Couldn't restore");
    } finally {
      this.restoring = false;
    }
  }
}

export const status = new StatusStore();
