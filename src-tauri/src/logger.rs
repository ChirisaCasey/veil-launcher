//! Tiny file logger: `%APPDATA%\VeilLauncher\logs\veil.log`, rotated at 2 MB.

use log::{LevelFilter, Metadata, Record};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_BYTES: u64 = 2 * 1024 * 1024;

struct FileLogger {
    file: Mutex<Option<File>>,
    tag: &'static str,
}

impl log::Log for FileLogger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= log::Level::Debug
            && (m.target().starts_with("veil") || m.level() <= log::Level::Warn)
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let line = format!(
            "{secs} [{}] {:<5} {}: {}\n",
            self.tag,
            r.level(),
            r.target(),
            r.args()
        );
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
        if let Some(f) = self.file.lock().unwrap().as_mut() {
            let _ = f.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        if let Some(f) = self.file.lock().unwrap().as_mut() {
            let _ = f.flush();
        }
    }
}

fn open(dir: &Path) -> Option<File> {
    fs::create_dir_all(dir).ok()?;
    let path: PathBuf = dir.join("veil.log");
    if fs::metadata(&path)
        .map(|m| m.len() > MAX_BYTES)
        .unwrap_or(false)
    {
        let _ = fs::rename(&path, dir.join("veil.old.log"));
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Initialise once; later calls are ignored. `tag` distinguishes app / helper / cleanup lines.
pub fn init(dir: &Path, tag: &'static str) {
    let logger = FileLogger {
        file: Mutex::new(open(dir)),
        tag,
    };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }
}
