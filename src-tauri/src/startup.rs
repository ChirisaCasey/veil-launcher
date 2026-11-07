//! Start-with-Windows / background integration.
//!
//! * Menu folder writable without admin → HKCU Run entry (tauri-plugin-autostart, `--background`).
//! * Otherwise → logon scheduled task with highest privileges (one UAC prompt to create it),
//!   so background swaps never need a prompt.
//! * Disabling removes both.

use anyhow::{Result, anyhow};
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::config::Settings;
use crate::core::elevate;
use crate::core::paths::InstallInfo;

pub fn wants_startup(s: &Settings) -> bool {
    s.background_mode || s.start_with_windows
}

pub fn autostart_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

pub fn installed(app: &AppHandle) -> bool {
    autostart_enabled(app) || elevate::task_exists()
}

/// Bring the OS startup entries in line with `s`. Returns whether something is installed.
pub fn sync(app: &AppHandle, s: &Settings, info: &InstallInfo) -> Result<bool> {
    let al = app.autolaunch();
    if wants_startup(s) {
        let task = elevate::task_exists();
        let needs_admin = info.menu_dir.is_some() && !info.writable && !elevate::is_elevated();
        if task || needs_admin {
            if !task {
                elevate::install_task()?;
            }
            if autostart_enabled(app) {
                al.disable()
                    .map_err(|e| anyhow!("disabling autostart: {e}"))?;
            }
        } else if !autostart_enabled(app) {
            al.enable()
                .map_err(|e| anyhow!("enabling autostart: {e}"))?;
        }
    } else {
        if autostart_enabled(app) {
            al.disable()
                .map_err(|e| anyhow!("disabling autostart: {e}"))?;
        }
        elevate::remove_task()?;
    }
    Ok(installed(app))
}
