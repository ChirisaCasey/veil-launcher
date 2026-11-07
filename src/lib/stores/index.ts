import { EVENTS, on, type ProgressEvent, type Settings } from "$lib/api";
import { detection } from "./detection.svelte";
import { ffmpeg } from "./ffmpeg.svelte";
import { nav } from "./navigation.svelte";
import { settings } from "./settings.svelte";
import { status } from "./status.svelte";
import { toasts } from "./toasts.svelte";
import { videos } from "./videos.svelte";

export { detection, ffmpeg, nav, settings, status, toasts, videos };
export { theme } from "./theme.svelte";

/** Subscribe to backend events and load initial state. Returns a cleanup function. */
export async function initApp(): Promise<() => void> {
  const unlisteners = await Promise.all([
    toasts.listen(),
    nav.listen(),
    status.listen(),
    on<Settings>(EVENTS.settings, (s) => settings.set(s)),
    on<ProgressEvent>(EVENTS.progress, (e) => {
      if (e.jobId === "ffmpeg") ffmpeg.onProgress(e);
      else videos.onProgress(e);
    }),
  ]);

  void settings.load();
  void status.load();
  void videos.load();
  void detection.load();
  void ffmpeg.load();

  return () => unlisteners.forEach((u) => u());
}
