//! Custom video library: `%APPDATA%\VeilLauncher\videos\` + `library.json`.

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::ffmpeg::ProbeInfo;
use super::paths::{path_key, to_display};
use crate::config::{now_ms, read_json, write_json_atomic};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub added_at: i64,
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub compatible: bool,
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

pub struct Library {
    dir: PathBuf,
    lock: Mutex<()>,
}

impl Library {
    pub fn new(videos_dir: PathBuf) -> Self {
        Self {
            dir: videos_dir,
            lock: Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn index(&self) -> PathBuf {
        self.dir.join("library.json")
    }

    fn load(&self) -> Vec<VideoItem> {
        match read_json::<Vec<VideoItem>>(&self.index()) {
            Ok(v) => v.unwrap_or_default(),
            Err(e) => {
                log::error!("library.json unreadable: {e:#}");
                Vec::new()
            }
        }
    }

    fn save(&self, items: &[VideoItem]) -> Result<()> {
        write_json_atomic(&self.index(), items)
    }

    /// All videos whose file still exists, newest first.
    pub fn list(&self) -> Vec<VideoItem> {
        let _g = self.lock.lock().unwrap();
        let mut v: Vec<VideoItem> = self
            .load()
            .into_iter()
            .filter(|i| Path::new(&i.path).is_file())
            .collect();
        v.sort_by_key(|i| std::cmp::Reverse(i.added_at));
        v
    }

    pub fn get(&self, id: &str) -> Option<VideoItem> {
        self.list().into_iter().find(|v| v.id == id)
    }

    /// Path a new library file with this id should be written to.
    pub fn file_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.mp4"))
    }

    /// Copy an `.mp4` into the library.
    pub fn import(&self, src: &Path, probe: Option<&ProbeInfo>) -> Result<VideoItem> {
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext != "mp4" {
            bail!(
                "Only .mp4 files can be imported directly. Use Convert to transcode other formats."
            );
        }
        if !src.is_file() {
            bail!("File not found: {}", src.display());
        }
        fs::create_dir_all(&self.dir)?;
        let id = new_id();
        let dest = self.file_for(&id);
        let tmp = dest.with_extension("mp4.part");
        fs::copy(src, &tmp).with_context(|| format!("copying {}", src.display()))?;
        fs::rename(&tmp, &dest)?;
        let name = src
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| id.clone());
        self.add_existing(&id, &name, &dest, probe)
    }

    /// Register a file that already sits in the library folder (e.g. a finished transcode).
    pub fn add_existing(
        &self,
        id: &str,
        name: &str,
        file: &Path,
        probe: Option<&ProbeInfo>,
    ) -> Result<VideoItem> {
        let size = fs::metadata(file)?.len();
        let item = VideoItem {
            id: id.to_string(),
            name: name.to_string(),
            path: to_display(file),
            size,
            added_at: now_ms(),
            duration_secs: probe.and_then(|p| p.duration_secs),
            width: probe.and_then(|p| p.width),
            height: probe.and_then(|p| p.height),
            codec: probe.and_then(|p| p.codec.clone()),
            compatible: probe.map(ProbeInfo::compatible).unwrap_or(true),
        };
        let _g = self.lock.lock().unwrap();
        let mut items = self.load();
        items.retain(|i| i.id != id);
        items.push(item.clone());
        self.save(&items)?;
        Ok(item)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut items = self.load();
        let pos = items
            .iter()
            .position(|i| i.id == id)
            .ok_or_else(|| anyhow!("video not found"))?;
        let item = items.remove(pos);
        let p = PathBuf::from(&item.path);
        // Only ever delete files inside the library folder.
        if path_key(&to_display(p.parent().unwrap_or(Path::new(""))))
            == path_key(&to_display(&self.dir))
            && let Err(e) = fs::remove_file(&p)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e).context("deleting video file");
        }
        self.save(&items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_list_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = Library::new(tmp.path().join("videos"));
        let src = tmp.path().join("My Clip.mp4");
        fs::write(&src, b"video").unwrap();
        let item = lib.import(&src, None).unwrap();
        assert_eq!(item.name, "My Clip");
        assert!(item.compatible);
        assert_eq!(item.size, 5);
        assert!(Path::new(&item.path).is_file());
        assert_eq!(lib.list(), vec![item.clone()]);
        assert_eq!(lib.get(&item.id), Some(item.clone()));

        let bad = tmp.path().join("clip.webm");
        fs::write(&bad, b"x").unwrap();
        assert!(lib.import(&bad, None).is_err());

        lib.remove(&item.id).unwrap();
        assert!(lib.list().is_empty());
        assert!(!Path::new(&item.path).exists());
        assert!(src.exists(), "source must never be deleted");
    }

    #[test]
    fn probe_marks_incompatible() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = Library::new(tmp.path().join("videos"));
        let src = tmp.path().join("a.mp4");
        fs::write(&src, b"v").unwrap();
        let probe = ProbeInfo {
            duration_secs: Some(12.5),
            width: Some(1920),
            height: Some(1080),
            codec: Some("hevc".into()),
            format_name: Some("mov,mp4,m4a,3gp,3g2,mj2".into()),
            fps: Some("30/1".into()),
            has_audio: false,
        };
        let item = lib.import(&src, Some(&probe)).unwrap();
        assert!(!item.compatible);
        assert_eq!(item.width, Some(1920));
    }
}
