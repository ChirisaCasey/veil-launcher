//! Tauri commands: the Rust side of `src/lib/api.ts`. Errors are plain strings.

use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use crate::config::Settings;
use crate::core::detect::DetectionResult;
use crate::core::elevate;
use crate::core::ffmpeg::{self, FfmpegStatus, ProgressEvent};
use crate::core::launch;
use crate::core::library::VideoItem;
use crate::core::paths::{InstallInfo, to_display};
use crate::engine::{Engine, Status};
use crate::startup;

type Eng<'a> = State<'a, Arc<Engine>>;
type CmdResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub elevated: bool,
    pub data_dir: String,
}

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// Run blocking engine work off the async runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let engine = app.state::<Arc<Engine>>();
    AppInfo {
        version: app.package_info().version.to_string(),
        elevated: elevate::is_elevated(),
        data_dir: to_display(&engine.dirs.root),
    }
}

#[tauri::command]
pub fn get_settings(engine: Eng<'_>) -> Settings {
    engine.settings()
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, engine: Eng<'_>, patch: Value) -> CmdResult<Settings> {
    let engine = engine.inner().clone();
    blocking(move || {
        let (new, old) = engine.update_settings(&patch)?;
        let startup_changed = new.background_mode != old.background_mode
            || new.start_with_windows != old.start_with_windows;
        if startup_changed {
            match startup::sync(&app, &new, &engine.install_info()) {
                Ok(installed) => engine.set_background_installed(installed),
                Err(e) => {
                    // Roll the startup flags back so the UI reflects reality.
                    let _ = engine.update_settings(&json!({
                        "backgroundMode": old.background_mode,
                        "startWithWindows": old.start_with_windows,
                    }));
                    engine.set_background_installed(startup::installed(&app));
                    return Err(e.context("Couldn't change start-up settings"));
                }
            }
        }
        Ok(engine.settings())
    })
    .await
}

#[tauri::command]
pub async fn get_install_info(engine: Eng<'_>) -> CmdResult<InstallInfo> {
    let engine = engine.inner().clone();
    blocking(move || Ok(engine.refresh_install())).await
}

#[tauri::command]
pub async fn set_install_override(engine: Eng<'_>, path: Option<String>) -> CmdResult<InstallInfo> {
    let engine = engine.inner().clone();
    blocking(move || engine.set_install_override(path)).await
}

#[tauri::command]
pub async fn detect_targets(engine: Eng<'_>) -> CmdResult<DetectionResult> {
    let engine = engine.inner().clone();
    blocking(move || Ok(engine.detect())).await
}

#[tauri::command]
pub async fn pin_target(engine: Eng<'_>, file_name: Option<String>) -> CmdResult<DetectionResult> {
    let engine = engine.inner().clone();
    blocking(move || engine.pin_target(file_name)).await
}

#[tauri::command]
pub fn list_videos(engine: Eng<'_>) -> Vec<VideoItem> {
    engine.library.list()
}

#[tauri::command]
pub async fn import_video(engine: Eng<'_>, path: String) -> CmdResult<VideoItem> {
    let engine = engine.inner().clone();
    blocking(move || engine.import_video(&path)).await
}

#[tauri::command]
pub async fn remove_video(engine: Eng<'_>, id: String) -> CmdResult<()> {
    let engine = engine.inner().clone();
    blocking(move || engine.remove_video(&id)).await
}

#[tauri::command]
pub async fn select_video(engine: Eng<'_>, id: Option<String>) -> CmdResult<Settings> {
    let engine = engine.inner().clone();
    blocking(move || engine.select_video(id)).await
}

#[tauri::command]
pub fn get_status(engine: Eng<'_>) -> Status {
    engine.status()
}

#[tauri::command]
pub async fn launch_game(engine: Eng<'_>) -> CmdResult<()> {
    let engine = engine.inner().clone();
    blocking(move || launch::launch_valorant(engine.install_info().riot_client.as_deref())).await
}

#[tauri::command]
pub async fn apply_now(engine: Eng<'_>) -> CmdResult<Status> {
    let engine = engine.inner().clone();
    blocking(move || engine.apply_now(false)).await
}

#[tauri::command]
pub async fn restore_now(engine: Eng<'_>) -> CmdResult<Status> {
    let engine = engine.inner().clone();
    blocking(move || engine.restore_now(false)).await
}

#[tauri::command]
pub async fn ffmpeg_status(engine: Eng<'_>) -> CmdResult<FfmpegStatus> {
    let dir = engine.dirs.ffmpeg();
    blocking(move || Ok(ffmpeg::status(&dir))).await
}

#[tauri::command]
pub async fn install_ffmpeg(engine: Eng<'_>) -> CmdResult<FfmpegStatus> {
    let engine = engine.inner().clone();
    let dir = engine.dirs.ffmpeg();
    let e2 = engine.clone();
    match ffmpeg::install(&dir, move |ev| e2.emit_progress(ev)).await {
        Ok(st) => Ok(st),
        Err(e) => {
            let msg = err(e);
            engine.emit_progress(ProgressEvent::failed(ffmpeg::FFMPEG_JOB_ID, msg.clone()));
            Err(msg)
        }
    }
}

#[tauri::command]
pub fn transcode_video(engine: Eng<'_>, path: String) -> CmdResult<String> {
    engine.inner().start_transcode(path).map_err(err)
}

#[tauri::command]
pub fn hide_to_tray(app: AppHandle) -> CmdResult<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle, engine: Eng<'_>) -> CmdResult<()> {
    let engine = engine.inner().clone();
    blocking(move || {
        engine.prepare_quit();
        Ok(())
    })
    .await?;
    app.exit(0);
    Ok(())
}
