//! ffmpeg discovery, download, probing and transcoding.

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use super::elevate::hidden_command;
use super::library::VideoItem;
use super::paths::to_display;

pub const FFMPEG_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
pub const FFMPEG_JOB_ID: &str = "ffmpeg";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub job_id: String,
    pub percent: f64,
    pub stage: String,
    pub done: bool,
    pub error: Option<String>,
    pub video: Option<VideoItem>,
}

impl ProgressEvent {
    pub fn step(job: &str, percent: f64, stage: impl Into<String>) -> Self {
        Self {
            job_id: job.into(),
            percent: percent.clamp(0.0, 100.0),
            stage: stage.into(),
            done: false,
            error: None,
            video: None,
        }
    }
    pub fn failed(job: &str, err: impl Into<String>) -> Self {
        Self {
            job_id: job.into(),
            percent: 0.0,
            stage: "Failed".into(),
            done: true,
            error: Some(err.into()),
            video: None,
        }
    }
    pub fn finished(job: &str, stage: impl Into<String>, video: Option<VideoItem>) -> Self {
        Self {
            job_id: job.into(),
            percent: 100.0,
            stage: stage.into(),
            done: true,
            error: None,
            video,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProbeInfo {
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub format_name: Option<String>,
    /// Frame rate as ffprobe reports it, e.g. "30000/1001".
    pub fps: Option<String>,
    pub has_audio: bool,
}

impl ProbeInfo {
    /// H.264 in an mp4/mov container. Unknown fields count as compatible.
    pub fn compatible(&self) -> bool {
        let codec_ok = self
            .codec
            .as_deref()
            .is_none_or(|c| c.eq_ignore_ascii_case("h264"));
        let fmt_ok = self.format_name.as_deref().is_none_or(|f| {
            let f = f.to_lowercase();
            f.contains("mp4") || f.contains("mov")
        });
        codec_ok && fmt_ok
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn find_on_path(file: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(file))
        .find(|p| p.is_file())
}

fn find_tool(ffmpeg_dir: &Path, name: &str) -> Option<PathBuf> {
    let local = ffmpeg_dir.join("bin").join(exe(name));
    if local.is_file() {
        return Some(local);
    }
    find_on_path(&exe(name))
}

pub fn find_ffmpeg(ffmpeg_dir: &Path) -> Option<PathBuf> {
    find_tool(ffmpeg_dir, "ffmpeg")
}

pub fn find_ffprobe(ffmpeg_dir: &Path) -> Option<PathBuf> {
    // Prefer the ffprobe next to the ffmpeg we would use.
    if let Some(ff) = find_ffmpeg(ffmpeg_dir) {
        let sib = ff.with_file_name(exe("ffprobe"));
        if sib.is_file() {
            return Some(sib);
        }
    }
    find_tool(ffmpeg_dir, "ffprobe")
}

pub fn parse_version_line(out: &str) -> Option<String> {
    let line = out.lines().next()?;
    let rest = line.strip_prefix("ffmpeg version ")?;
    Some(rest.split_whitespace().next()?.to_string())
}

pub fn status(ffmpeg_dir: &Path) -> FfmpegStatus {
    let Some(path) = find_ffmpeg(ffmpeg_dir) else {
        return FfmpegStatus {
            installed: false,
            path: None,
            version: None,
        };
    };
    let version = hidden_command(&path)
        .arg("-version")
        .stdin(Stdio::null())
        .output()
        .ok()
        .and_then(|o| parse_version_line(&String::from_utf8_lossy(&o.stdout)));
    FfmpegStatus {
        installed: true,
        path: Some(to_display(&path)),
        version,
    }
}

/// Download the BtbN win64 GPL build and extract ffmpeg.exe + ffprobe.exe into `<ffmpeg_dir>/bin`.
pub async fn install(
    ffmpeg_dir: &Path,
    on_progress: impl Fn(ProgressEvent),
) -> Result<FfmpegStatus> {
    fs::create_dir_all(ffmpeg_dir)?;
    let zip_path = ffmpeg_dir.join("download.zip.part");
    on_progress(ProgressEvent::step(FFMPEG_JOB_ID, 0.0, "Connecting"));
    let client = reqwest::Client::builder()
        .user_agent(concat!("VeilLauncher/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let mut resp = client
        .get(FFMPEG_URL)
        .send()
        .await?
        .error_for_status()
        .context("downloading ffmpeg")?;
    let total = resp.content_length();
    let mut file = fs::File::create(&zip_path)?;
    let mut done: u64 = 0;
    let mut last_pct = -1.0f64;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk)?;
        done += chunk.len() as u64;
        let mb = done as f64 / 1_048_576.0;
        let pct = total
            .map(|t| done as f64 / t.max(1) as f64 * 90.0)
            .unwrap_or(0.0);
        if (pct - last_pct).abs() >= 0.5 || total.is_none() && done % (4 << 20) < chunk.len() as u64
        {
            last_pct = pct;
            let stage = match total {
                Some(t) => format!("Downloading {mb:.1} / {:.1} MB", t as f64 / 1_048_576.0),
                None => format!("Downloading {mb:.1} MB"),
            };
            on_progress(ProgressEvent::step(FFMPEG_JOB_ID, pct, stage));
        }
    }
    file.flush()?;
    drop(file);

    on_progress(ProgressEvent::step(FFMPEG_JOB_ID, 92.0, "Extracting"));
    let bin = ffmpeg_dir.join("bin");
    let zp = zip_path.clone();
    let bin2 = bin.clone();
    tauri::async_runtime::spawn_blocking(move || extract_tools(&zp, &bin2))
        .await
        .map_err(|e| anyhow!("extract task failed: {e}"))??;
    let _ = fs::remove_file(&zip_path);
    let st = status(ffmpeg_dir);
    if !st.installed {
        bail!("ffmpeg was extracted but cannot be found");
    }
    on_progress(ProgressEvent::finished(FFMPEG_JOB_ID, "ffmpeg ready", None));
    Ok(st)
}

pub fn extract_tools(zip_path: &Path, bin_dir: &Path) -> Result<()> {
    let f = fs::File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(f).context("opening ffmpeg zip")?;
    fs::create_dir_all(bin_dir)?;
    let mut found = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        let Some(base) = name.rsplit('/').next() else {
            continue;
        };
        let wanted = ["ffmpeg.exe", "ffprobe.exe"]
            .iter()
            .any(|w| base.eq_ignore_ascii_case(w));
        if !wanted || !name.to_lowercase().contains("/bin/") || entry.is_dir() {
            continue;
        }
        let dest = bin_dir.join(base.to_lowercase());
        let tmp = dest.with_extension("exe.part");
        let mut out = fs::File::create(&tmp)?;
        std::io::copy(&mut entry, &mut out)?;
        drop(out);
        fs::rename(&tmp, &dest)?;
        found += 1;
    }
    if found < 2 {
        bail!("ffmpeg.exe / ffprobe.exe not found in the downloaded archive");
    }
    Ok(())
}

pub fn parse_probe_json(text: &str) -> Result<ProbeInfo> {
    let v: serde_json::Value = serde_json::from_str(text).context("parsing ffprobe output")?;
    let streams = v
        .get("streams")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let video = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("video"));
    let has_audio = streams
        .iter()
        .any(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("audio"));
    let fmt = v.get("format");
    let num = |x: Option<&serde_json::Value>| -> Option<f64> {
        x.and_then(|x| {
            x.as_f64()
                .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
        })
    };
    let duration = num(fmt.and_then(|f| f.get("duration")))
        .or_else(|| num(video.and_then(|s| s.get("duration"))));
    let fps = video
        .and_then(|s| {
            [s.get("avg_frame_rate"), s.get("r_frame_rate")]
                .into_iter()
                .flatten()
                .filter_map(|x| x.as_str())
                .find(|f| fps_value(f).is_some_and(|v| v > 0.0))
        })
        .map(str::to_string);
    Ok(ProbeInfo {
        duration_secs: duration.filter(|d| *d > 0.0),
        width: video
            .and_then(|s| s.get("width"))
            .and_then(|w| w.as_u64())
            .map(|w| w as u32),
        height: video
            .and_then(|s| s.get("height"))
            .and_then(|w| w.as_u64())
            .map(|w| w as u32),
        codec: video
            .and_then(|s| s.get("codec_name"))
            .and_then(|c| c.as_str())
            .map(str::to_string),
        format_name: fmt
            .and_then(|f| f.get("format_name"))
            .and_then(|c| c.as_str())
            .map(str::to_string),
        fps,
        has_audio,
    })
}

/// "30000/1001" → 29.97
pub fn fps_value(s: &str) -> Option<f64> {
    match s.split_once('/') {
        Some((a, b)) => {
            let (a, b): (f64, f64) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
            (b != 0.0).then_some(a / b)
        }
        None => s.trim().parse().ok(),
    }
}

pub fn probe(ffprobe: &Path, file: &Path) -> Result<ProbeInfo> {
    let out = hidden_command(ffprobe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file)
        .stdin(Stdio::null())
        .output()
        .context("running ffprobe")?;
    if !out.status.success() {
        bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    parse_probe_json(&String::from_utf8_lossy(&out.stdout))
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscodeTarget {
    pub width: u32,
    pub height: u32,
    pub fps: String,
}

impl Default for TranscodeTarget {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: "30".into(),
        }
    }
}

impl TranscodeTarget {
    /// Match the original game video, falling back to 1920x1080@30.
    pub fn from_probe(p: Option<&ProbeInfo>) -> Self {
        let d = Self::default();
        let Some(p) = p else { return d };
        let even = |v: u32| v.max(2) & !1;
        match (p.width, p.height) {
            (Some(w), Some(h)) if w >= 16 && h >= 16 => Self {
                width: even(w),
                height: even(h),
                fps: p
                    .fps
                    .clone()
                    .filter(|f| fps_value(f).is_some_and(|v| v > 0.0 && v <= 240.0))
                    .unwrap_or(d.fps),
            },
            _ => d,
        }
    }
}

pub fn transcode_args(
    input: &Path,
    output: &Path,
    t: &TranscodeTarget,
    keep_audio: bool,
) -> Vec<String> {
    let (w, h) = (t.width, t.height);
    let mut a: Vec<String> = vec![
        "-hide_banner".into(),
        "-nostats".into(),
        "-y".into(),
        "-i".into(),
        input.to_string_lossy().into(),
        "-vf".into(),
        format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"),
        "-r".into(),
        t.fps.clone(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-crf".into(),
        "20".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-profile:v".into(),
        "high".into(),
        "-movflags".into(),
        "+faststart".into(),
    ];
    if keep_audio {
        a.extend(["-c:a", "aac", "-b:a", "160k"].map(String::from));
    } else {
        a.push("-an".into());
    }
    a.extend(["-progress", "pipe:1"].map(String::from));
    a.push(output.to_string_lossy().into());
    a
}

/// Parse a `-progress` line; returns processed seconds for `out_time_us` / `out_time_ms`.
/// (ffmpeg's `out_time_ms` is actually in microseconds.)
pub fn parse_progress_secs(line: &str) -> Option<f64> {
    let (k, v) = line.trim().split_once('=')?;
    match k {
        "out_time_us" | "out_time_ms" => v
            .trim()
            .parse::<f64>()
            .ok()
            .map(|us| us / 1_000_000.0)
            .filter(|s| *s >= 0.0),
        _ => None,
    }
}

/// Run ffmpeg, reporting percent (0–100) via `on_percent`. Blocking.
pub fn transcode(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
    target: &TranscodeTarget,
    keep_audio: bool,
    duration_secs: Option<f64>,
    on_percent: impl Fn(f64),
) -> Result<()> {
    let mut child = hidden_command(ffmpeg)
        .args(transcode_args(input, output, target, keep_audio))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("starting ffmpeg")?;

    let tail: Arc<Mutex<VecDeque<String>>> = Arc::default();
    let stderr = child.stderr.take();
    let tail2 = tail.clone();
    let err_thread = std::thread::spawn(move || {
        if let Some(mut s) = stderr {
            let mut buf = String::new();
            let _ = s.read_to_string(&mut buf);
            let mut t = tail2.lock().unwrap();
            for l in buf.lines().filter(|l| !l.trim().is_empty()) {
                t.push_back(l.to_string());
                if t.len() > 8 {
                    t.pop_front();
                }
            }
        }
    });

    if let Some(out) = child.stdout.take() {
        let mut last = -1.0;
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let (Some(secs), Some(d)) = (parse_progress_secs(&line), duration_secs) {
                let pct = (secs / d * 100.0).clamp(0.0, 99.0);
                if pct - last >= 0.5 {
                    last = pct;
                    on_percent(pct);
                }
            }
        }
    }
    let status = child.wait()?;
    let _ = err_thread.join();
    if !status.success() {
        let _ = fs::remove_file(output);
        let msg = tail
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        bail!("ffmpeg failed ({status}): {}", msg.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_probe_output() {
        let json = r#"{"streams":[
            {"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"r_frame_rate":"30/1","avg_frame_rate":"30000/1001"},
            {"codec_type":"audio","codec_name":"aac"}],
            "format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"61.500000"}}"#;
        let p = parse_probe_json(json).unwrap();
        assert_eq!(p.width, Some(1920));
        assert_eq!(p.height, Some(1080));
        assert_eq!(p.codec.as_deref(), Some("h264"));
        assert_eq!(p.duration_secs, Some(61.5));
        assert_eq!(p.fps.as_deref(), Some("30000/1001"));
        assert!(p.has_audio);
        assert!(p.compatible());

        let webm = r#"{"streams":[{"codec_type":"video","codec_name":"vp9","width":1280,"height":720}],"format":{"format_name":"matroska,webm"}}"#;
        let p = parse_probe_json(webm).unwrap();
        assert!(!p.compatible());
        assert!(ProbeInfo::default().compatible());
    }

    #[test]
    fn transcode_target_and_args() {
        let p = ProbeInfo {
            width: Some(2561),
            height: Some(1440),
            fps: Some("60/1".into()),
            ..Default::default()
        };
        let t = TranscodeTarget::from_probe(Some(&p));
        assert_eq!(
            t,
            TranscodeTarget {
                width: 2560,
                height: 1440,
                fps: "60/1".into()
            }
        );
        assert_eq!(
            TranscodeTarget::from_probe(None),
            TranscodeTarget::default()
        );

        let a = transcode_args(
            Path::new("in.webm"),
            Path::new("out.mp4"),
            &TranscodeTarget::default(),
            false,
        );
        let s = a.join(" ");
        assert!(s.contains("scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080"));
        assert!(s.contains("-c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p -profile:v high -movflags +faststart"));
        assert!(s.contains("-an") && !s.contains("aac"));
        assert!(s.ends_with("-progress pipe:1 out.mp4"));
        let s = transcode_args(
            Path::new("i"),
            Path::new("o"),
            &TranscodeTarget::default(),
            true,
        )
        .join(" ");
        assert!(s.contains("-c:a aac -b:a 160k") && !s.contains("-an"));
    }

    #[test]
    fn progress_and_version_parsing() {
        assert_eq!(parse_progress_secs("out_time_ms=1500000"), Some(1.5));
        assert_eq!(parse_progress_secs("out_time_us=3000000"), Some(3.0));
        assert_eq!(parse_progress_secs("out_time=00:00:01.5"), None);
        assert_eq!(parse_progress_secs("out_time_ms=N/A"), None);
        assert_eq!(
            fps_value("30000/1001").map(|v| (v * 100.0).round()),
            Some(2997.0)
        );
        assert_eq!(fps_value("0/0"), None);
        assert_eq!(
            parse_version_line("ffmpeg version N-117000-gabc Copyright (c) 2000-2026\nbuilt with"),
            Some("N-117000-gabc".into())
        );
    }
}
