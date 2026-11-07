// TODO: replace with the real repository URL once the project is published.
export const REPO_URL = "https://github.com/";

/** The lightweight .bat alternative. */
export const VAL_PATCHER_URL = "https://github.com/ChirisaCasey/Val-Patcher";

export const ACCENT_PRESETS = [
  { name: "Violet", value: "#7c5cff" },
  { name: "Crimson", value: "#ff4655" },
  { name: "Cyan", value: "#22d3ee" },
  { name: "Emerald", value: "#10b981" },
  { name: "Amber", value: "#f59e0b" },
  { name: "Pink", value: "#ec4899" },
] as const;

export const DEFAULT_ACCENT = "#7c5cff";

/** Extensions offered in the file picker. Only mp4 is imported directly; the rest are converted. */
export const VIDEO_EXTENSIONS = ["mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv"];

/** Approximate size of the optional ffmpeg download, shown to the user. */
export const FFMPEG_DOWNLOAD_MB = 100;
