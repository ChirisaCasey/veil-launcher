//! Veil Launcher: swaps VALORANT's main-menu background video after Riot's file check and
//! restores the original when the game exits. File swapping only: nothing is injected,
//! hooked, read from game memory, or overlaid.

pub mod cli;
pub mod commands;
pub mod config;
pub mod core;
pub mod engine;
pub mod helper;
pub mod logger;
pub mod startup;
pub mod tray;

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;

use crate::cli::Mode;
use crate::config::{DataDirs, Settings};
use crate::core::ffmpeg::ProgressEvent;
use crate::core::watcher::SysinfoSource;
use crate::engine::{Engine, EventSink, NotifyEvent, NotifyLevel, Status};

pub const EVENT_STATUS: &str = "status-changed";
pub const EVENT_PROGRESS: &str = "progress";
pub const EVENT_NOTIFY: &str = "notify";
pub const EVENT_SETTINGS: &str = "settings-changed";

/// Bridges engine events to the webview, tray and desktop notifications.
struct TauriSink {
    app: AppHandle,
}

impl TauriSink {
    fn window_in_front(&self) -> bool {
        self.app
            .get_webview_window("main")
            .map(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
            .unwrap_or(false)
    }
}

impl EventSink for TauriSink {
    fn status_changed(&self, status: &Status) {
        let _ = self.app.emit(EVENT_STATUS, status);
        tray::set_tooltip(&self.app, status);
    }

    fn settings_changed(&self, settings: &Settings) {
        let _ = self.app.emit(EVENT_SETTINGS, settings);
        tray::set_enabled_checked(&self.app, settings.enabled);
    }

    fn notify(&self, event: &NotifyEvent, desktop: bool) {
        let _ = self.app.emit(EVENT_NOTIFY, event);
        if desktop && event.level != NotifyLevel::Info && !self.window_in_front() {
            let _ = self
                .app
                .notification()
                .builder()
                .title("Veil Launcher")
                .body(&event.message)
                .show();
        }
    }

    fn progress(&self, event: &ProgressEvent) {
        let _ = self.app.emit(EVENT_PROGRESS, event);
    }
}

/// When running elevated (background scheduled task), Windows UIPI drops messages from
/// unelevated processes, so a normal Start-menu launch couldn't reach this instance and the
/// window would never appear. tauri-plugin-single-instance hands off via WM_COPYDATA; allow it.
fn allow_unelevated_handoff() {
    #[cfg(windows)]
    if core::elevate::is_elevated() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            ChangeWindowMessageFilter, MSGFLT_ADD, WM_COPYDATA,
        };
        // SAFETY: plain Win32 call with constant arguments.
        let ok = unsafe { ChangeWindowMessageFilter(WM_COPYDATA, MSGFLT_ADD) };
        if ok == 0 {
            log::warn!("could not allow WM_COPYDATA from unelevated processes");
        }
    }
}

/// Entry point. Returns the process exit code.
pub fn run() -> i32 {
    let args = cli::parse(std::env::args().skip(1));
    match &args.mode {
        Mode::Helper { op, args } => return helper::run_helper(op, args),
        Mode::Cleanup => return helper::run_cleanup(),
        Mode::App => {}
    }

    let dirs = DataDirs::default_location();
    logger::init(&dirs.logs(), "app");
    log::info!(
        "Veil Launcher {} starting (args: {:?})",
        env!("CARGO_PKG_VERSION"),
        args
    );
    let engine = match Engine::new(dirs, true) {
        Ok(e) => e,
        Err(e) => {
            log::error!("cannot initialise: {e:#}");
            return 1;
        }
    };
    let start_hidden = args.background || args.minimized || engine.settings().start_minimized;
    let setup_engine = engine.clone();
    allow_unelevated_handoff();

    let result = tauri::Builder::default()
        // Must be first: a second launch focuses the existing instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let a = cli::parse(argv.iter().skip(1));
            // A second `--background` start (e.g. autostart + scheduled task) stays silent.
            if !(a.background || a.minimized) {
                tray::show_main(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(engine)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_settings,
            commands::update_settings,
            commands::get_install_info,
            commands::set_install_override,
            commands::detect_targets,
            commands::pin_target,
            commands::list_videos,
            commands::import_video,
            commands::remove_video,
            commands::select_video,
            commands::get_status,
            commands::launch_game,
            commands::apply_now,
            commands::restore_now,
            commands::ffmpeg_status,
            commands::install_ffmpeg,
            commands::transcode_video,
            commands::hide_to_tray,
            commands::quit_app,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let engine = setup_engine;
            engine.set_sink(Arc::new(TauriSink {
                app: handle.clone(),
            }));
            tray::build(&handle, &engine)?;
            if !start_hidden {
                tray::show_main(&handle);
            }

            let watcher = engine.clone();
            std::thread::Builder::new()
                .name("watcher".into())
                .spawn(move || watcher.run_watcher(Box::new(SysinfoSource::new())))?;

            // schtasks /Query spawns a process; keep it off the main thread.
            let h = handle.clone();
            std::thread::spawn(move || engine.set_background_installed(startup::installed(&h)));
            Ok(())
        })
        .run(tauri::generate_context!());

    match result {
        Ok(()) => 0,
        Err(e) => {
            log::error!("tauri error: {e}");
            1
        }
    }
}
