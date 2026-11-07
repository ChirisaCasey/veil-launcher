import { api, type ProgressEvent, type VideoItem } from "$lib/api";
import { basename } from "$lib/format";
import { settings } from "./settings.svelte";
import { toasts } from "./toasts.svelte";

export interface Job {
  id: string;
  name: string;
  percent: number;
  stage: string;
  done: boolean;
  error: string | null;
}

class VideosStore {
  items = $state<VideoItem[]>([]);
  loading = $state(true);
  jobs = $state<Job[]>([]);
  /** File names currently being imported (rendered as skeleton cards). */
  importing = $state<string[]>([]);
  selecting = $state<string | null>(null);

  selected = $derived(this.items.find((v) => v.id === settings.value?.selectedVideoId) ?? null);

  async load() {
    try {
      this.items = await api.listVideos();
    } catch (e) {
      toasts.error(e, "Couldn't load library");
    } finally {
      this.loading = false;
    }
  }

  async importFile(path: string): Promise<VideoItem | null> {
    const name = basename(path);
    this.importing = [...this.importing, name];
    try {
      const video = await api.importVideo(path);
      this.items = [video, ...this.items.filter((v) => v.id !== video.id)];
      return video;
    } catch (e) {
      toasts.error(e, `Couldn't add ${name}`);
      return null;
    } finally {
      const i = this.importing.indexOf(name);
      if (i >= 0) this.importing = this.importing.filter((_, j) => j !== i);
    }
  }

  async remove(video: VideoItem) {
    const prevItems = this.items;
    const prevSettings = settings.value;
    this.items = this.items.filter((v) => v.id !== video.id);
    if (settings.value?.selectedVideoId === video.id) settings.set({ ...settings.value, selectedVideoId: null });
    try {
      await api.removeVideo(video.id);
      toasts.info(`${video.name} was removed from the library.`, "Removed");
    } catch (e) {
      this.items = prevItems;
      if (prevSettings) settings.set(prevSettings);
      toasts.error(e, "Couldn't remove video");
    }
  }

  async select(id: string | null) {
    if (!settings.value || settings.value.selectedVideoId === id) return;
    const prev = settings.value;
    this.selecting = id;
    settings.set({ ...prev, selectedVideoId: id });
    try {
      settings.set(await api.selectVideo(id));
    } catch (e) {
      settings.set(prev);
      toasts.error(e, "Couldn't select video");
    } finally {
      this.selecting = null;
    }
  }

  async transcode(path: string, displayName = basename(path)): Promise<string | null> {
    try {
      const jobId = await api.transcodeVideo(path);
      this.#upsertJob(jobId, { name: displayName });
      return jobId;
    } catch (e) {
      toasts.error(e, `Couldn't convert ${displayName}`);
      return null;
    }
  }

  dismissJob(id: string) {
    this.jobs = this.jobs.filter((j) => j.id !== id);
  }

  onProgress(e: ProgressEvent) {
    this.#upsertJob(e.jobId, { percent: e.percent, stage: e.stage, done: e.done, error: e.error });
    if (e.error) {
      toasts.error(e.error, "Conversion failed");
      return;
    }
    if (e.done) {
      const job = this.jobs.find((j) => j.id === e.jobId);
      if (e.video) {
        const v = e.video;
        this.items = [v, ...this.items.filter((x) => x.id !== v.id)];
        toasts.success(`${v.name} is ready to use.`, "Conversion finished");
        // Make the first video in an empty selection the active one.
        if (settings.value && !settings.value.selectedVideoId) void this.select(v.id);
      } else if (job) {
        toasts.success(`${job.name} finished converting.`, "Conversion finished");
        void this.load();
      }
      setTimeout(() => this.dismissJob(e.jobId), 1400);
    }
  }

  #upsertJob(id: string, patch: Partial<Job>) {
    const existing = this.jobs.find((j) => j.id === id);
    if (existing) {
      // Don't let a late "name" update clobber progress that already arrived.
      Object.assign(existing, patch);
    } else {
      this.jobs = [
        ...this.jobs,
        { id, name: "Video", percent: 0, stage: "Queued", done: false, error: null, ...patch },
      ];
    }
  }
}

export const videos = new VideosStore();
