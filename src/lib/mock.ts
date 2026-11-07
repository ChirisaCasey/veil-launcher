/**
 * In-memory fake backend used when the UI runs outside Tauri (plain `npm run dev` in a browser).
 * Implements every command in api.ts with realistic data and simulated timings so the whole UI
 * can be exercised without the Rust side.
 */
import {
  EVENTS,
  type AppInfo,
  type Candidate,
  type DetectionResult,
  type FfmpegStatus,
  type InstallInfo,
  type NotifyEvent,
  type ProgressEvent,
  type Settings,
  type Status,
  type VideoItem,
} from "./api";

type Handler = (payload: unknown) => void;

const DATA_DIR = "C:/Users/You/AppData/Roaming/VeilLauncher";
const LIB_DIR = `${DATA_DIR}/videos`;
const AUTO_INSTALL_DIR = "D:/Riot Games/VALORANT/live";
const MENU_SUFFIX = "/ShooterGame/Content/Movies/Menu";
const MB = 1024 * 1024;
const DAY = 86_400_000;

const now = Date.now();

const state = {
  settings: {
    themeMode: "system",
    accent: "#7c5cff",
    backdrop: "waves",
    selectedVideoId: "vid_neon",
    enabled: true,
    pinnedTarget: null,
    installOverride: null,
    backgroundMode: false,
    startWithWindows: false,
    startMinimized: false,
    swapTrigger: "riotClientClosed",
    swapDelaySecs: 3,
    keepAudio: false,
    notifications: true,
  } as Settings,

  install: {
    installDir: AUTO_INSTALL_DIR,
    menuDir: AUTO_INSTALL_DIR + MENU_SUFFIX,
    riotClient: "C:/Riot Games/Riot Client/RiotClientServices.exe",
    source: "metadata",
    writable: false,
  } as InstallInfo,

  candidates: [] as Candidate[],

  videos: [
    {
      id: "vid_neon",
      name: "Neon Skyline.mp4",
      path: `${LIB_DIR}/vid_neon.mp4`,
      size: 24.6 * MB,
      addedAt: now - 3 * DAY,
      durationSecs: 32,
      width: 1920,
      height: 1080,
      codec: "h264",
      compatible: true,
    },
    {
      id: "vid_reyna",
      name: "Reyna Ambient Loop.mp4",
      path: `${LIB_DIR}/vid_reyna.mp4`,
      size: 41.2 * MB,
      addedAt: now - 9 * DAY,
      durationSecs: 45,
      width: 2560,
      height: 1440,
      codec: "h264",
      compatible: true,
    },
    {
      id: "vid_rain",
      name: "Rainy Window 4K.mp4",
      path: `${LIB_DIR}/vid_rain.mp4`,
      size: 118.4 * MB,
      addedAt: now - 21 * DAY,
      durationSecs: 60,
      width: 3840,
      height: 2160,
      codec: "hevc",
      compatible: false,
    },
  ] as VideoItem[],

  status: {
    phase: "idle",
    swapped: false,
    gameRunning: false,
    riotClientRunning: false,
    target: "13_06_Homescreen.mp4",
    message: "Watching for VALORANT.",
    elevated: false,
    backgroundInstalled: false,
    hasBackup: true,
  } as Status,

  ffmpeg: { installed: false, path: null, version: null } as FfmpegStatus,

  app: { version: "0.1.0", elevated: false, dataDir: DATA_DIR } as AppInfo,
};

function buildCandidates(menuDir: string): Candidate[] {
  return [
    {
      fileName: "13_06_Homescreen.mp4",
      path: `${menuDir}/13_06_Homescreen.mp4`,
      size: 38.2 * MB,
      modified: now - 6 * DAY,
      score: 96,
      isHomescreen: true,
      version: "13.06",
      reason: "Name contains \"Homescreen\", newest version prefix, typical menu-loop size",
    },
    {
      fileName: "13_03_Homescreen.mp4",
      path: `${menuDir}/13_03_Homescreen.mp4`,
      size: 36.9 * MB,
      modified: now - 48 * DAY,
      score: 64,
      isHomescreen: true,
      version: "13.03",
      reason: "Homescreen match, but an older version prefix",
    },
    {
      fileName: "13_09_Seasonal_Prelude_Cinematic.mp4",
      path: `${menuDir}/13_09_Seasonal_Prelude_Cinematic.mp4`,
      size: 212.5 * MB,
      modified: now - 2 * DAY,
      score: 14,
      isHomescreen: false,
      version: "13.09",
      reason: "cinematic, not a menu loop",
    },
  ];
}
state.candidates = buildCandidates(state.install.menuDir!);

// ---------- Event bus ----------

const handlers = new Map<string, Set<Handler>>();

function emit<T>(event: string, payload: T) {
  const set = handlers.get(event);
  if (!set) return;
  for (const h of set) h(structuredClone(payload));
}

function notify(level: NotifyEvent["level"], message: string) {
  emit<NotifyEvent>(EVENTS.notify, { level, message });
}

function setStatus(patch: Partial<Status>) {
  state.status = { ...state.status, ...patch };
  emit(EVENTS.status, state.status);
}

export function mockListen<T>(event: string, handler: (payload: T) => void): () => void {
  let set = handlers.get(event);
  if (!set) {
    set = new Set();
    handlers.set(event, set);
  }
  const h = handler as Handler;
  set.add(h);
  return () => {
    set.delete(h);
  };
}

// ---------- Helpers ----------

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
const latency = (base = 120) => sleep(base + Math.random() * 180);

function detection(): DetectionResult {
  const candidates = state.install.menuDir ? [...state.candidates].sort((a, b) => b.score - a.score) : [];
  const pin = state.settings.pinnedTarget;
  const pinnedCandidate = pin ? candidates.find((c) => c.fileName === pin) ?? null : null;
  const selected = pinnedCandidate ?? candidates[0] ?? null;
  return {
    menuDir: state.install.menuDir,
    candidates,
    selected,
    pinned: !!pinnedCandidate,
    pinMissing: !!pin && !pinnedCandidate,
  };
}

function refreshTarget() {
  setStatus({ target: detection().selected?.fileName ?? null });
}

function baseName(path: string) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

let idCounter = 0;
function newId(prefix: string) {
  idCounter += 1;
  return `${prefix}_${Date.now().toString(36)}${idCounter}`;
}

function fakeMeta(name: string): Pick<VideoItem, "durationSecs" | "width" | "height" | "codec" | "size"> {
  const seed = [...name].reduce((a, c) => a + c.charCodeAt(0), 0);
  const res = [
    [1920, 1080],
    [2560, 1440],
    [3840, 2160],
  ][seed % 3];
  return {
    durationSecs: 20 + (seed % 50),
    width: res[0],
    height: res[1],
    codec: "h264",
    size: (15 + (seed % 80)) * MB,
  };
}

// ---------- Simulations ----------

let launchTimers: ReturnType<typeof setTimeout>[] = [];

function simulateLaunch() {
  launchTimers.forEach(clearTimeout);
  launchTimers = [];
  const at = (ms: number, fn: () => void) => launchTimers.push(setTimeout(fn, ms));
  const willSwap = state.settings.enabled && !!state.settings.selectedVideoId && !!detection().selected;

  setStatus({ phase: "riotClient", riotClientRunning: true, message: "Riot Client is verifying game files." });
  at(2500, () =>
    setStatus({
      phase: "waiting",
      gameRunning: true,
      message: "VALORANT started. Waiting for the Riot Client to close.",
    }),
  );
  at(4000, () => setStatus({ riotClientRunning: false, message: "File check finished." }));
  if (willSwap) {
    at(5000, () => {
      setStatus({
        phase: "swapped",
        swapped: true,
        hasBackup: true,
        message: `Custom video applied to ${detection().selected?.fileName}.`,
      });
      if (state.settings.notifications) notify("success", "Custom background applied.");
    });
  } else {
    at(5000, () => setStatus({ message: "Custom background is off. VALORANT is running normally." }));
  }
  at(8800, () =>
    setStatus({
      phase: willSwap ? "restoring" : "idle",
      gameRunning: false,
      message: willSwap ? "VALORANT closed. Restoring the original video." : "VALORANT closed.",
    }),
  );
  if (willSwap) {
    at(10000, () => {
      setStatus({ phase: "idle", swapped: false, message: "Original video restored." });
      if (state.settings.notifications) notify("info", "Original video restored.");
    });
  }
}

let ffmpegInstall: Promise<FfmpegStatus> | null = null;

async function simulateFfmpegInstall(): Promise<FfmpegStatus> {
  const steps = 40;
  for (let i = 0; i <= steps; i++) {
    const percent = Math.round((i / steps) * 100);
    const stage = percent < 82 ? `Downloading ffmpeg (${Math.round((percent / 82) * 98)} / 98 MB)` : "Extracting";
    emit<ProgressEvent>(EVENTS.progress, { jobId: "ffmpeg", percent, stage, done: false, error: null, video: null });
    await sleep(90 + Math.random() * 60);
  }
  state.ffmpeg = { installed: true, path: `${DATA_DIR}/bin/ffmpeg.exe`, version: "7.1" };
  emit<ProgressEvent>(EVENTS.progress, {
    jobId: "ffmpeg",
    percent: 100,
    stage: "Installed",
    done: true,
    error: null,
    video: null,
  });
  return state.ffmpeg;
}

async function simulateTranscode(jobId: string, path: string) {
  const fromLibrary = state.videos.find((v) => v.path === path);
  const srcName = fromLibrary?.name ?? baseName(path);
  const stem = srcName.replace(/\.[^.]+$/, "");
  const send = (percent: number, stage: string) =>
    emit<ProgressEvent>(EVENTS.progress, { jobId, percent, stage, done: false, error: null, video: null });

  send(0, "Analyzing");
  await sleep(500);
  const total = 30;
  for (let i = 1; i <= total; i++) {
    send(Math.round(3 + (i / total) * 92), "Converting to H.264");
    await sleep(120 + Math.random() * 80);
  }
  send(97, "Finalizing");
  await sleep(400);

  const id = newId("vid");
  const meta = fakeMeta(srcName);
  const video: VideoItem = {
    id,
    name: fromLibrary ? `${stem} (converted).mp4` : `${stem}.mp4`,
    path: `${LIB_DIR}/${id}.mp4`,
    addedAt: Date.now(),
    ...meta,
    width: fromLibrary?.width ?? meta.width,
    height: fromLibrary?.height ?? meta.height,
    durationSecs: fromLibrary?.durationSecs ?? meta.durationSecs,
    size: Math.round((fromLibrary?.size ?? meta.size) * 0.55),
    codec: "h264",
    compatible: true,
  };
  state.videos.unshift(video);
  emit<ProgressEvent>(EVENTS.progress, { jobId, percent: 100, stage: "Done", done: true, error: null, video });
}

// ---------- Commands ----------

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const result = await handle(cmd, args);
  return structuredClone(result) as T;
}

async function handle(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "get_app_info":
      await latency(40);
      return state.app;

    case "get_settings":
      await latency(60);
      return state.settings;

    case "update_settings": {
      await latency(60);
      const patch = (args.patch ?? {}) as Partial<Settings>;
      if (patch.swapDelaySecs != null) patch.swapDelaySecs = Math.max(0, Math.min(60, Math.round(patch.swapDelaySecs)));
      if (patch.accent != null && !/^#[0-9a-f]{6}$/i.test(patch.accent)) throw "Accent must be a #rrggbb color.";
      state.settings = { ...state.settings, ...patch };
      if (patch.backgroundMode !== undefined) {
        await sleep(400);
        setStatus({ backgroundInstalled: patch.backgroundMode });
      }
      return state.settings;
    }

    case "get_install_info":
      await latency(120);
      return state.install;

    case "set_install_override": {
      await latency(200);
      const path = (args.path as string | null) ?? null;
      if (path) {
        const dir = path.replace(/\\/g, "/").replace(/\/+$/, "");
        state.install = {
          installDir: dir,
          menuDir: dir + MENU_SUFFIX,
          riotClient: state.install.riotClient,
          source: "override",
          writable: true,
        };
      } else {
        state.install = {
          installDir: AUTO_INSTALL_DIR,
          menuDir: AUTO_INSTALL_DIR + MENU_SUFFIX,
          riotClient: "C:/Riot Games/Riot Client/RiotClientServices.exe",
          source: "metadata",
          writable: false,
        };
      }
      state.settings = { ...state.settings, installOverride: path };
      state.candidates = buildCandidates(state.install.menuDir!);
      refreshTarget();
      return state.install;
    }

    case "detect_targets":
      await latency(650);
      return detection();

    case "pin_target": {
      await latency(120);
      const fileName = (args.fileName as string | null) ?? null;
      if (fileName && !state.candidates.some((c) => c.fileName === fileName)) throw `${fileName} was not found in the Menu folder.`;
      state.settings = { ...state.settings, pinnedTarget: fileName };
      refreshTarget();
      return detection();
    }

    case "list_videos":
      await latency(350);
      return state.videos;

    case "import_video": {
      await latency(500);
      const path = String(args.path ?? "");
      const name = baseName(path);
      if (!/\.mp4$/i.test(name)) throw `${name} isn't an .mp4 file. Convert it with ffmpeg instead.`;
      const id = newId("vid");
      const meta = fakeMeta(name);
      // Pretend files with "hevc" or "4k" in the name are HEVC so the conversion flow can be tried.
      const hevc = /hevc|x265|4k/i.test(name);
      const video: VideoItem = {
        id,
        name,
        path: `${LIB_DIR}/${id}.mp4`,
        addedAt: Date.now(),
        ...meta,
        codec: hevc ? "hevc" : "h264",
        compatible: !hevc,
      };
      state.videos.unshift(video);
      return video;
    }

    case "remove_video": {
      await latency(150);
      const id = String(args.id);
      state.videos = state.videos.filter((v) => v.id !== id);
      if (state.settings.selectedVideoId === id) state.settings = { ...state.settings, selectedVideoId: null };
      return null;
    }

    case "select_video": {
      await latency(100);
      const id = (args.id as string | null) ?? null;
      if (id && !state.videos.some((v) => v.id === id)) throw "That video is no longer in the library.";
      state.settings = { ...state.settings, selectedVideoId: id };
      if (state.status.swapped && id) {
        setStatus({ message: "Swapped in the newly selected video." });
      }
      return state.settings;
    }

    case "get_status":
      await latency(60);
      return state.status;

    case "launch_game":
      await latency(300);
      if (!state.install.installDir) throw "VALORANT install not found. Set the folder in Detection.";
      if (state.status.gameRunning) throw "VALORANT is already running.";
      simulateLaunch();
      return null;

    case "apply_now": {
      await latency(400);
      if (!state.settings.selectedVideoId) throw "Pick a video in the library first.";
      const target = detection().selected;
      if (!target) throw "No Homescreen video found to replace.";
      setStatus({
        phase: "swapped",
        swapped: true,
        hasBackup: true,
        target: target.fileName,
        message: `Custom video applied to ${target.fileName}.`,
      });
      return state.status;
    }

    case "restore_now":
      await latency(400);
      if (!state.status.swapped) {
        setStatus({ message: "The original video is already in place." });
        return state.status;
      }
      setStatus({ phase: "idle", swapped: false, message: "Original video restored." });
      return state.status;

    case "ffmpeg_status":
      await latency(80);
      return state.ffmpeg;

    case "install_ffmpeg":
      if (state.ffmpeg.installed) return state.ffmpeg;
      if (!ffmpegInstall) ffmpegInstall = simulateFfmpegInstall().finally(() => (ffmpegInstall = null));
      return ffmpegInstall;

    case "transcode_video": {
      await latency(150);
      if (!state.ffmpeg.installed) throw "ffmpeg is not installed.";
      const jobId = newId("job");
      void simulateTranscode(jobId, String(args.path ?? ""));
      return jobId;
    }

    case "hide_to_tray":
      await latency(20);
      notify("info", "Hidden to tray (browser preview: nothing actually hides).");
      return null;

    case "quit_app":
      await latency(20);
      notify("info", "Quit requested (browser preview).");
      return null;

    default:
      throw `Unknown command: ${cmd}`;
  }
}
