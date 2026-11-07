//! Smart detection of the main-menu Homescreen video.
//!
//! Rules (conservative: selecting nothing beats selecting the wrong file):
//! * Only `*.mp4` files are candidates.
//! * A name containing "homescreen" dominates everything else.
//! * Among homescreens, a higher leading version prefix (`13_06_`, `13.06-`, `v13_06`) wins, then mtime.
//! * "cinematic" / "seasonal" / "prelude" / "trailer" / "teaser" are penalised.
//! * A non-homescreen file is only selected if no homescreen exists, it looks like a menu loop
//!   and it carries no penalty word.
//! * A user pin (exact file name) overrides the ranking if the file still exists.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::paths::to_display;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub score: i64,
    pub is_homescreen: bool,
    pub version: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub menu_dir: Option<String>,
    pub candidates: Vec<Candidate>,
    pub selected: Option<Candidate>,
    pub pinned: bool,
    pub pin_missing: bool,
}

impl DetectionResult {
    pub fn empty(menu_dir: Option<String>, pin_missing: bool) -> Self {
        Self {
            menu_dir,
            candidates: vec![],
            selected: None,
            pinned: false,
            pin_missing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub size: u64,
    /// Unix ms
    pub modified: i64,
}

const HOMESCREEN_SCORE: i64 = 10_000_000;
const PENALTY_HOMESCREEN: i64 = 4_000_000;
const MENU_LOOP_SCORE: i64 = 100_000;
const PENALTY_OTHER: i64 = 1_000_000;
const PENALTY_WORDS: [&str; 5] = ["cinematic", "seasonal", "prelude", "trailer", "teaser"];
const LOOP_WORDS: [&str; 6] = ["mainmenu", "menu", "loop", "background", "lobby", "home"];

/// Parsed leading version prefix: numeric groups plus display string ("13.06").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub parts: Vec<u32>,
    pub display: String,
}

/// Parse a leading version prefix such as `13_06_`, `13.06-`, `v13_06_`, `13-06 `.
/// Each numeric group must be followed by a separator (`_`, `.`, `-`, space), so names like
/// `1080p_Homescreen` do not produce a version. At most 3 groups are read.
pub fn parse_version(stem: &str) -> Option<Version> {
    let s = stem.strip_prefix(['v', 'V']).unwrap_or(stem);
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut groups: Vec<&str> = Vec::new();
    while groups.len() < 3 {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start || i >= bytes.len() || !matches!(bytes[i], b'_' | b'.' | b'-' | b' ') {
            break;
        }
        groups.push(&s[start..i]);
        i += 1; // separator
    }
    if groups.is_empty() {
        return None;
    }
    let parts = groups
        .iter()
        .map(|g| g.parse::<u32>().unwrap_or(u32::MAX))
        .collect();
    Some(Version {
        parts,
        display: groups.join("."),
    })
}

fn version_bonus(v: &Version) -> i64 {
    let major = v.parts.first().copied().unwrap_or(0).min(999) as i64;
    let minor = v.parts.get(1).copied().unwrap_or(0).min(999) as i64;
    major * 1000 + minor
}

pub fn is_mp4(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".mp4")
}

fn is_our_temp(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.starts_with(".veil") || l.contains(".veil-tmp")
}

/// Score one file. Returns (candidate, selectable_if_no_homescreen).
fn score(entry: &FileEntry) -> (Candidate, bool) {
    let lower = entry.file_name.to_lowercase();
    let stem = Path::new(&entry.file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_homescreen = lower.contains("homescreen");
    let penalties: Vec<&str> = PENALTY_WORDS
        .iter()
        .copied()
        .filter(|w| lower.contains(w))
        .collect();
    let version = parse_version(&stem);
    let mut reasons: Vec<String> = Vec::new();
    let mut score: i64;
    let mut loop_like = false;

    if is_homescreen {
        score = HOMESCREEN_SCORE;
        reasons.push("Name contains \"Homescreen\"".into());
        if let Some(v) = &version {
            score += version_bonus(v);
            reasons.push(format!("version {}", v.display));
        } else {
            reasons.push("no version prefix".into());
        }
        if !penalties.is_empty() {
            score -= PENALTY_HOMESCREEN;
            reasons.push(format!("penalised: looks like a {}", penalties.join("/")));
        }
    } else {
        loop_like = LOOP_WORDS.iter().any(|w| lower.contains(w));
        if !penalties.is_empty() {
            score = -PENALTY_OTHER;
            reasons.push(format!(
                "Not a Homescreen; looks like a {} (never auto-selected)",
                penalties.join("/")
            ));
        } else if loop_like {
            score = MENU_LOOP_SCORE;
            reasons.push(
                "Not a Homescreen, but the name looks like a menu loop (fallback only)".into(),
            );
        } else {
            score = 0;
            reasons.push("Not a Homescreen video (never auto-selected)".into());
        }
    }

    let cand = Candidate {
        file_name: entry.file_name.clone(),
        path: to_display(&entry.path),
        size: entry.size,
        modified: entry.modified,
        score,
        is_homescreen,
        version: version.map(|v| v.display),
        reason: reasons.join(" · "),
    };
    (cand, loop_like && penalties.is_empty() && !is_homescreen)
}

/// Raw score → 0..=100. Input must be visited in ranked order.
/// Clean homescreens: 100, 92, 84 … (min 60); penalised homescreens: 45, 43 … (min 35);
/// menu-loop fallback: 30; other mp4: 10; penalised non-homescreen: 0.
fn display_score(raw: i64, clean_rank: &mut i64, penalised_rank: &mut i64) -> i64 {
    if raw >= HOMESCREEN_SCORE {
        let s = (100 - 8 * *clean_rank).max(60);
        *clean_rank += 1;
        s
    } else if raw >= HOMESCREEN_SCORE - PENALTY_HOMESCREEN {
        let s = (45 - 2 * *penalised_rank).max(35);
        *penalised_rank += 1;
        s
    } else if raw >= MENU_LOOP_SCORE {
        30
    } else if raw >= 0 {
        10
    } else {
        0
    }
}

/// Pure ranking over a directory listing.
pub fn rank(
    entries: &[FileEntry],
    pinned: Option<&str>,
    menu_dir: Option<String>,
) -> DetectionResult {
    let mut scored: Vec<(Candidate, bool)> = entries
        .iter()
        .filter(|e| is_mp4(&e.file_name) && !is_our_temp(&e.file_name))
        .map(score)
        .collect();
    scored.sort_by(|(a, _), (b, _)| {
        b.score
            .cmp(&a.score)
            .then(b.modified.cmp(&a.modified))
            .then(a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()))
    });

    // Explain tie-breaks among homescreens.
    let homescreens: Vec<usize> = scored
        .iter()
        .enumerate()
        .filter(|(_, (c, _))| c.is_homescreen)
        .map(|(i, _)| i)
        .collect();
    if homescreens.len() > 1 {
        let top = homescreens[0];
        let tied = scored[homescreens[1]].0.score == scored[top].0.score;
        let note = if tied {
            "best match (newest file)"
        } else {
            "best match (highest version)"
        };
        scored[top].0.reason.push_str(&format!(" · {note}"));
    }

    // Map raw ranking scores to the 0-100 scale the UI draws (order is preserved).
    let (mut clean_rank, mut penalised_rank) = (0i64, 0i64);
    for (c, _) in scored.iter_mut() {
        c.score = display_score(c.score, &mut clean_rank, &mut penalised_rank);
    }

    let any_homescreen = !homescreens.is_empty();
    let auto: Option<Candidate> = scored.first().and_then(|(c, loop_ok)| {
        if c.is_homescreen || (!any_homescreen && *loop_ok) {
            Some(c.clone())
        } else {
            None
        }
    });

    let mut pinned_flag = false;
    let mut pin_missing = false;
    let mut selected = auto;
    if let Some(pin) = pinned.map(str::trim).filter(|p| !p.is_empty()) {
        match scored
            .iter_mut()
            .find(|(c, _)| c.file_name.eq_ignore_ascii_case(pin))
        {
            Some((c, _)) => {
                c.reason = format!("Pinned by you · {}", c.reason);
                selected = Some(c.clone());
                pinned_flag = true;
            }
            None => pin_missing = true,
        }
    }

    DetectionResult {
        menu_dir,
        candidates: scored.into_iter().map(|(c, _)| c).collect(),
        selected,
        pinned: pinned_flag,
        pin_missing,
    }
}

pub fn scan_dir(dir: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir)? {
        let Ok(e) = e else { continue };
        let Ok(md) = e.metadata() else { continue };
        if !md.is_file() {
            continue;
        }
        let modified = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        out.push(FileEntry {
            file_name: e.file_name().to_string_lossy().to_string(),
            path: e.path(),
            size: md.len(),
            modified,
        });
    }
    Ok(out)
}

/// Read-only: list the Menu dir and rank it.
pub fn detect(menu_dir: Option<&Path>, pinned: Option<&str>) -> DetectionResult {
    let Some(dir) = menu_dir else {
        return DetectionResult::empty(None, pinned.is_some_and(|p| !p.trim().is_empty()));
    };
    match scan_dir(dir) {
        Ok(entries) => rank(&entries, pinned, Some(to_display(dir))),
        Err(e) => {
            log::warn!("cannot read {}: {e}", dir.display());
            DetectionResult::empty(
                Some(to_display(dir)),
                pinned.is_some_and(|p| !p.trim().is_empty()),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, size: u64, modified: i64) -> FileEntry {
        FileEntry {
            file_name: name.into(),
            path: PathBuf::from("M").join(name),
            size,
            modified,
        }
    }

    /// The exact listing verified on the dev machine.
    fn real_listing() -> Vec<FileEntry> {
        vec![
            e("13_06_Homescreen.mp4", 42_430_703, 1_790_000_000_000),
            e(
                "13_09_Seasonal_Prelude_Cinematic.mp4",
                31_500_161,
                1_793_000_000_000,
            ),
            e("Battle Pass Glitches.webm", 588_065, 1_741_000_000_000),
            e("Contract Glitches.webm", 2_729_576, 1_741_000_000_000),
        ]
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("13_06_Homescreen").unwrap().display, "13.06");
        assert_eq!(
            parse_version("13.06-Homescreen").unwrap().parts,
            vec![13, 6]
        );
        assert_eq!(parse_version("v13_06_Homescreen").unwrap().display, "13.06");
        assert_eq!(parse_version("9_Homescreen").unwrap().display, "9");
        assert_eq!(parse_version("Homescreen"), None);
        assert_eq!(parse_version("1080p_Homescreen"), None);
        assert_eq!(
            parse_version("13_06"),
            Some(Version {
                parts: vec![13],
                display: "13".into()
            })
        );
    }

    #[test]
    fn real_listing_picks_13_06_homescreen() {
        let r = rank(&real_listing(), None, None);
        let sel = r.selected.expect("a target");
        assert_eq!(sel.file_name, "13_06_Homescreen.mp4");
        assert!(sel.is_homescreen);
        assert_eq!(sel.version.as_deref(), Some("13.06"));
        // webm files are not candidates; the cinematic is ranked below.
        assert_eq!(r.candidates.len(), 2);
        assert_eq!(
            r.candidates[1].file_name,
            "13_09_Seasonal_Prelude_Cinematic.mp4"
        );
        assert_eq!(r.candidates[0].score, 100);
        assert_eq!(r.candidates[1].score, 0);
        assert!(!r.pinned && !r.pin_missing);
        assert!(!sel.reason.is_empty());
    }

    #[test]
    fn highest_version_homescreen_wins_over_newer_mtime() {
        let files = vec![
            e("12_10_Homescreen.mp4", 1, 3_000),
            e("13_06_Homescreen.mp4", 1, 1_000),
            e("9_99_Homescreen.mp4", 1, 5_000),
            e("Homescreen.mp4", 1, 9_000),
            e("14_01_Seasonal_Prelude_Cinematic.mp4", 1, 9_999),
        ];
        let r = rank(&files, None, None);
        assert_eq!(r.selected.unwrap().file_name, "13_06_Homescreen.mp4");
        assert_eq!(r.candidates[1].file_name, "12_10_Homescreen.mp4");
        assert!(r.candidates[0].reason.contains("highest version"));
        let scores: Vec<i64> = r.candidates.iter().map(|c| c.score).collect();
        assert_eq!(scores, vec![100, 92, 84, 76, 0]);
    }

    #[test]
    fn mtime_breaks_version_ties_and_handles_postfix() {
        let files = vec![
            e("Homescreen_old.mp4", 1, 1_000),
            e("Homescreen_v2.MP4", 1, 2_000),
        ];
        let r = rank(&files, None, None);
        assert_eq!(r.selected.unwrap().file_name, "Homescreen_v2.MP4");
    }

    #[test]
    fn penalised_homescreen_loses_to_plain() {
        let files = vec![
            e("14_00_Homescreen_Cinematic.mp4", 1, 2),
            e("13_00_Homescreen.mp4", 1, 1),
        ];
        assert_eq!(
            rank(&files, None, None).selected.unwrap().file_name,
            "13_00_Homescreen.mp4"
        );
    }

    #[test]
    fn pin_overrides_and_reports_missing() {
        let r = rank(
            &real_listing(),
            Some("13_09_seasonal_prelude_cinematic.mp4"),
            None,
        );
        assert!(r.pinned && !r.pin_missing);
        assert_eq!(
            r.selected.unwrap().file_name,
            "13_09_Seasonal_Prelude_Cinematic.mp4"
        );

        let r = rank(&real_listing(), Some("12_00_Homescreen.mp4"), None);
        assert!(!r.pinned && r.pin_missing);
        assert_eq!(r.selected.unwrap().file_name, "13_06_Homescreen.mp4");
    }

    #[test]
    fn no_homescreen_is_conservative() {
        // Only cinematics / unrelated videos: select nothing.
        let files = vec![
            e("13_09_Seasonal_Prelude_Cinematic.mp4", 1, 1),
            e("Agent_Trailer.mp4", 1, 2),
            e("Random.mp4", 1, 3),
        ];
        let r = rank(&files, None, None);
        assert!(r.selected.is_none());
        assert_eq!(r.candidates.len(), 3);

        // A menu-loop-looking file is accepted only when no homescreen exists.
        let files = vec![e("MainMenu_Loop.mp4", 1, 1), e("Random.mp4", 1, 3)];
        assert_eq!(
            rank(&files, None, None).selected.unwrap().file_name,
            "MainMenu_Loop.mp4"
        );
        let files = vec![e("MainMenu_Loop.mp4", 1, 9), e("Homescreen.mp4", 1, 1)];
        assert_eq!(
            rank(&files, None, None).selected.unwrap().file_name,
            "Homescreen.mp4"
        );
        // ...and never if it is penalised.
        let files = vec![e("Menu_Cinematic.mp4", 1, 1)];
        assert!(rank(&files, None, None).selected.is_none());
    }

    #[test]
    fn ignores_non_mp4_and_temp_files() {
        let files = vec![
            e("Homescreen.webm", 1, 1),
            e(".veil-write-test-1.tmp", 1, 1),
            e(".13_06_Homescreen.mp4.veil-tmp", 1, 1),
        ];
        let r = rank(&files, None, None);
        assert!(r.candidates.is_empty() && r.selected.is_none());
    }

    #[test]
    fn detect_on_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "13_06_Homescreen.mp4",
            "13_09_Seasonal_Prelude_Cinematic.mp4",
            "Contract Glitches.webm",
        ] {
            fs::write(dir.path().join(f), b"x").unwrap();
        }
        let r = detect(Some(dir.path()), None);
        assert_eq!(r.selected.unwrap().file_name, "13_06_Homescreen.mp4");
        assert!(r.menu_dir.is_some());
        let r = detect(None, Some("x.mp4"));
        assert!(r.pin_missing && r.selected.is_none());
    }
}
