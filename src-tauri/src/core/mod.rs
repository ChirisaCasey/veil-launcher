//! Tauri-independent core: discovery, detection, swapping, watching, elevation, ffmpeg.

pub mod detect;
pub mod elevate;
pub mod ffmpeg;
pub mod hash;
pub mod launch;
pub mod library;
pub mod paths;
pub mod swap;
pub mod watcher;
