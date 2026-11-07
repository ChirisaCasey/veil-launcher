//! The engine ties settings, discovery, detection, swapping and the watcher together.
//! It knows nothing about Tauri: UI side effects go through `EventSink`.

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use crate::config::{DataDirs, Settings};
use crate::core::detect::{self, DetectionResult};
use crate::core::elevate::{self, helper_or_err};
use crate::core::ffmpeg::{self, ProgressEvent, TranscodeTarget};
use crate::core::hash::HashCache;
use crate::core::library::{Library, VideoItem, new_id};
use crate::core::paths::{self, DiscoveryOptions, InstallInfo, path_key, to_display};
use crate::core::swap::{
    FileWriter, ReplaceError, RestoreOutcome, SwapOutcome, Swapper, WRITE_TIMEOUT,
    replace_with_retry,
};
use crate::core::watcher::{
    Action, Machine, POLL_INTERVAL, Phase, ProcSnapshot, ProcessSource, WatchConfig,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: Phase,
    pub swapped: bool,
    pub game_running: bool,
    pub riot_client_running: bool,
    pub target: Option<String>,
    pub message: Option<String>,
    pub elevated: bool,
    pub background_installed: bool,
    pub has_backup: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NotifyLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyEvent {
    pub level: NotifyLevel,
    pub message: String,
}

pub trait EventSink: Send + Sync {
    fn status_changed(&self, _status: &Status) {}
    fn settings_changed(&self, _settings: &Settings) {}
    /// In-app toast; `desktop` = user wants desktop notifications too.
    fn notify(&self, _event: &NotifyEvent, _desktop: bool) {}
    fn progress(&self, _event: &ProgressEvent) {}
}

pub struct NullSink;
impl EventSink for NullSink {}

/// Writes into the game folder, falling back to the elevated helper on access denied.
pub struct EngineWriter {
    pub allow_elevation: bool,
}

impl FileWriter for EngineWriter {
    fn replace(&self, src: &Path, dst: &Path) -> Result<(), ReplaceError> {
        match replace_with_retry(src, dst, WRITE_TIMEOUT) {
            Err(ReplaceError::AccessDenied(shown, e))
                if self.allow_elevation && !elevate::is_elevated() =>
            {
                log::info!("access denied writing {shown}; using elevated helper");
                helper_or_err(
                    "swap",
                    &json!({ "source": to_display(src), "target": to_display(dst) }),
                )
                .map_err(|he| ReplaceError::Io(format!("{he:#}"), e))
            }
            r => r,
        }
    }
}

pub struct Engine {
    pub dirs: DataDirs,
    settings: Mutex<Settings>,
    status: Mutex<Status>,
    install: Mutex<Option<InstallInfo>>,
    cache: Arc<HashCache>,
    pub swapper: Swapper,
    pub library: Library,
    machine: Mutex<Machine>,
    procs: Mutex<ProcSnapshot>,
    op_lock: Mutex<()>,
    sink: RwLock<Arc<dyn EventSink>>,
    clock: Instant,
    writer: EngineWriter,
    stop: AtomicBool,
    pin_missing_warned: AtomicBool,
}

impl Engine {
    pub fn new(dirs: DataDirs, allow_elevation: bool) -> Result<Arc<Self>> {
        dirs.ensure()?;
        let settings = dirs.load_settings();
        let cache = Arc::new(HashCache::new());
        let swapper = Swapper::new(dirs.clone(), cache.clone());
        let swapped = swapper.journal().swapped;
        let status = Status {
            phase: Phase::Idle,
            swapped,
            game_running: false,
            riot_client_running: false,
            target: None,
            message: None,
            elevated: elevate::is_elevated(),
            background_installed: false,
            has_backup: !swapper.manifest().entries.is_empty(),
        };
        Ok(Arc::new(Self {
            library: Library::new(dirs.videos()),
            dirs,
            settings: Mutex::new(settings),
            status: Mutex::new(status),
            install: Mutex::new(None),
            cache,
            swapper,
            machine: Mutex::new(Machine::new()),
            procs: Mutex::new(ProcSnapshot::default()),
            op_lock: Mutex::new(()),
            sink: RwLock::new(Arc::new(NullSink)),
            clock: Instant::now(),
            writer: EngineWriter { allow_elevation },
            stop: AtomicBool::new(false),
            pin_missing_warned: AtomicBool::new(false),
        }))
    }

    pub fn set_sink(&self, sink: Arc<dyn EventSink>) {
        *self.sink.write().unwrap() = sink;
    }

    fn sink(&self) -> Arc<dyn EventSink> {
        self.sink.read().unwrap().clone()
    }

    // ----- settings -------------------------------------------------------

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Merge + persist. Returns (new, old). Engine-side side effects (restore when disabled,
    /// re-detect when the pin / override changes) are applied here; OS startup integration is
    /// handled by the caller.
    pub fn update_settings(&self, patch: &Value) -> Result<(Settings, Settings)> {
        let (new, old) = {
            let mut s = self.settings.lock().unwrap();
            let old = s.clone();
            let new = old.merged(patch)?;
            self.dirs.save_settings(&new)?;
            *s = new.clone();
            (new, old)
        };
        if new != old {
            self.sink().settings_changed(&new);
        }
        if new.install_override != old.install_override {
            self.refresh_install();
        }
        if new.pinned_target != old.pinned_target || new.install_override != old.install_override {
            self.pin_missing_warned.store(false, Ordering::Relaxed);
            self.detect();
        }
        if old.enabled
            && !new.enabled
            && self.swapper.journal().swapped
            && let Err(e) = self.restore_now(true)
        {
            self.report(
                NotifyLevel::Error,
                format!("Couldn't restore the original: {e:#}"),
            );
        }
        Ok((new, old))
    }

    // ----- discovery / detection ----------------------------------------

    pub fn refresh_install(&self) -> InstallInfo {
        #[allow(unused_mut)]
        let mut opts = DiscoveryOptions {
            install_override: self.settings().install_override,
            ..Default::default()
        };
        #[cfg(test)]
        {
            // Unit tests must never see (let alone write to) the real game install.
            opts.program_data = self.dirs.root.join("no-program-data");
            opts.use_registry = false;
            opts.scan_drives = false;
        }
        let info = paths::discover(&opts);
        *self.install.lock().unwrap() = Some(info.clone());
        info
    }

    pub fn install_info(&self) -> InstallInfo {
        let cached = self.install.lock().unwrap().clone();
        match cached {
            Some(i) if i.menu_dir.as_deref().is_some_and(|m| Path::new(m).is_dir()) => i,
            _ => self.refresh_install(),
        }
    }

    /// Validate + store the install override. `None` clears it.
    pub fn set_install_override(&self, path: Option<String>) -> Result<InstallInfo> {
        let value = match path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            None => Value::Null,
            Some(p) => {
                let live = paths::normalize_live_dir(Path::new(p)).ok_or_else(|| {
                    anyhow!("That folder doesn't look like a VALORANT install (expected ...\\VALORANT\\live\\ShooterGame\\Content\\Movies\\Menu)")
                })?;
                Value::from(to_display(&live))
            }
        };
        self.update_settings(&json!({ "installOverride": value }))?;
        Ok(self.refresh_install())
    }

    /// Detection without toasts (UI-initiated).
    pub fn detect(&self) -> DetectionResult {
        self.detect_inner(false)
    }

    /// Detection that warns once when a pinned file disappears (e.g. after a game patch).
    pub fn detect_and_warn(&self) -> DetectionResult {
        self.detect_inner(true)
    }

    fn detect_inner(&self, warn_pin_missing: bool) -> DetectionResult {
        let info = self.install_info();
        let pinned = self.settings().pinned_target;
        let r = detect::detect(info.menu_dir.as_deref().map(Path::new), pinned.as_deref());
        let target = r.selected.as_ref().map(|c| c.file_name.clone());
        let has_backup = match &target {
            Some(t) => self.swapper.has_backup(t),
            None => !self.swapper.manifest().entries.is_empty(),
        };
        self.update_status(|s| {
            s.target = target;
            s.has_backup = has_backup;
        });
        if !r.pin_missing {
            self.pin_missing_warned.store(false, Ordering::Relaxed);
        } else if warn_pin_missing && !self.pin_missing_warned.swap(true, Ordering::Relaxed) {
            let pin = self.settings().pinned_target.unwrap_or_default();
            let fallback = r
                .selected
                .as_ref()
                .map(|c| c.file_name.as_str())
                .unwrap_or("nothing");
            self.report(
                NotifyLevel::Warning,
                format!("Pinned video {pin} no longer exists (game update?). Using auto-detection: {fallback}"),
            );
        }
        r
    }

    pub fn pin_target(&self, file_name: Option<String>) -> Result<DetectionResult> {
        let v = file_name.map(Value::from).unwrap_or(Value::Null);
        self.update_settings(&json!({ "pinnedTarget": v }))?;
        Ok(self.detect())
    }

    // ----- status ---------------------------------------------------------

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    pub fn update_status(&self, f: impl FnOnce(&mut Status)) {
        let changed = {
            let mut s = self.status.lock().unwrap();
            let before = s.clone();
            f(&mut s);
            (*s != before).then(|| s.clone())
        };
        if let Some(s) = changed {
            self.sink().status_changed(&s);
        }
    }

    pub fn set_background_installed(&self, v: bool) {
        self.update_status(|s| s.background_installed = v);
    }

    pub fn notify_error(&self, message: impl Into<String>) {
        self.report(NotifyLevel::Error, message);
    }

    /// Log + status message, and a toast/desktop notification only when `notify`.
    fn note(&self, level: NotifyLevel, message: impl Into<String>, notify: bool) {
        if notify {
            return self.report(level, message);
        }
        let message = message.into();
        match level {
            NotifyLevel::Error => log::error!("{message}"),
            NotifyLevel::Warning => log::warn!("{message}"),
            _ => log::info!("{message}"),
        }
        self.update_status(|s| s.message = Some(message));
    }

    pub fn report(&self, level: NotifyLevel, message: impl Into<String>) {
        let message = message.into();
        match level {
            NotifyLevel::Error => log::error!("{message}"),
            NotifyLevel::Warning => log::warn!("{message}"),
            _ => log::info!("{message}"),
        }
        let desktop = self.settings().notifications;
        self.update_status(|s| s.message = Some(message.clone()));
        self.sink().notify(&NotifyEvent { level, message }, desktop);
    }

    pub fn emit_progress(&self, ev: ProgressEvent) {
        self.sink().progress(&ev);
    }

    fn sync_swapped_flag(&self) {
        let j = self.swapper.journal();
        let has_backup = match self.status().target {
            Some(t) => self.swapper.has_backup(&t),
            None => !self.swapper.manifest().entries.is_empty(),
        };
        self.update_status(|s| {
            s.swapped = j.swapped;
            s.has_backup = has_backup;
        });
    }

    // ----- swap / restore -------------------------------------------------

    fn selected_video(&self) -> Result<VideoItem> {
        let id = self
            .settings()
            .selected_video_id
            .ok_or_else(|| anyhow!("No custom video selected"))?;
        self.library
            .get(&id)
            .ok_or_else(|| anyhow!("The selected video is missing from the library"))
    }

    fn target_path(&self) -> Result<PathBuf> {
        let r = self.detect_and_warn();
        let c = r.selected.ok_or_else(|| {
            if r.menu_dir.is_none() {
                anyhow!("VALORANT install not found")
            } else {
                anyhow!("No Homescreen video detected; pin one in Detection")
            }
        })?;
        Ok(PathBuf::from(c.path))
    }

    /// Swap the selected video in. Serialised with restores.
    pub fn perform_swap(&self) -> Result<SwapOutcome> {
        let _g = self.op_lock.lock().unwrap();
        let video = self.selected_video()?;
        let target = self.target_path()?;
        let j = self.swapper.journal();
        if j.swapped
            && let Some(old) = j
                .target_path
                .as_deref()
                .filter(|t| path_key(t) != path_key(&to_display(&target)))
        {
            log::info!("target changed while swapped; restoring {old} first");
            self.swapper
                .restore(Some(Path::new(old)), &[], &self.writer)?;
        }
        let r = self
            .swapper
            .swap_in(&target, Path::new(&video.path), &self.writer);
        self.sync_swapped_flag();
        r
    }

    pub fn perform_restore(&self) -> Result<RestoreOutcome> {
        let _g = self.op_lock.lock().unwrap();
        let j = self.swapper.journal();
        let target = match j.target_path.as_deref() {
            Some(t) if j.swapped => PathBuf::from(t),
            _ => self.target_path()?,
        };
        let mut extra = Vec::new();
        if let Ok(v) = self.selected_video()
            && let Ok(h) = self.cache.hash(Path::new(&v.path))
        {
            extra.push(h);
        }
        let r = self.swapper.restore(Some(&target), &extra, &self.writer);
        self.sync_swapped_flag();
        r
    }

    fn describe_swap(&self, r: &Result<SwapOutcome>, notify: bool) {
        match r {
            Ok(SwapOutcome::Swapped) => {
                self.note(NotifyLevel::Success, "Custom background applied", notify)
            }
            Ok(SwapOutcome::AlreadyApplied) => self.note(
                NotifyLevel::Info,
                "Custom background already in place",
                notify,
            ),
            Err(e) => self.note(
                NotifyLevel::Error,
                format!("Couldn't apply the custom background: {e:#}"),
                notify,
            ),
        }
    }

    fn describe_restore(&self, r: &Result<RestoreOutcome>, will_retry: bool, notify: bool) {
        match r {
            Ok(RestoreOutcome::Restored) => {
                self.note(NotifyLevel::Success, "Original video restored", notify)
            }
            Ok(RestoreOutcome::AlreadyOriginal) => {
                self.note(NotifyLevel::Info, "Original video is in place", notify)
            }
            Ok(RestoreOutcome::KeptGenuine) => self.note(
                NotifyLevel::Info,
                "The game video was updated by Riot; kept the new version",
                notify,
            ),
            Err(e) if will_retry => self.note(
                NotifyLevel::Error,
                format!("Couldn't restore the original yet: {e:#} (retrying)"),
                notify,
            ),
            Err(e) => self.note(
                NotifyLevel::Error,
                format!("Couldn't restore the original: {e:#}"),
                notify,
            ),
        }
    }

    /// Manual swap. `notify`: toast the result (tray) or not (UI toasts itself).
    pub fn apply_now(&self, notify: bool) -> Result<Status> {
        let r = self.perform_swap();
        self.describe_swap(&r, notify);
        let game = self.procs.lock().unwrap().game;
        if r.is_ok() {
            self.machine.lock().unwrap().on_manual_swap(game);
            let phase = self.machine.lock().unwrap().phase;
            self.update_status(|s| s.phase = phase);
        }
        r.map(|_| self.status())
    }

    /// Manual restore. `notify`: toast the result (tray) or not (UI toasts itself).
    pub fn restore_now(&self, notify: bool) -> Result<Status> {
        let r = self.perform_restore();
        self.describe_restore(&r, false, notify);
        let o = *self.procs.lock().unwrap();
        if r.is_ok() {
            self.machine.lock().unwrap().on_manual_restore(o.game, o);
            let phase = self.machine.lock().unwrap().phase;
            self.update_status(|s| s.phase = phase);
        }
        r.map(|_| self.status())
    }

    // ----- library --------------------------------------------------------

    pub fn import_video(&self, path: &str) -> Result<VideoItem> {
        let src = Path::new(path);
        let probe =
            ffmpeg::find_ffprobe(&self.dirs.ffmpeg()).and_then(|p| ffmpeg::probe(&p, src).ok());
        let item = self.library.import(src, probe.as_ref())?;
        if item.compatible && self.settings().selected_video_id.is_none() {
            self.update_settings(&json!({ "selectedVideoId": item.id }))?;
        }
        Ok(item)
    }

    pub fn remove_video(&self, id: &str) -> Result<()> {
        self.library.remove(id)?;
        if self.settings().selected_video_id.as_deref() == Some(id) {
            self.update_settings(&json!({ "selectedVideoId": Value::Null }))?;
        }
        Ok(())
    }

    /// Select a video; if a swap is active, apply the new one (or restore when cleared).
    pub fn select_video(&self, id: Option<String>) -> Result<Settings> {
        if let Some(i) = &id
            && self.library.get(i).is_none()
        {
            bail!("video not found");
        }
        let (new, old) = self.update_settings(&json!({ "selectedVideoId": id }))?;
        if new.selected_video_id != old.selected_video_id && self.swapper.journal().swapped {
            if new.selected_video_id.is_some() {
                let r = self.perform_swap();
                self.describe_swap(&r, false);
                r?;
            } else {
                self.restore_now(false)?;
            }
        }
        Ok(new)
    }

    /// The genuine game video (backup if we have one, else the live file when not swapped).
    fn original_video(&self) -> Option<PathBuf> {
        let target = self.target_path().ok()?;
        let name = target.file_name()?.to_string_lossy().to_string();
        if let Some(b) = self.swapper.backup_for(&name) {
            return Some(self.swapper.backup_path(&b.file_name));
        }
        (!self.swapper.is_swapped_for(Some(&target))).then_some(target)
    }

    /// Start a transcode on a background thread; progress + final VideoItem via the sink.
    pub fn start_transcode(self: &Arc<Self>, input: String) -> Result<String> {
        let ffmpeg = ffmpeg::find_ffmpeg(&self.dirs.ffmpeg())
            .ok_or_else(|| anyhow!("ffmpeg is not installed"))?;
        if !Path::new(&input).is_file() {
            bail!("File not found: {input}");
        }
        let job = new_id();
        let me = self.clone();
        let job2 = job.clone();
        std::thread::Builder::new()
            .name(format!("transcode-{job}"))
            .spawn(
                move || match me.run_transcode(&job2, Path::new(&input), &ffmpeg) {
                    Ok(item) => {
                        me.emit_progress(ProgressEvent::finished(&job2, "Done", Some(item)))
                    }
                    Err(e) => {
                        log::error!("transcode {job2} failed: {e:#}");
                        me.emit_progress(ProgressEvent::failed(&job2, format!("{e:#}")));
                    }
                },
            )?;
        Ok(job)
    }

    fn run_transcode(&self, job: &str, input: &Path, ffmpeg_exe: &Path) -> Result<VideoItem> {
        self.emit_progress(ProgressEvent::step(job, 0.0, "Analyzing"));
        let ffprobe = ffmpeg::find_ffprobe(&self.dirs.ffmpeg());
        let in_probe = ffprobe.as_ref().and_then(|p| ffmpeg::probe(p, input).ok());
        let orig_probe = match (&ffprobe, self.original_video()) {
            (Some(p), Some(orig)) => ffmpeg::probe(p, &orig).ok(),
            _ => None,
        };
        let target = TranscodeTarget::from_probe(orig_probe.as_ref());
        let keep_audio = self.settings().keep_audio;
        let id = new_id();
        let out = self.library.file_for(&id);
        let tmp = out.with_file_name(format!("{id}.converting.mp4"));
        self.emit_progress(ProgressEvent::step(
            job,
            0.0,
            format!("Converting to {}x{} H.264", target.width, target.height),
        ));
        ffmpeg::transcode(
            ffmpeg_exe,
            input,
            &tmp,
            &target,
            keep_audio,
            in_probe.as_ref().and_then(|p| p.duration_secs),
            |pct| self.emit_progress(ProgressEvent::step(job, pct, "Converting")),
        )?;
        std::fs::rename(&tmp, &out).context("finalising converted video")?;
        self.emit_progress(ProgressEvent::step(job, 99.0, "Adding to library"));
        let out_probe = ffprobe.as_ref().and_then(|p| ffmpeg::probe(p, &out).ok());
        // Converting a library file (non-H.264 .mp4 imported first): keep its display name.
        let source_item = self
            .library
            .list()
            .into_iter()
            .find(|v| path_key(&v.path) == path_key(&to_display(input)));
        let name = source_item
            .as_ref()
            .map(|v| v.name.clone())
            .unwrap_or_else(|| {
                input
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| id.clone())
            });
        let item = self
            .library
            .add_existing(&id, &name, &out, out_probe.as_ref())?;
        let selected = self.settings().selected_video_id;
        if selected.is_none() || selected == source_item.map(|v| v.id) {
            let _ = self.update_settings(&json!({ "selectedVideoId": item.id }));
        }
        Ok(item)
    }

    // ----- watcher --------------------------------------------------------

    fn watch_config(&self) -> WatchConfig {
        let s = self.settings();
        WatchConfig {
            trigger: s.swap_trigger,
            delay: std::time::Duration::from_secs(s.swap_delay_secs as u64),
            armed: s.enabled && s.selected_video_id.is_some(),
            swapped: self.swapper.journal().swapped,
        }
    }

    /// On startup: if a swap was left behind and the game is not running, restore it.
    pub fn startup_recovery(&self, o: ProcSnapshot) {
        *self.procs.lock().unwrap() = o;
        let j = self.swapper.journal();
        if j.swapped && !o.game {
            log::info!("recovering: a swap was left active, restoring the original");
            let r = self.perform_restore();
            self.describe_restore(&r, false, true);
        }
        self.sync_swapped_flag();
    }

    /// One watcher poll.
    pub fn tick(&self, source: &mut dyn ProcessSource) {
        let o = source.snapshot();
        *self.procs.lock().unwrap() = o;
        let now = self.clock.elapsed();
        let cfg = self.watch_config();
        let action = self.machine.lock().unwrap().step(o, now, cfg);
        match action {
            Action::None => {}
            Action::Swap => {
                let r = self.perform_swap();
                self.describe_swap(&r, true);
                self.machine.lock().unwrap().on_swap_result(r.is_ok());
            }
            Action::Restore => {
                self.update_status(|s| s.phase = Phase::Restoring);
                let r = self.perform_restore();
                self.describe_restore(&r, true, true);
                self.machine
                    .lock()
                    .unwrap()
                    .on_restore_result(r.is_ok(), now, o);
            }
        }
        let phase = self.machine.lock().unwrap().phase;
        self.update_status(|s| {
            s.phase = phase;
            s.game_running = o.game;
            s.riot_client_running = o.riot_client_ui;
        });
    }

    /// Blocking watcher loop until `stop()`.
    pub fn run_watcher(&self, mut source: Box<dyn ProcessSource>) {
        self.refresh_install();
        self.detect_and_warn();
        let first = source.snapshot();
        self.startup_recovery(first);
        while !self.stop.load(Ordering::Relaxed) {
            self.tick(source.as_mut());
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Before exiting: restore if swapped and the game is not running.
    pub fn prepare_quit(&self) {
        self.stop();
        let game = self.procs.lock().unwrap().game;
        if !self.swapper.journal().swapped {
            return;
        }
        if game {
            self.report(
                NotifyLevel::Warning,
                "VALORANT is still running; the original video will be restored the next time Veil Launcher starts",
            );
        } else {
            let r = self.perform_restore();
            self.describe_restore(&r, false, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::watcher::FakeSource;
    use std::fs;

    #[derive(Default)]
    struct Recorder {
        statuses: Mutex<Vec<Status>>,
        notes: Mutex<Vec<NotifyEvent>>,
    }
    impl EventSink for Recorder {
        fn status_changed(&self, s: &Status) {
            self.statuses.lock().unwrap().push(s.clone());
        }
        fn notify(&self, e: &NotifyEvent, _: bool) {
            self.notes.lock().unwrap().push(e.clone());
        }
    }

    /// Full watcher cycle on temp dirs with a fake process source and an install override.
    #[test]
    fn end_to_end_cycle_on_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("Riot Games/VALORANT/live");
        let menu = paths::menu_dir_of(&live);
        fs::create_dir_all(&menu).unwrap();
        fs::write(menu.join("13_06_Homescreen.mp4"), b"ORIGINAL").unwrap();
        fs::write(menu.join("13_09_Seasonal_Prelude_Cinematic.mp4"), b"CINE").unwrap();
        let clip = tmp.path().join("clip.mp4");
        fs::write(&clip, b"MY-CUSTOM-CLIP").unwrap();

        let engine = Engine::new(DataDirs::new(tmp.path().join("data")), false).unwrap();
        let rec = Arc::new(Recorder::default());
        engine.set_sink(rec.clone());
        engine
            .set_install_override(Some(to_display(&live)))
            .unwrap();
        engine
            .update_settings(&json!({"swapTrigger": "gameStarted", "swapDelaySecs": 0}))
            .unwrap();
        let item = engine.import_video(&to_display(&clip)).unwrap();
        assert_eq!(
            engine.settings().selected_video_id.as_deref(),
            Some(item.id.as_str())
        );
        assert_eq!(
            engine.detect().selected.unwrap().file_name,
            "13_06_Homescreen.mp4"
        );

        let game = ProcSnapshot {
            riot_client_ui: false,
            game: true,
        };
        let none = ProcSnapshot::default();
        let mut src = FakeSource::new([none, game, game, none, none, none]);
        let target = menu.join("13_06_Homescreen.mp4");

        engine.tick(&mut src); // idle
        engine.tick(&mut src); // game → swap
        assert_eq!(fs::read(&target).unwrap(), b"MY-CUSTOM-CLIP");
        assert!(engine.status().swapped);
        assert_eq!(engine.status().phase, Phase::Swapped);
        engine.tick(&mut src);
        engine.tick(&mut src); // gone once (debounce)
        assert_eq!(fs::read(&target).unwrap(), b"MY-CUSTOM-CLIP");
        engine.tick(&mut src); // gone → restore
        assert_eq!(fs::read(&target).unwrap(), b"ORIGINAL");
        let st = engine.status();
        assert!(!st.swapped && st.has_backup);
        assert_eq!(st.phase, Phase::Idle);
        assert_eq!(
            fs::read(menu.join("13_09_Seasonal_Prelude_Cinematic.mp4")).unwrap(),
            b"CINE"
        );
        assert!(
            rec.notes
                .lock()
                .unwrap()
                .iter()
                .any(|n| n.level == NotifyLevel::Success)
        );
        assert!(!rec.statuses.lock().unwrap().is_empty());

        // Manual apply + crash + startup recovery.
        engine.apply_now(false).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"MY-CUSTOM-CLIP");
        drop(engine);
        let engine2 = Engine::new(DataDirs::new(tmp.path().join("data")), false).unwrap();
        assert!(engine2.status().swapped);
        engine2.startup_recovery(none);
        assert_eq!(fs::read(&target).unwrap(), b"ORIGINAL");
        assert!(!engine2.status().swapped);

        // Disabling while swapped restores.
        engine2.apply_now(false).unwrap();
        engine2.update_settings(&json!({"enabled": false})).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"ORIGINAL");
    }

    #[test]
    fn invalid_override_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = Engine::new(DataDirs::new(tmp.path().join("data")), false).unwrap();
        assert!(
            engine
                .set_install_override(Some(to_display(tmp.path())))
                .is_err()
        );
        assert!(engine.settings().install_override.is_none());
    }

    #[test]
    fn manual_actions_are_quiet_and_pin_missing_warns_once() {
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("VALORANT/live");
        let menu = paths::menu_dir_of(&live);
        fs::create_dir_all(&menu).unwrap();
        fs::write(menu.join("13_06_Homescreen.mp4"), b"ORIGINAL").unwrap();
        let clip = tmp.path().join("clip.mp4");
        fs::write(&clip, b"CLIP").unwrap();
        let engine = Engine::new(DataDirs::new(tmp.path().join("data")), false).unwrap();
        let rec = Arc::new(Recorder::default());
        engine.set_sink(rec.clone());
        engine
            .set_install_override(Some(to_display(&live)))
            .unwrap();
        engine.import_video(&to_display(&clip)).unwrap();

        engine.apply_now(false).unwrap();
        engine.restore_now(false).unwrap();
        engine
            .pin_target(Some("13_06_Homescreen.mp4".into()))
            .unwrap();
        assert!(
            rec.notes.lock().unwrap().is_empty(),
            "UI-initiated actions must not toast"
        );
        assert_eq!(
            engine.status().message.as_deref(),
            Some("Original video restored")
        );

        // A patch renames the file: the pin goes missing.
        fs::rename(
            menu.join("13_06_Homescreen.mp4"),
            menu.join("13_10_Homescreen.mp4"),
        )
        .unwrap();
        assert!(engine.detect().pin_missing);
        assert!(
            rec.notes.lock().unwrap().is_empty(),
            "plain detect stays quiet"
        );
        engine.detect_and_warn();
        engine.detect_and_warn();
        let notes = rec.notes.lock().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].level, NotifyLevel::Warning);
        assert!(notes[0].message.contains("13_10_Homescreen.mp4"));
    }
}
