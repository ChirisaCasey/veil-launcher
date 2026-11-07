//! `--helper <op> <args>` (elevated one-shot ops) and `--cleanup` (uninstaller). No Tauri.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::path::Path;

use crate::config::DataDirs;
use crate::core::elevate::{
    self, HELPER_FAILED, HELPER_OK, HELPER_USAGE, HELPER_VERIFY_FAILED, decode_args,
};
use crate::core::hash::sha256_file;
use crate::core::swap::{WRITE_TIMEOUT, replace_with_retry};
use crate::core::watcher::{ProcessSource, SysinfoSource};
use crate::engine::Engine;
use crate::logger;

/// The elevated helper may only ever write `.mp4` files inside a `...\Movies\Menu` folder.
pub fn validate_game_target(target: &Path) -> Result<()> {
    let ext_ok = target
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"));
    let parent = target.parent();
    let menu_ok = parent
        .and_then(Path::file_name)
        .is_some_and(|n| n.eq_ignore_ascii_case("Menu"));
    let movies_ok = parent
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .is_some_and(|n| n.eq_ignore_ascii_case("Movies"));
    if !(ext_ok && menu_ok && movies_ok) {
        bail!(
            "refusing to write outside a VALORANT Movies\\Menu folder: {}",
            target.display()
        );
    }
    Ok(())
}

fn str_arg<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v.get(k)
        .and_then(Value::as_str)
        .with_context(|| format!("missing '{k}'"))
}

fn helper_replace(v: &Value) -> Result<i32> {
    let source = Path::new(str_arg(v, "source")?);
    let target = Path::new(str_arg(v, "target")?);
    validate_game_target(target)?;
    if !source.is_file() {
        bail!("source not found: {}", source.display());
    }
    let expected = match v.get("sha256").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => sha256_file(source)?,
    };
    replace_with_retry(source, target, WRITE_TIMEOUT)?;
    if sha256_file(target)? != expected {
        log::error!("helper: verification failed for {}", target.display());
        return Ok(HELPER_VERIFY_FAILED);
    }
    Ok(HELPER_OK)
}

pub fn run_helper(op: &str, args: &str) -> i32 {
    logger::init(&DataDirs::default_location().logs(), "helper");
    log::info!("helper op {op} (elevated: {})", elevate::is_elevated());
    let v = match decode_args(args) {
        Ok(v) => v,
        Err(e) => {
            log::error!("helper: bad args: {e:#}");
            return HELPER_USAGE;
        }
    };
    let r = match op {
        "swap" | "restore" => helper_replace(&v),
        // Always our own exe: never let the helper register an arbitrary program.
        "install-task" => std::env::current_exe()
            .context("locating exe")
            .and_then(|exe| elevate::create_task_direct(&exe))
            .map(|_| HELPER_OK),
        "remove-task" => elevate::delete_task_direct().map(|_| HELPER_OK),
        _ => {
            log::error!("helper: unknown op {op}");
            return HELPER_USAGE;
        }
    };
    match r {
        Ok(code) => code,
        Err(e) => {
            log::error!("helper {op} failed: {e:#}");
            HELPER_FAILED
        }
    }
}

#[cfg(windows)]
fn remove_run_entries() {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let approved = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    let Ok(key) = hkcu.open_subkey_with_flags(run, KEY_READ | KEY_SET_VALUE) else {
        return;
    };
    let names: Vec<String> = key
        .enum_values()
        .filter_map(|r| r.ok())
        .filter(|(_, val)| val.to_string().to_lowercase().contains("veil-launcher.exe"))
        .map(|(n, _)| n)
        .collect();
    let approved_key = hkcu.open_subkey_with_flags(approved, KEY_SET_VALUE).ok();
    for n in names {
        log::info!("cleanup: removing Run entry {n}");
        let _ = key.delete_value(&n);
        if let Some(a) = &approved_key {
            let _ = a.delete_value(&n);
        }
    }
}

#[cfg(not(windows))]
fn remove_run_entries() {}

/// Uninstaller cleanup: restore the original if swapped, remove the scheduled task and
/// autostart entry. Never creates the data dir if it doesn't exist.
pub fn run_cleanup() -> i32 {
    let dirs = DataDirs::default_location();
    let mut code = 0;
    if dirs.root.is_dir() {
        logger::init(&dirs.logs(), "cleanup");
        match Engine::new(dirs, true) {
            Ok(engine) if engine.swapper.journal().swapped => {
                engine.refresh_install();
                let game = SysinfoSource::new().snapshot().game;
                if game {
                    log::warn!("cleanup: VALORANT is running; trying to restore anyway");
                }
                if let Err(e) = engine.perform_restore() {
                    log::error!("cleanup: restore failed: {e:#}");
                    code = 1;
                }
            }
            Ok(_) => {}
            Err(e) => log::error!("cleanup: {e:#}"),
        }
    }
    if let Err(e) = elevate::remove_task() {
        log::error!("cleanup: removing scheduled task failed: {e:#}");
    }
    remove_run_entries();
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_target_validation() {
        assert!(
            validate_game_target(Path::new(
                r"D:\Riot Games\VALORANT\live\ShooterGame\Content\Movies\Menu\13_06_Homescreen.mp4"
            ))
            .is_ok()
        );
        assert!(validate_game_target(Path::new("D:/x/Movies/menu/a.MP4")).is_ok());
        assert!(validate_game_target(Path::new(r"C:\Windows\System32\evil.dll")).is_err());
        assert!(validate_game_target(Path::new(r"C:\x\Menu\a.mp4")).is_err());
        assert!(validate_game_target(Path::new(r"C:\x\Movies\Menu\a.exe")).is_err());
    }

    #[test]
    fn helper_replace_on_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let menu = tmp.path().join("Movies").join("Menu");
        std::fs::create_dir_all(&menu).unwrap();
        let target = menu.join("Homescreen.mp4");
        std::fs::write(&target, b"orig").unwrap();
        let src = tmp.path().join("c.mp4");
        std::fs::write(&src, b"custom").unwrap();
        let v = serde_json::json!({"source": src.to_string_lossy(), "target": target.to_string_lossy()});
        assert_eq!(helper_replace(&v).unwrap(), HELPER_OK);
        assert_eq!(std::fs::read(&target).unwrap(), b"custom");
        let v = serde_json::json!({"source": src.to_string_lossy(), "target": target.to_string_lossy(), "sha256": "00"});
        assert_eq!(helper_replace(&v).unwrap(), HELPER_VERIFY_FAILED);
        let v = serde_json::json!({"source": src.to_string_lossy(), "target": tmp.path().join("x.mp4").to_string_lossy()});
        assert!(helper_replace(&v).is_err());
    }
}
