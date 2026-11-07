import { api, type FfmpegStatus, type ProgressEvent } from "$lib/api";
import { errorMessage } from "$lib/format";
import { toasts } from "./toasts.svelte";

class FfmpegStore {
  status = $state<FfmpegStatus | null>(null);
  installing = $state(false);
  percent = $state(0);
  stage = $state("");
  error = $state<string | null>(null);

  get installed() {
    return this.status?.installed ?? false;
  }

  async load() {
    try {
      this.status = await api.ffmpegStatus();
    } catch (e) {
      toasts.error(e, "Couldn't check ffmpeg");
    }
  }

  /** Resolves true once ffmpeg is available. Safe to call while an install is running. */
  async install(): Promise<boolean> {
    if (this.installed) return true;
    this.installing = true;
    this.error = null;
    this.percent = 0;
    this.stage = "Starting download";
    try {
      this.status = await api.installFfmpeg();
      this.percent = 100;
      toasts.success("ffmpeg is ready. You can now convert videos.", "ffmpeg installed");
      return this.status.installed;
    } catch (e) {
      this.error = errorMessage(e);
      toasts.error(e, "ffmpeg download failed");
      return false;
    } finally {
      this.installing = false;
    }
  }

  onProgress(e: ProgressEvent) {
    this.percent = e.percent;
    this.stage = e.stage;
    if (e.error) this.error = e.error;
    if (!e.done) this.installing = true;
  }
}

export const ffmpeg = new FfmpegStore();
