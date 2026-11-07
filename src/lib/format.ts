import type { Phase, VideoItem } from "./api";

/** Backend errors are rejected as plain strings; normalize anything else. */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return "Something went wrong.";
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  const v = bytes / Math.pow(1024, i);
  return `${v >= 100 || i === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`;
}

export function formatDuration(secs: number | null): string | null {
  if (secs == null || !Number.isFinite(secs)) return null;
  const s = Math.round(secs);
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, "0")}`;
}

export function resolutionLabel(v: Pick<VideoItem, "width" | "height">): string | null {
  if (!v.width || !v.height) return null;
  const h = Math.min(v.width, v.height);
  if (h >= 2160) return "4K";
  if (h >= 1440) return "1440p";
  if (h >= 1080) return "1080p";
  if (h >= 720) return "720p";
  return `${v.width}×${v.height}`;
}

export function codecLabel(codec: string | null): string | null {
  if (!codec) return null;
  const c = codec.toLowerCase();
  const map: Record<string, string> = {
    h264: "H.264",
    avc: "H.264",
    avc1: "H.264",
    hevc: "HEVC",
    h265: "HEVC",
    vp8: "VP8",
    vp9: "VP9",
    av1: "AV1",
    mpeg4: "MPEG-4",
    prores: "ProRes",
  };
  return map[c] ?? codec.toUpperCase();
}

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

export function formatRelative(ms: number): string {
  const diff = ms - Date.now();
  const abs = Math.abs(diff);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (abs < minute) return "just now";
  if (abs < hour) return rtf.format(Math.round(diff / minute), "minute");
  if (abs < day) return rtf.format(Math.round(diff / hour), "hour");
  if (abs < 30 * day) return rtf.format(Math.round(diff / day), "day");
  return new Date(ms).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function formatDate(ms: number): string {
  return new Date(ms).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export function extension(path: string): string {
  const name = basename(path);
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot + 1).toLowerCase() : "";
}

export function stripExtension(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

export interface PhaseMeta {
  label: string;
  description: string;
  /** CSS color token used for the indicator. */
  tone: "accent" | "info" | "warning" | "success" | "danger";
  busy: boolean;
}

export const PHASES: Record<Phase, PhaseMeta> = {
  idle: {
    label: "Ready",
    description: "Launch VALORANT and Veil takes care of the rest.",
    tone: "accent",
    busy: false,
  },
  riotClient: {
    label: "Riot Client open",
    description: "Riot is checking game files. Veil waits until it's done.",
    tone: "info",
    busy: true,
  },
  waiting: {
    label: "Waiting for file check",
    description: "VALORANT is starting. Your video goes in once the check finishes.",
    tone: "warning",
    busy: true,
  },
  swapped: {
    label: "Custom video active",
    description: "Your background is in place. The original comes back when the game closes.",
    tone: "success",
    busy: false,
  },
  restoring: {
    label: "Restoring original",
    description: "VALORANT closed. Putting the original video back.",
    tone: "info",
    busy: true,
  },
  error: {
    label: "Needs attention",
    description: "Something went wrong. Check the message below.",
    tone: "danger",
    busy: false,
  },
};
