/**
 * Thin wrappers over Tauri plugins/window APIs with browser fallbacks, so views never
 * have to branch on `isTauri` themselves.
 */
import { convertFileSrc } from "@tauri-apps/api/core";
import { isTauri } from "./api";
import { VIDEO_EXTENSIONS } from "./constants";

// ---------- Media ----------

/** URL a <video> can play for a local file, or null when not available (browser preview). */
export function mediaSrc(path: string | null | undefined): string | null {
  if (!path || !isTauri) return null;
  try {
    return convertFileSrc(path);
  } catch {
    return null;
  }
}

// ---------- Dialogs ----------

/** Browser preview: pick files with a hidden <input type="file"> and fabricate plausible paths. */
function browserPick(accept: string, multiple: boolean): Promise<string[]> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.style.display = "none";
    input.addEventListener("change", () => {
      const files = Array.from(input.files ?? []);
      resolve(files.map((f) => `C:/Users/You/Videos/${f.name}`));
      input.remove();
    });
    input.addEventListener("cancel", () => {
      resolve([]);
      input.remove();
    });
    document.body.appendChild(input);
    input.click();
  });
}

export async function pickVideos(): Promise<string[]> {
  if (!isTauri) return browserPick(VIDEO_EXTENSIONS.map((e) => "." + e).join(","), true);
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    multiple: true,
    directory: false,
    title: "Add videos",
    filters: [
      { name: "Videos", extensions: VIDEO_EXTENSIONS },
      { name: "MP4", extensions: ["mp4"] },
    ],
  });
  if (!result) return [];
  return Array.isArray(result) ? result : [result];
}

export async function pickFolder(title: string, defaultPath?: string | null): Promise<string | null> {
  if (!isTauri) {
    // No directory picker in the browser preview; pretend the user picked a typical alternate install.
    return "E:/Games/Riot Games/VALORANT/live";
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({ directory: true, multiple: false, title, defaultPath: defaultPath ?? undefined });
  return typeof result === "string" ? result : null;
}

// ---------- Opener ----------

export async function openExternal(url: string): Promise<void> {
  if (!isTauri) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

/** Show a file or folder in Explorer. Uses revealItemInDir, which `opener:default` allows. */
export async function revealPath(path: string): Promise<void> {
  if (!isTauri) {
    console.info("[preview] reveal", path);
    return;
  }
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}

// ---------- Window ----------

export async function minimizeWindow() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().minimize();
}

export async function toggleMaximizeWindow() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().toggleMaximize();
}

/** Calls `cb` with the maximized state now and whenever the window resizes. Returns a cleanup. */
export function watchMaximized(cb: (maximized: boolean) => void): () => void {
  if (!isTauri) return () => {};
  let unlisten: (() => void) | null = null;
  let disposed = false;
  void (async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const update = async () => cb(await win.isMaximized());
    await update();
    const un = await win.onResized(() => void update());
    if (disposed) un();
    else unlisten = un;
  })().catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}

// ---------- Drag & drop ----------

export interface DropHandlers {
  onEnter: () => void;
  onLeave: () => void;
  onDrop: (paths: string[]) => void;
}

/**
 * Native file drops (Tauri webview drag-drop event, which yields real paths).
 * In the browser preview, falls back to HTML5 drag events on `el` with fabricated paths.
 */
export function watchFileDrops(el: HTMLElement, h: DropHandlers): () => void {
  if (isTauri) {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void (async () => {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      const un = await getCurrentWebview().onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter") h.onEnter();
        else if (p.type === "leave") h.onLeave();
        else if (p.type === "drop") h.onDrop(p.paths);
      });
      if (disposed) un();
      else unlisten = un;
    })().catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }

  let depth = 0;
  const hasFiles = (e: DragEvent) => Array.from(e.dataTransfer?.types ?? []).includes("Files");
  const enter = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    if (depth++ === 0) h.onEnter();
  };
  const over = (e: DragEvent) => {
    if (hasFiles(e)) e.preventDefault();
  };
  const leave = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    if (--depth <= 0) {
      depth = 0;
      h.onLeave();
    }
  };
  const drop = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    depth = 0;
    const files = Array.from(e.dataTransfer?.files ?? []);
    h.onDrop(files.map((f) => `C:/Users/You/Videos/${f.name}`));
  };
  el.addEventListener("dragenter", enter);
  el.addEventListener("dragover", over);
  el.addEventListener("dragleave", leave);
  el.addEventListener("drop", drop);
  return () => {
    el.removeEventListener("dragenter", enter);
    el.removeEventListener("dragover", over);
    el.removeEventListener("dragleave", leave);
    el.removeEventListener("drop", drop);
  };
}
