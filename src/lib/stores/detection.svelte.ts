import { api, type DetectionResult, type InstallInfo } from "$lib/api";
import { settings } from "./settings.svelte";
import { status } from "./status.svelte";
import { toasts } from "./toasts.svelte";

class DetectionStore {
  install = $state<InstallInfo | null>(null);
  result = $state<DetectionResult | null>(null);
  loadingInstall = $state(true);
  scanning = $state(false);
  /** fileName currently being pinned, or "" for "back to auto". */
  pinning = $state<string | null>(null);
  changingFolder = $state(false);

  async load() {
    await Promise.all([this.loadInstall(), this.rescan(true)]);
  }

  async loadInstall() {
    try {
      this.install = await api.getInstallInfo();
    } catch (e) {
      toasts.error(e, "Couldn't read install info");
    } finally {
      this.loadingInstall = false;
    }
  }

  async rescan(silent = false) {
    if (this.scanning) return;
    this.scanning = true;
    try {
      this.result = await api.detectTargets();
      if (!silent) {
        const n = this.result.candidates.length;
        toasts.info(n ? `Found ${n} menu video${n === 1 ? "" : "s"}.` : "No menu videos found.", "Scan complete");
      }
    } catch (e) {
      toasts.error(e, "Scan failed");
    } finally {
      this.scanning = false;
    }
  }

  async pin(fileName: string | null) {
    if (this.pinning !== null) return;
    this.pinning = fileName ?? "";
    try {
      this.result = await api.pinTarget(fileName);
      if (settings.value) settings.set({ ...settings.value, pinnedTarget: fileName });
      if (status.value) status.value = { ...status.value, target: this.result.selected?.fileName ?? null };
      toasts.success(fileName ? `Veil will always swap ${fileName}.` : "Back to automatic detection.", fileName ? "Pinned" : "Auto-detect");
    } catch (e) {
      toasts.error(e, "Couldn't change target");
    } finally {
      this.pinning = null;
    }
  }

  async setOverride(path: string | null) {
    this.changingFolder = true;
    try {
      this.install = await api.setInstallOverride(path);
      if (settings.value) settings.set({ ...settings.value, installOverride: path });
      toasts.success(path ? "Using your chosen VALORANT folder." : "Back to automatic install detection.", "Install folder");
      await this.rescan(true);
    } catch (e) {
      toasts.error(e, "Couldn't use that folder");
    } finally {
      this.changingFolder = false;
    }
  }
}

export const detection = new DetectionStore();
