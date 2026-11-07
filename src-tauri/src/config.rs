//! Settings and data-directory layout.
//!
//! Settings live in `%APPDATA%\VeilLauncher\config.json` as plain serde JSON (not
//! tauri-plugin-store) so `--helper` / `--cleanup` can read them without Tauri.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SwapTrigger {
    /// Also wait for the Riot Client window to close (falls back to a grace period if it stays open).
    RiotClientClosed,
    /// Default: Riot's file check finishes before the game process starts, so swap right away.
    #[default]
    GameStarted,
}

pub const MAX_SWAP_DELAY_SECS: u32 = 60;
pub const DEFAULT_ACCENT: &str = "#7c5cff";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme_mode: ThemeMode,
    pub accent: String,
    pub selected_video_id: Option<String>,
    pub enabled: bool,
    pub pinned_target: Option<String>,
    pub install_override: Option<String>,
    pub background_mode: bool,
    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub swap_trigger: SwapTrigger,
    pub swap_delay_secs: u32,
    pub keep_audio: bool,
    pub notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
            accent: DEFAULT_ACCENT.to_string(),
            selected_video_id: None,
            enabled: true,
            pinned_target: None,
            install_override: None,
            background_mode: false,
            start_with_windows: false,
            start_minimized: false,
            swap_trigger: SwapTrigger::GameStarted,
            swap_delay_secs: 2,
            keep_audio: false,
            notifications: true,
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl Settings {
    /// Clamp/repair values that the UI (or a hand-edited file) may have broken.
    pub fn sanitize(&mut self) {
        self.swap_delay_secs = self.swap_delay_secs.min(MAX_SWAP_DELAY_SECS);
        if !is_hex_color(&self.accent) {
            self.accent = DEFAULT_ACCENT.to_string();
        } else {
            self.accent = self.accent.to_ascii_lowercase();
        }
        for opt in [
            &mut self.selected_video_id,
            &mut self.pinned_target,
            &mut self.install_override,
        ] {
            if opt.as_deref().is_some_and(|s| s.trim().is_empty()) {
                *opt = None;
            }
        }
    }

    /// Merge a partial JSON patch (camelCase keys, as sent by the UI) over these settings.
    /// Unknown keys are ignored; wrongly typed values are an error.
    pub fn merged(&self, patch: &Value) -> Result<Settings> {
        let Value::Object(patch) = patch else {
            bail!("settings patch must be an object");
        };
        let mut base = serde_json::to_value(self)?;
        let obj = base
            .as_object_mut()
            .expect("settings serialize to an object");
        for (k, v) in patch {
            if !obj.contains_key(k) {
                log::warn!("ignoring unknown settings key {k}");
                continue;
            }
            let v = match (k.as_str(), v) {
                // Accept fractional / negative delays from sliders.
                ("swapDelaySecs", Value::Number(n)) => {
                    let f = n
                        .as_f64()
                        .unwrap_or(0.0)
                        .round()
                        .clamp(0.0, MAX_SWAP_DELAY_SECS as f64);
                    Value::from(f as u32)
                }
                _ => v.clone(),
            };
            obj.insert(k.clone(), v);
        }
        let mut next: Settings = serde_json::from_value(base).context("invalid settings value")?;
        if let Some(a) = patch.get("accent").and_then(Value::as_str)
            && !is_hex_color(a)
        {
            bail!("accent must be a #rrggbb color");
        }
        next.sanitize();
        Ok(next)
    }
}

/// `%APPDATA%\VeilLauncher\` and its sub folders.
#[derive(Debug, Clone)]
pub struct DataDirs {
    pub root: PathBuf,
}

impl DataDirs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The per-user default location (`%APPDATA%\VeilLauncher`).
    pub fn default_location() -> Self {
        let base = dirs::config_dir()
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(base.join("VeilLauncher"))
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.json")
    }
    pub fn state_file(&self) -> PathBuf {
        self.root.join("state.json")
    }
    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }
    pub fn videos(&self) -> PathBuf {
        self.root.join("videos")
    }
    pub fn library_file(&self) -> PathBuf {
        self.videos().join("library.json")
    }
    pub fn ffmpeg(&self) -> PathBuf {
        self.root.join("ffmpeg")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn ensure(&self) -> Result<()> {
        for d in [
            self.root.clone(),
            self.backups(),
            self.videos(),
            self.ffmpeg(),
            self.logs(),
        ] {
            fs::create_dir_all(&d).with_context(|| format!("creating {}", d.display()))?;
        }
        Ok(())
    }

    pub fn load_settings(&self) -> Settings {
        let path = self.config_file();
        match read_json::<Settings>(&path) {
            Ok(Some(mut s)) => {
                s.sanitize();
                s
            }
            Ok(None) => Settings::default(),
            Err(e) => {
                log::error!("config.json unreadable ({e:#}); using defaults");
                let _ = fs::rename(&path, path.with_extension("json.bad"));
                Settings::default()
            }
        }
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        write_json_atomic(&self.config_file(), s)
    }
}

/// Read a JSON file. `Ok(None)` if it does not exist.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => {
            // Tolerate a UTF-8 BOM (e.g. a file hand-edited in Notepad or written by
            // Windows PowerShell), so it doesn't silently reset settings or the swap journal.
            let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
            Ok(Some(
                serde_json::from_slice(body)
                    .with_context(|| format!("parsing {}", path.display()))?,
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// Write JSON via a temp file + rename so a crash never leaves a half-written file.
pub fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stale_keys_in_config_are_ignored() {
        // Builds that briefly had a background picker saved a "backdrop" key; it must not
        // break loading (which would reset every setting to defaults).
        let s: Settings =
            serde_json::from_value(json!({ "accent": "#ec4899", "backdrop": "lines" })).unwrap();
        assert_eq!(s.accent, "#ec4899");
    }

    #[test]
    fn read_json_accepts_utf8_bom() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.json");
        fs::write(&p, b"\xEF\xBB\xBF{\"accent\":\"#ec4899\"}").unwrap();
        let v: serde_json::Value = read_json(&p).unwrap().unwrap();
        assert_eq!(v["accent"], "#ec4899");
    }

    #[test]
    fn defaults_match_contract() {
        let s = Settings::default();
        assert_eq!(s.theme_mode, ThemeMode::System);
        assert_eq!(s.accent, "#7c5cff");
        assert!(s.enabled);
        assert_eq!(s.swap_trigger, SwapTrigger::GameStarted);
        assert_eq!(s.swap_delay_secs, 2);
        assert!(!s.keep_audio);
        assert!(s.notifications);
        assert!(!s.background_mode && !s.start_with_windows && !s.start_minimized);
        assert!(
            s.selected_video_id.is_none()
                && s.pinned_target.is_none()
                && s.install_override.is_none()
        );

        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["themeMode"], "system");
        assert_eq!(v["swapTrigger"], "gameStarted");
        assert_eq!(v["swapDelaySecs"], 2);
        assert_eq!(v["selectedVideoId"], Value::Null);
    }

    #[test]
    fn merge_patch_updates_only_given_keys() {
        let s = Settings::default();
        let n = s
            .merged(&json!({"themeMode": "dark", "swapTrigger": "gameStarted", "bogus": 1}))
            .unwrap();
        assert_eq!(n.theme_mode, ThemeMode::Dark);
        assert_eq!(n.swap_trigger, SwapTrigger::GameStarted);
        assert_eq!(n.accent, s.accent);
        assert!(n.enabled);
    }

    #[test]
    fn merge_clamps_delay_and_validates() {
        let s = Settings::default();
        assert_eq!(
            s.merged(&json!({"swapDelaySecs": 600}))
                .unwrap()
                .swap_delay_secs,
            60
        );
        assert_eq!(
            s.merged(&json!({"swapDelaySecs": 2.6}))
                .unwrap()
                .swap_delay_secs,
            3
        );
        assert_eq!(
            s.merged(&json!({"swapDelaySecs": -4}))
                .unwrap()
                .swap_delay_secs,
            0
        );
        assert!(s.merged(&json!({"accent": "red"})).is_err());
        assert!(s.merged(&json!({"enabled": "yes"})).is_err());
        assert!(s.merged(&json!([1, 2])).is_err());
        assert_eq!(
            s.merged(&json!({"accent": "#ABCDEF"})).unwrap().accent,
            "#abcdef"
        );
        assert_eq!(
            s.merged(&json!({"pinnedTarget": "  "}))
                .unwrap()
                .pinned_target,
            None
        );
        assert_eq!(
            s.merged(&json!({"selectedVideoId": "abc"}))
                .unwrap()
                .selected_video_id
                .as_deref(),
            Some("abc")
        );
        assert_eq!(
            s.merged(&json!({"selectedVideoId": null}))
                .unwrap()
                .selected_video_id,
            None
        );
    }

    #[test]
    fn partial_file_fills_defaults_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let dd = DataDirs::new(dir.path());
        dd.ensure().unwrap();
        fs::write(
            dd.config_file(),
            r##"{"accent":"#112233","enabled":false}"##,
        )
        .unwrap();
        let s = dd.load_settings();
        assert_eq!(s.accent, "#112233");
        assert!(!s.enabled);
        assert_eq!(s.swap_delay_secs, 2);
        assert!(s.notifications);

        let mut s2 = s.clone();
        s2.keep_audio = true;
        dd.save_settings(&s2).unwrap();
        assert_eq!(dd.load_settings(), s2);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let dd = DataDirs::new(dir.path());
        dd.ensure().unwrap();
        fs::write(dd.config_file(), "{not json").unwrap();
        assert_eq!(dd.load_settings(), Settings::default());
        assert!(dd.root.join("config.json.bad").exists());
    }
}
