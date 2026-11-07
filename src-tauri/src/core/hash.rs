//! SHA-256 hashing with a (path, size, mtime) cache so 40 MB videos are not re-hashed constantly.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    size: u64,
    mtime_ns: u128,
}

fn stamp(path: &Path) -> io::Result<Stamp> {
    let md = std::fs::metadata(path)?;
    let mtime_ns = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(Stamp {
        size: md.len(),
        mtime_ns,
    })
}

fn key(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('/', "\\").to_lowercase())
}

#[derive(Default)]
pub struct HashCache {
    map: Mutex<HashMap<PathBuf, (Stamp, String)>>,
}

impl HashCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hash, reusing a cached value when size and mtime are unchanged.
    pub fn hash(&self, path: &Path) -> io::Result<String> {
        let st = stamp(path)?;
        let k = key(path);
        if let Some((s, h)) = self.map.lock().unwrap().get(&k)
            && *s == st
        {
            return Ok(h.clone());
        }
        let h = sha256_file(path)?;
        self.map.lock().unwrap().insert(k, (st, h.clone()));
        Ok(h)
    }

    /// Always re-hash (used to verify a write) and refresh the cache entry.
    pub fn hash_fresh(&self, path: &Path) -> io::Result<String> {
        self.invalidate(path);
        self.hash(path)
    }

    pub fn invalidate(&self, path: &Path) {
        self.map.lock().unwrap().remove(&key(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_caches() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.bin");
        std::fs::write(&p, b"abc").unwrap();
        let c = HashCache::new();
        let h = c.hash(&p).unwrap();
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(c.hash(&p).unwrap(), h);
        std::fs::write(&p, b"abcd").unwrap();
        assert_ne!(c.hash(&p).unwrap(), h);
    }
}
