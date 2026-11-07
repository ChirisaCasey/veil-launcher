/**
 * Veil Launcher — UI <-> Rust contract.
 *
 * This file is the single source of truth for the Tauri command surface.
 * Rust mirrors every type with `#[serde(rename_all = "camelCase")]` structs
 * (enums use camelCase string variants) in `src-tauri/src/commands.rs`.
 * Command names are snake_case; Tauri maps camelCase JS args to snake_case Rust args.
 *
 * When running outside Tauri (plain `npm run dev` in a browser) calls are routed
 * to `./mock.ts` so the UI can be developed and previewed without the backend.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

// ---------- Types ----------

export type ThemeMode = "system" | "light" | "dark";

/** When the watcher swaps the custom video in. */
export type SwapTrigger =
  /** Default: after VALORANT has started AND the Riot Client UI window process has closed (post file-check). */
  | "riotClientClosed"
  /** As soon as the VALORANT game process is detected (+ swapDelaySecs). */
  | "gameStarted";

export interface Settings {
  themeMode: ThemeMode;
  /** Accent color as #rrggbb. */
  accent: string;
  /** Library id of the selected custom video, or null. */
  selectedVideoId: string | null;
  /** Master switch: apply the custom background on game launch. */
  enabled: boolean;
  /** Exact Menu filename the user pinned (detection override), or null for auto. */
  pinnedTarget: string | null;
  /** User override for the VALORANT `live` folder, or null for auto-detect. */
  installOverride: string | null;
  /** Opt-in: keep watching in the background so launches outside the app also get the custom video. */
  backgroundMode: boolean;
  /** Opt-in: start with Windows (scheduled task if admin is needed, HKCU Run otherwise). */
  startWithWindows: boolean;
  /** Start hidden in the tray. */
  startMinimized: boolean;
  swapTrigger: SwapTrigger;
  /** Extra delay after the trigger fires before swapping (0–60). */
  swapDelaySecs: number;
  /** ffmpeg transcode: keep the audio track. */
  keepAudio: boolean;
  /** Show desktop notifications on swap/restore/errors. */
  notifications: boolean;
}

export type InstallSource = "metadata" | "registry" | "scan" | "override" | "none";

export interface InstallInfo {
  /** e.g. D:/Riot Games/VALORANT/live */
  installDir: string | null;
  /** <installDir>/ShooterGame/Content/Movies/Menu */
  menuDir: string | null;
  /** Path to RiotClientServices.exe */
  riotClient: string | null;
  source: InstallSource;
  /** True if the current process can write to menuDir without elevation. */
  writable: boolean;
}

export interface Candidate {
  fileName: string;
  path: string;
  size: number;
  /** Unix ms */
  modified: number;
  score: number;
  isHomescreen: boolean;
  /** Parsed version prefix like "13.06", if any. */
  version: string | null;
  /** Human-readable reason for the score, shown in the Detection view. */
  reason: string;
}

export interface DetectionResult {
  menuDir: string | null;
  /** Sorted best-first. */
  candidates: Candidate[];
  /** The file that will be swapped (pinned if valid, else best candidate). */
  selected: Candidate | null;
  pinned: boolean;
  /** A pin is set but that file no longer exists (fell back to auto). */
  pinMissing: boolean;
}

export interface VideoItem {
  id: string;
  name: string;
  /** Absolute path inside the library folder (%APPDATA%/VeilLauncher/videos). */
  path: string;
  size: number;
  /** Unix ms */
  addedAt: number;
  durationSecs: number | null;
  width: number | null;
  height: number | null;
  codec: string | null;
  /** Likely playable by the game (H.264 mp4). Unknown (no ffprobe) counts as true. */
  compatible: boolean;
}

export type Phase =
  | "idle" // nothing running
  | "riotClient" // Riot Client open, game not started
  | "waiting" // game started, waiting for the swap trigger
  | "swapped" // custom video is in place
  | "restoring" // game exited, restoring original
  | "error";

export interface Status {
  phase: Phase;
  /** Custom video currently in the game folder. */
  swapped: boolean;
  gameRunning: boolean;
  riotClientRunning: boolean;
  /** Menu filename being targeted. */
  target: string | null;
  /** Last human-readable event or error. */
  message: string | null;
  /** This process is elevated. */
  elevated: boolean;
  /** The background scheduled task / autostart entry is installed. */
  backgroundInstalled: boolean;
  /** A backup of the original exists. */
  hasBackup: boolean;
}

export interface FfmpegStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
}

export interface ProgressEvent {
  /** Transcode job id, or "ffmpeg" for the ffmpeg download. */
  jobId: string;
  /** 0–100 */
  percent: number;
  stage: string;
  done: boolean;
  error: string | null;
  /** Set when a transcode finishes. */
  video: VideoItem | null;
}

export interface AppInfo {
  version: string;
  elevated: boolean;
  dataDir: string;
}

export interface NotifyEvent {
  level: "info" | "success" | "warning" | "error";
  message: string;
}

// ---------- Events (Rust -> UI) ----------

export const EVENTS = {
  /** payload: Status */
  status: "status-changed",
  /** payload: ProgressEvent */
  progress: "progress",
  /** payload: string route, e.g. "settings" — sent by tray menu items */
  navigate: "navigate",
  /** payload: NotifyEvent — in-app toast */
  notify: "notify",
} as const;

// ---------- Plumbing ----------

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    const { mockInvoke } = await import("./mock");
    return mockInvoke<T>(cmd, args);
  }
  return tauriInvoke<T>(cmd, args);
}

export async function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!isTauri) {
    const { mockListen } = await import("./mock");
    return mockListen<T>(event, handler);
  }
  return tauriListen<T>(event, (e) => handler(e.payload));
}

// ---------- Commands ----------
// Errors are rejected as plain strings.

export const api = {
  getAppInfo: () => call<AppInfo>("get_app_info"),

  getSettings: () => call<Settings>("get_settings"),
  /** Merges the patch, persists, and applies side effects (autostart, background task). Returns the new settings. */
  updateSettings: (patch: Partial<Settings>) => call<Settings>("update_settings", { patch }),

  getInstallInfo: () => call<InstallInfo>("get_install_info"),
  /** null clears the override. */
  setInstallOverride: (path: string | null) => call<InstallInfo>("set_install_override", { path }),

  detectTargets: () => call<DetectionResult>("detect_targets"),
  /** null returns to auto-detection. */
  pinTarget: (fileName: string | null) => call<DetectionResult>("pin_target", { fileName }),

  listVideos: () => call<VideoItem[]>("list_videos"),
  /** Copies the file into the library. Non-mp4 inputs are rejected; use transcodeVideo. */
  importVideo: (path: string) => call<VideoItem>("import_video", { path }),
  removeVideo: (id: string) => call<void>("remove_video", { id }),
  /** Also applies immediately if a swap is currently active. */
  selectVideo: (id: string | null) => call<Settings>("select_video", { id }),

  getStatus: () => call<Status>("get_status"),
  /** Starts VALORANT through the Riot Client; the watcher handles the swap. */
  launchGame: () => call<void>("launch_game"),
  /** Manually swap the custom video in now (e.g. game already running). */
  applyNow: () => call<Status>("apply_now"),
  /** Restore the original video now. */
  restoreNow: () => call<Status>("restore_now"),

  ffmpegStatus: () => call<FfmpegStatus>("ffmpeg_status"),
  /** Downloads ffmpeg into the data dir; progress via EVENTS.progress with jobId "ffmpeg". */
  installFfmpeg: () => call<FfmpegStatus>("install_ffmpeg"),
  /** Starts a transcode job, returns jobId; progress + final VideoItem via EVENTS.progress. */
  transcodeVideo: (path: string) => call<string>("transcode_video", { path }),

  /** Hide the window to the tray. */
  hideToTray: () => call<void>("hide_to_tray"),
  /** Fully exit (restores the original video first if swapped and the game is not running). */
  quitApp: () => call<void>("quit_app"),
};
