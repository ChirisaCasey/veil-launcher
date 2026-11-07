//! Backup / swap / restore of the menu video.
//!
//! Invariants:
//! * A backup is only ever (re)written from a file whose hash is NOT a known custom video.
//! * The journal (`state.json`) is written before the game file is touched, so a crash
//!   mid-swap is recovered on next start (restore is idempotent).
//! * Every write into the game folder is copy-to-temp-in-same-dir + atomic replace,
//!   verified by SHA-256 afterwards.

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::hash::HashCache;
use super::paths::{path_key, to_display};
use crate::config::{DataDirs, now_ms, read_json, write_json_atomic};

pub const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Journal + manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Journal {
    pub swapped: bool,
    pub target_path: Option<String>,
    pub original_sha: Option<String>,
    pub custom_sha: Option<String>,
    /// Unix ms
    pub at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub file_name: String,
    pub sha256: String,
    pub size: u64,
    pub source_path: String,
    /// Unix ms
    pub backed_up_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BackupManifest {
    /// Keyed by lowercase file name.
    pub entries: BTreeMap<String, BackupEntry>,
}

// ---------------------------------------------------------------------------
// Atomic replace with retry
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ReplaceError {
    /// We cannot create files in the destination folder: elevation is needed.
    #[error("access denied writing to {0}")]
    AccessDenied(String, #[source] io::Error),
    /// The file stayed locked by another process for the whole retry window.
    #[error("{0} is in use by another process")]
    InUse(String, #[source] io::Error),
    #[error("{0}")]
    Io(String, #[source] io::Error),
}

const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;

fn is_lock_error(e: &io::Error) -> bool {
    matches!(
        e.raw_os_error(),
        Some(ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION)
    )
}

fn is_access_denied(e: &io::Error) -> bool {
    e.raw_os_error() == Some(ERROR_ACCESS_DENIED) || e.kind() == io::ErrorKind::PermissionDenied
}

pub fn temp_path_for(dst: &Path) -> PathBuf {
    let name = dst
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    dst.with_file_name(format!(".{name}.veil-tmp"))
}

#[cfg(windows)]
fn move_replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let w = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    let (a, b) = (w(from), w(to));
    let ok = unsafe {
        MoveFileExW(
            a.as_ptr(),
            b.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn move_replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

/// Overwrite `dst` in place (fallback when the rename is blocked but the file is writable).
fn overwrite_in_place(src: &Path, dst: &Path) -> io::Result<()> {
    let mut input = fs::File::open(src)?;
    let mut out = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(dst)?;
    io::copy(&mut input, &mut out)?;
    out.sync_all()
}

/// One attempt: copy `src` next to `dst`, then atomically move it over `dst`.
fn replace_once(src: &Path, dst: &Path) -> Result<(), ReplaceError> {
    let tmp = temp_path_for(dst);
    let shown = to_display(dst);
    let _ = fs::remove_file(&tmp);
    if let Err(e) = fs::copy(src, &tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(if is_access_denied(&e) {
            ReplaceError::AccessDenied(shown, e)
        } else {
            ReplaceError::Io(format!("copying {} next to {shown}", to_display(src)), e)
        });
    }
    let result = match move_replace(&tmp, dst) {
        Ok(()) => Ok(()),
        // We could create the temp file, so access denied here means the target is in use
        // (open without FILE_SHARE_DELETE) rather than a permission problem.
        Err(e) if is_lock_error(&e) || is_access_denied(&e) => {
            match overwrite_in_place(&tmp, dst) {
                Ok(()) => Ok(()),
                Err(_) => Err(ReplaceError::InUse(shown, e)),
            }
        }
        Err(e) => Err(ReplaceError::Io(format!("replacing {shown}"), e)),
    };
    let _ = fs::remove_file(&tmp);
    result
}

/// Replace `dst` with a copy of `src`, retrying with backoff while the file is locked.
pub fn replace_with_retry(src: &Path, dst: &Path, timeout: Duration) -> Result<(), ReplaceError> {
    let deadline = Instant::now() + timeout;
    let mut delay = Duration::from_millis(100);
    loop {
        match replace_once(src, dst) {
            Ok(()) => return Ok(()),
            Err(ReplaceError::InUse(s, e)) if Instant::now() + delay < deadline => {
                log::debug!("{s} locked ({e}); retrying in {delay:?}");
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_secs(2));
            }
            Err(e) => return Err(e),
        }
    }
}

/// Abstraction over "write this file into the game folder" so the app can fall back to an
/// elevated helper while tests write directly into temp dirs.
pub trait FileWriter: Send + Sync {
    fn replace(&self, src: &Path, dst: &Path) -> Result<(), ReplaceError>;
}

pub struct DirectWriter {
    pub timeout: Duration,
}

impl Default for DirectWriter {
    fn default() -> Self {
        Self {
            timeout: WRITE_TIMEOUT,
        }
    }
}

impl FileWriter for DirectWriter {
    fn replace(&self, src: &Path, dst: &Path) -> Result<(), ReplaceError> {
        replace_with_retry(src, dst, self.timeout)
    }
}

// ---------------------------------------------------------------------------
// Swapper
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapOutcome {
    Swapped,
    AlreadyApplied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreOutcome {
    Restored,
    AlreadyOriginal,
    /// The game file holds new genuine content (patch / Riot repair): kept it and re-backed it up.
    KeptGenuine,
}

pub struct Swapper {
    dirs: DataDirs,
    cache: Arc<HashCache>,
}

impl Swapper {
    pub fn new(dirs: DataDirs, cache: Arc<HashCache>) -> Self {
        Self { dirs, cache }
    }

    pub fn cache(&self) -> &HashCache {
        &self.cache
    }

    pub fn journal(&self) -> Journal {
        match read_json::<Journal>(&self.dirs.state_file()) {
            Ok(j) => j.unwrap_or_default(),
            Err(e) => {
                log::error!("state.json unreadable: {e:#}");
                Journal::default()
            }
        }
    }

    fn write_journal(&self, j: &Journal) -> Result<()> {
        write_json_atomic(&self.dirs.state_file(), j)
    }

    fn clear_journal(&self) -> Result<()> {
        let mut j = self.journal();
        j.swapped = false;
        j.at = Some(now_ms());
        self.write_journal(&j)
    }

    fn manifest_path(&self) -> PathBuf {
        self.dirs.backups().join("manifest.json")
    }

    pub fn manifest(&self) -> BackupManifest {
        read_json(&self.manifest_path())
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn backup_path(&self, file_name: &str) -> PathBuf {
        self.dirs.backups().join(file_name)
    }

    /// The manifest entry for `file_name`, if the backup file exists.
    pub fn backup_for(&self, file_name: &str) -> Option<BackupEntry> {
        let e = self
            .manifest()
            .entries
            .get(&file_name.to_lowercase())
            .cloned()?;
        self.backup_path(&e.file_name).is_file().then_some(e)
    }

    pub fn has_backup(&self, file_name: &str) -> bool {
        self.backup_for(file_name).is_some()
    }

    /// Make sure `backups/<name>` holds the genuine file at `target` (hash `target_sha`).
    /// Refuses to back up anything matching a known custom hash.
    pub fn ensure_backup(
        &self,
        target: &Path,
        target_sha: &str,
        forbidden: &[&str],
    ) -> Result<BackupEntry> {
        if forbidden.contains(&target_sha) {
            bail!("refusing to back up a custom video as the original");
        }
        let name = file_name(target)?;
        if let Some(e) = self.backup_for(&name)
            && e.sha256 == target_sha
            && self
                .cache
                .hash(&self.backup_path(&e.file_name))
                .ok()
                .as_deref()
                == Some(target_sha)
        {
            return Ok(e);
        }
        fs::create_dir_all(self.dirs.backups())?;
        let dest = self.backup_path(&name);
        let tmp = dest.with_file_name(format!("{name}.tmp"));
        fs::copy(target, &tmp).with_context(|| format!("backing up {}", target.display()))?;
        let got = self.cache.hash_fresh(&tmp)?;
        if got != target_sha {
            let _ = fs::remove_file(&tmp);
            bail!("backup verification failed (file changed while copying)");
        }
        if dest.exists() {
            // Keep the previous original around, just in case.
            let prev = dest.with_file_name(format!("{name}.prev"));
            let _ = fs::rename(&dest, &prev);
        }
        fs::rename(&tmp, &dest).context("finalising backup")?;
        self.cache.invalidate(&tmp);
        let entry = BackupEntry {
            file_name: name.clone(),
            sha256: target_sha.to_string(),
            size: fs::metadata(&dest)?.len(),
            source_path: to_display(target),
            backed_up_at: now_ms(),
        };
        let mut m = self.manifest();
        m.entries.insert(name.to_lowercase(), entry.clone());
        write_json_atomic(&self.manifest_path(), &m)?;
        log::info!(
            "backed up original {} ({})",
            name,
            &target_sha[..12.min(target_sha.len())]
        );
        Ok(entry)
    }

    /// Put `custom` in place of `target`, backing up the genuine original first.
    pub fn swap_in(
        &self,
        target: &Path,
        custom: &Path,
        writer: &dyn FileWriter,
    ) -> Result<SwapOutcome> {
        if !custom.is_file() {
            bail!("custom video not found: {}", custom.display());
        }
        if !target.is_file() {
            bail!("target video not found: {}", target.display());
        }
        let name = file_name(target)?;
        let custom_sha = self.cache.hash(custom).context("hashing custom video")?;
        let target_sha = self.cache.hash(target).context("hashing game video")?;
        let journal = self.journal();
        let prev_custom = journal.custom_sha.clone().filter(|_| journal.swapped);
        let is_ours =
            target_sha == custom_sha || prev_custom.as_deref() == Some(target_sha.as_str());

        let backup = if is_ours {
            self.backup_for(&name).ok_or_else(|| {
                anyhow!("the game file already holds a custom video but no backup exists")
            })?
        } else {
            // Genuine (possibly patched) original: refresh the backup if it differs.
            let mut forbidden = vec![custom_sha.as_str()];
            if let Some(p) = prev_custom.as_deref() {
                forbidden.push(p);
            }
            self.ensure_backup(target, &target_sha, &forbidden)?
        };

        let new_journal = Journal {
            swapped: true,
            target_path: Some(to_display(target)),
            original_sha: Some(backup.sha256.clone()),
            custom_sha: Some(custom_sha.clone()),
            at: Some(now_ms()),
        };
        self.write_journal(&new_journal)?;

        if target_sha == custom_sha {
            return Ok(SwapOutcome::AlreadyApplied);
        }

        if let Err(e) = writer.replace(custom, target) {
            self.cache.invalidate(target);
            // If the target is untouched, put the journal back the way it was.
            // (If it changed, keep "swapped" so restore puts the verified backup back.)
            if self.cache.hash(target).ok().as_deref() == Some(target_sha.as_str()) {
                let _ = if is_ours {
                    self.write_journal(&journal)
                } else {
                    self.clear_journal()
                };
            }
            return Err(e.into());
        }
        let after = self.cache.hash_fresh(target)?;
        if after != custom_sha {
            bail!("swap verification failed: game file hash does not match the custom video");
        }
        log::info!("swapped {} -> custom {}", name, &custom_sha[..12]);
        Ok(SwapOutcome::Swapped)
    }

    /// Restore the original. `target` defaults to the journal's target. `extra_custom` lists
    /// other hashes known to be custom videos (e.g. the selected video) for when the journal is gone.
    pub fn restore(
        &self,
        target: Option<&Path>,
        extra_custom: &[String],
        writer: &dyn FileWriter,
    ) -> Result<RestoreOutcome> {
        let journal = self.journal();
        let target: PathBuf = match (target, journal.target_path.as_deref()) {
            (Some(t), _) => t.to_path_buf(),
            (None, Some(t)) => PathBuf::from(t),
            (None, None) => bail!("nothing to restore"),
        };
        let name = file_name(&target)?;
        let entry = self
            .backup_for(&name)
            .ok_or_else(|| anyhow!("no backup of {name} exists"))?;
        let backup_file = self.backup_path(&entry.file_name);
        let backup_sha = self.cache.hash(&backup_file)?;
        if backup_sha != entry.sha256 {
            bail!("backup of {name} is corrupted (hash mismatch); not restoring");
        }

        let mut customs: Vec<&str> = extra_custom.iter().map(String::as_str).collect();
        if let Some(c) = journal.custom_sha.as_deref() {
            customs.push(c);
        }

        if target.is_file() {
            let cur = self.cache.hash(&target)?;
            if cur == entry.sha256 {
                self.clear_journal()?;
                return Ok(RestoreOutcome::AlreadyOriginal);
            }
            if !customs.contains(&cur.as_str()) && !journal.swapped {
                // Not ours, not the backup, and no swap recorded: genuine content
                // (e.g. a game patch). Leave it alone; the next swap re-backs it up.
                log::warn!("{name} holds content that is neither ours nor the backup; leaving it");
                return Ok(RestoreOutcome::KeptGenuine);
            }
            // Ours, or unknown content while a swap is recorded (e.g. an interrupted
            // in-place write): put the verified backup back. If Riot patched the file
            // meanwhile, its own file check simply repairs it again.
        } else if !target.parent().is_some_and(Path::is_dir) {
            bail!("game folder {} no longer exists", target.display());
        }

        writer.replace(&backup_file, &target)?;
        let after = self.cache.hash_fresh(&target)?;
        if after != entry.sha256 {
            bail!("restore verification failed for {name}");
        }
        self.clear_journal()?;
        log::info!("restored original {name}");
        Ok(RestoreOutcome::Restored)
    }

    /// Whether the journal says a swap is active for `target` (or any target if None).
    pub fn is_swapped_for(&self, target: Option<&Path>) -> bool {
        let j = self.journal();
        j.swapped
            && match (target, j.target_path.as_deref()) {
                (Some(t), Some(jt)) => path_key(&to_display(t)) == path_key(jt),
                _ => true,
            }
    }
}

fn file_name(p: &Path) -> Result<String> {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("invalid path {}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hash::sha256_file;

    struct Env {
        _tmp: tempfile::TempDir,
        swapper: Swapper,
        target: PathBuf,
        custom: PathBuf,
        dirs: DataDirs,
    }

    fn env() -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let menu = tmp.path().join("live/ShooterGame/Content/Movies/Menu");
        fs::create_dir_all(&menu).unwrap();
        let target = menu.join("13_06_Homescreen.mp4");
        fs::write(&target, b"ORIGINAL-VIDEO").unwrap();
        let custom = tmp.path().join("custom.mp4");
        fs::write(&custom, b"CUSTOM-VIDEO-CONTENT").unwrap();
        let dirs = DataDirs::new(tmp.path().join("data"));
        dirs.ensure().unwrap();
        let swapper = Swapper::new(dirs.clone(), Arc::new(HashCache::new()));
        Env {
            _tmp: tmp,
            swapper,
            target,
            custom,
            dirs,
        }
    }

    fn w() -> DirectWriter {
        DirectWriter {
            timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn swap_then_restore_roundtrip() {
        let e = env();
        assert_eq!(
            e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap(),
            SwapOutcome::Swapped
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"CUSTOM-VIDEO-CONTENT");
        let backup = e.swapper.backup_path("13_06_Homescreen.mp4");
        assert_eq!(fs::read(&backup).unwrap(), b"ORIGINAL-VIDEO");
        let j = e.swapper.journal();
        assert!(j.swapped);
        assert_eq!(
            j.original_sha.as_deref(),
            Some(sha256_file(&backup).unwrap().as_str())
        );
        assert!(!temp_path_for(&e.target).exists());

        assert_eq!(
            e.swapper.restore(None, &[], &w()).unwrap(),
            RestoreOutcome::Restored
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
        assert!(!e.swapper.journal().swapped);
        assert_eq!(
            e.swapper.restore(None, &[], &w()).unwrap(),
            RestoreOutcome::AlreadyOriginal
        );
    }

    #[test]
    fn backup_is_never_overwritten_by_custom() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        // Swapping again while the custom is in place must not touch the backup.
        assert_eq!(
            e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap(),
            SwapOutcome::AlreadyApplied
        );
        // Switching to a different custom video while swapped must not back up the first one.
        let other = e.custom.with_file_name("other.mp4");
        fs::write(&other, b"OTHER-CUSTOM").unwrap();
        assert_eq!(
            e.swapper.swap_in(&e.target, &other, &w()).unwrap(),
            SwapOutcome::Swapped
        );
        let backup = e.swapper.backup_path("13_06_Homescreen.mp4");
        assert_eq!(fs::read(&backup).unwrap(), b"ORIGINAL-VIDEO");
        assert!(!backup.with_file_name("13_06_Homescreen.mp4.prev").exists());
        // Direct attempt is refused too.
        let sha = sha256_file(&e.custom).unwrap();
        assert!(
            e.swapper
                .ensure_backup(&e.custom, &sha, &[sha.as_str()])
                .is_err()
        );

        e.swapper.restore(None, &[], &w()).unwrap();
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
    }

    #[test]
    fn patched_original_refreshes_backup() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        e.swapper.restore(None, &[], &w()).unwrap();
        // Game patch replaces the original.
        fs::write(&e.target, b"PATCHED-ORIGINAL").unwrap();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        let backup = e.swapper.backup_path("13_06_Homescreen.mp4");
        assert_eq!(fs::read(&backup).unwrap(), b"PATCHED-ORIGINAL");
        assert_eq!(
            fs::read(backup.with_file_name("13_06_Homescreen.mp4.prev")).unwrap(),
            b"ORIGINAL-VIDEO"
        );
        e.swapper.restore(None, &[], &w()).unwrap();
        assert_eq!(fs::read(&e.target).unwrap(), b"PATCHED-ORIGINAL");
    }

    #[test]
    fn unknown_content_while_swapped_gets_verified_backup() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        // e.g. a torn in-place write: unknown bytes while the journal says swapped.
        fs::write(&e.target, b"GARBAGE-PARTIAL-WRITE").unwrap();
        assert_eq!(
            e.swapper.restore(None, &[], &w()).unwrap(),
            RestoreOutcome::Restored
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
        // The garbage never became the backup.
        assert_eq!(
            fs::read(e.swapper.backup_path("13_06_Homescreen.mp4")).unwrap(),
            b"ORIGINAL-VIDEO"
        );
        assert!(!e.swapper.journal().swapped);
    }

    #[test]
    fn unknown_content_without_swap_is_left_alone() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        e.swapper.restore(None, &[], &w()).unwrap();
        fs::write(&e.target, b"NEW-PATCH-FROM-RIOT").unwrap();
        assert_eq!(
            e.swapper.restore(Some(&e.target), &[], &w()).unwrap(),
            RestoreOutcome::KeptGenuine
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"NEW-PATCH-FROM-RIOT");
    }

    #[test]
    fn failed_switch_keeps_previous_journal() {
        struct Failing;
        impl FileWriter for Failing {
            fn replace(&self, _: &Path, d: &Path) -> Result<(), ReplaceError> {
                Err(ReplaceError::InUse(
                    to_display(d),
                    io::Error::from_raw_os_error(32),
                ))
            }
        }
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        let before = e.swapper.journal();
        let other = e.custom.with_file_name("other.mp4");
        fs::write(&other, b"OTHER-CUSTOM").unwrap();
        assert!(e.swapper.swap_in(&e.target, &other, &Failing).is_err());
        assert_eq!(e.swapper.journal(), before);
        // Restore still recognises the first custom video and brings the original back.
        assert_eq!(
            e.swapper.restore(None, &[], &w()).unwrap(),
            RestoreOutcome::Restored
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
    }

    #[test]
    fn restore_verifies_backup_hash() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        fs::write(e.swapper.backup_path("13_06_Homescreen.mp4"), b"CORRUPTED").unwrap();
        let err = e.swapper.restore(None, &[], &w()).unwrap_err();
        assert!(err.to_string().contains("corrupted"), "{err}");
        // Game file untouched, journal still says swapped.
        assert_eq!(fs::read(&e.target).unwrap(), b"CUSTOM-VIDEO-CONTENT");
        assert!(e.swapper.journal().swapped);
    }

    #[test]
    fn crash_recovery_from_journal() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        // Simulate an app restart: brand new Swapper + cache, only the files on disk survive.
        let fresh = Swapper::new(e.dirs.clone(), Arc::new(HashCache::new()));
        let j = fresh.journal();
        assert!(j.swapped);
        assert!(fresh.is_swapped_for(Some(&e.target)));
        assert_eq!(
            fresh.restore(None, &[], &w()).unwrap(),
            RestoreOutcome::Restored
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
    }

    #[test]
    fn crash_before_write_is_harmless() {
        let e = env();
        // Journal written, but the process died before the file was replaced.
        e.swapper
            .ensure_backup(&e.target, &sha256_file(&e.target).unwrap(), &[])
            .unwrap();
        write_json_atomic(
            &e.dirs.state_file(),
            &Journal {
                swapped: true,
                target_path: Some(to_display(&e.target)),
                original_sha: None,
                custom_sha: Some(sha256_file(&e.custom).unwrap()),
                at: Some(1),
            },
        )
        .unwrap();
        assert_eq!(
            e.swapper.restore(None, &[], &w()).unwrap(),
            RestoreOutcome::AlreadyOriginal
        );
        assert!(!e.swapper.journal().swapped);
    }

    #[test]
    fn restore_without_journal_uses_known_custom_hash() {
        let e = env();
        e.swapper.swap_in(&e.target, &e.custom, &w()).unwrap();
        fs::remove_file(e.dirs.state_file()).unwrap();
        let sha = sha256_file(&e.custom).unwrap();
        assert_eq!(
            e.swapper.restore(Some(&e.target), &[sha], &w()).unwrap(),
            RestoreOutcome::Restored
        );
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
    }

    #[test]
    fn missing_target_is_an_error_and_writes_nothing() {
        let e = env();
        fs::remove_file(&e.target).unwrap();
        assert!(e.swapper.swap_in(&e.target, &e.custom, &w()).is_err());
        assert!(!e.swapper.journal().swapped);
        assert!(!e.target.exists());
    }

    #[test]
    fn replace_reports_access_denied_for_missing_dir_as_io() {
        let e = env();
        let bad = e.target.parent().unwrap().join("nope").join("x.mp4");
        assert!(matches!(
            replace_with_retry(&e.custom, &bad, Duration::from_millis(10)),
            Err(ReplaceError::Io(..))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn locked_file_retries_then_succeeds() {
        use std::os::windows::fs::OpenOptionsExt;
        let e = env();
        // Open the target with no sharing at all, release it after a moment.
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&e.target)
            .unwrap();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            drop(lock);
        });
        replace_with_retry(&e.custom, &e.target, Duration::from_secs(10)).unwrap();
        t.join().unwrap();
        assert_eq!(fs::read(&e.target).unwrap(), b"CUSTOM-VIDEO-CONTENT");
    }

    #[cfg(windows)]
    #[test]
    fn locked_file_times_out_as_in_use() {
        use std::os::windows::fs::OpenOptionsExt;
        let e = env();
        let _lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&e.target)
            .unwrap();
        let r = replace_with_retry(&e.custom, &e.target, Duration::from_millis(500));
        assert!(matches!(r, Err(ReplaceError::InUse(..))), "{r:?}");
        drop(_lock);
        assert_eq!(fs::read(&e.target).unwrap(), b"ORIGINAL-VIDEO");
    }
}
