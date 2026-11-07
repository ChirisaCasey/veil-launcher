//! VALORANT install discovery.
//!
//! Order: user override (if valid) → Riot metadata yaml → Uninstall registry key → drive scan.
//! Everything here is read-only except the optional `writable` probe.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallSource {
    Metadata,
    Registry,
    Scan,
    Override,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallInfo {
    pub install_dir: Option<String>,
    pub menu_dir: Option<String>,
    pub riot_client: Option<String>,
    pub source: InstallSource,
    pub writable: bool,
}

impl InstallInfo {
    pub fn none() -> Self {
        Self {
            install_dir: None,
            menu_dir: None,
            riot_client: None,
            source: InstallSource::None,
            writable: false,
        }
    }
}

pub const MENU_SUBPATH: [&str; 4] = ["ShooterGame", "Content", "Movies", "Menu"];
const UNINSTALL_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Riot Game valorant.live";

/// Forward-slash display form used in the UI contract (`D:/Riot Games/VALORANT/live`).
pub fn to_display(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Normalised comparison key for paths (case-insensitive, slash-agnostic, no trailing slash).
pub fn path_key(s: &str) -> String {
    s.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

pub fn menu_dir_of(live: &Path) -> PathBuf {
    MENU_SUBPATH
        .iter()
        .fold(live.to_path_buf(), |p, s| p.join(s))
}

pub fn is_valid_live_dir(live: &Path) -> bool {
    menu_dir_of(live).is_dir()
}

/// Accepts the `live` folder, its parent `VALORANT` folder, or the Menu folder itself.
pub fn normalize_live_dir(p: &Path) -> Option<PathBuf> {
    if is_valid_live_dir(p) {
        return Some(p.to_path_buf());
    }
    let live = p.join("live");
    if is_valid_live_dir(&live) {
        return Some(live);
    }
    // .../live/ShooterGame/Content/Movies/Menu → .../live
    let mut cur = p;
    for _ in 0..MENU_SUBPATH.len() {
        cur = cur.parent()?;
    }
    if is_valid_live_dir(cur) && menu_dir_of(cur) == p {
        Some(cur.to_path_buf())
    } else {
        None
    }
}

/// Parse `product_install_full_path: "D:/Riot Games/VALORANT/live"` from the product settings yaml.
pub fn parse_product_settings(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("product_install_full_path:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Candidate Riot Client paths from `RiotClientInstalls.json`, best first.
pub fn parse_riot_client_installs(json: &str, install_dir: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return out;
    };
    if let (Some(dir), Some(map)) = (
        install_dir,
        v.get("associated_client").and_then(|m| m.as_object()),
    ) {
        let want = path_key(dir);
        for (k, val) in map {
            if path_key(k) == want
                && let Some(s) = val.as_str()
            {
                out.push(s.to_string());
            }
        }
    }
    for key in ["rc_live", "rc_default"] {
        if let Some(s) = v.get(key).and_then(|x| x.as_str())
            && !out.iter().any(|o| path_key(o) == path_key(s))
        {
            out.push(s.to_string());
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    pub install_override: Option<String>,
    /// Create+delete a temp file in the Menu dir to test write access.
    pub probe_writable: bool,
    /// Usually `C:\ProgramData`.
    pub program_data: PathBuf,
    pub use_registry: bool,
    pub scan_drives: bool,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            install_override: None,
            probe_writable: true,
            program_data: std::env::var_os("ProgramData")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData")),
            use_registry: true,
            scan_drives: true,
        }
    }
}

fn from_metadata(program_data: &Path) -> Option<PathBuf> {
    let yaml = program_data
        .join("Riot Games")
        .join("Metadata")
        .join("valorant.live")
        .join("valorant.live.product_settings.yaml");
    let text = fs::read_to_string(yaml).ok()?;
    let dir = PathBuf::from(parse_product_settings(&text)?);
    normalize_live_dir(&dir)
}

#[cfg(windows)]
fn from_registry() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let keys = [
        (HKEY_CURRENT_USER, UNINSTALL_KEY.to_string()),
        (HKEY_LOCAL_MACHINE, UNINSTALL_KEY.to_string()),
        (
            HKEY_LOCAL_MACHINE,
            UNINSTALL_KEY.replacen("Software\\", "Software\\WOW6432Node\\", 1),
        ),
    ];
    for (hive, sub) in keys {
        let Ok(k) = RegKey::predef(hive).open_subkey(&sub) else {
            continue;
        };
        let Ok(loc) = k.get_value::<String, _>("InstallLocation") else {
            continue;
        };
        if let Some(p) = normalize_live_dir(Path::new(loc.trim().trim_matches('"'))) {
            return Some(p);
        }
    }
    None
}

#[cfg(not(windows))]
fn from_registry() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
pub fn drive_roots() -> Vec<PathBuf> {
    let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| PathBuf::from(format!("{}:\\", (b'A' + i) as char)))
        .collect()
}

#[cfg(not(windows))]
pub fn drive_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn from_scan() -> Option<PathBuf> {
    for root in drive_roots() {
        for rel in [
            r"Riot Games\VALORANT\live",
            r"Program Files\Riot Games\VALORANT\live",
            r"Games\Riot Games\VALORANT\live",
        ] {
            let p = root.join(rel);
            if is_valid_live_dir(&p) {
                return Some(p);
            }
        }
    }
    None
}

fn find_riot_client(
    program_data: &Path,
    install_dir: Option<&Path>,
    scan: bool,
) -> Option<PathBuf> {
    let installs = program_data
        .join("Riot Games")
        .join("RiotClientInstalls.json");
    if let Ok(text) = fs::read_to_string(installs) {
        let dir = install_dir.map(to_display);
        for c in parse_riot_client_installs(&text, dir.as_deref()) {
            let p = PathBuf::from(&c);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    // <root>/VALORANT/live → <root>/Riot Client/RiotClientServices.exe
    if let Some(root) = install_dir
        .and_then(|d| d.parent())
        .and_then(|d| d.parent())
    {
        let p = root.join("Riot Client").join("RiotClientServices.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    if scan {
        for root in drive_roots() {
            let p = root.join(r"Riot Games\Riot Client\RiotClientServices.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// True if we can create and delete a file in `dir` without elevation.
pub fn probe_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".veil-write-test-{}.tmp", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(f) => {
            drop(f);
            fs::remove_file(&probe).is_ok()
        }
        Err(_) => false,
    }
}

pub fn discover(opts: &DiscoveryOptions) -> InstallInfo {
    let mut found: Option<(PathBuf, InstallSource)> = None;
    if let Some(o) = opts
        .install_override
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        match normalize_live_dir(Path::new(o.trim())) {
            Some(p) => found = Some((p, InstallSource::Override)),
            None => {
                log::warn!("install override {o} is not a VALORANT live folder; auto-detecting")
            }
        }
    }
    if found.is_none() {
        found = from_metadata(&opts.program_data).map(|p| (p, InstallSource::Metadata));
    }
    if found.is_none() && opts.use_registry {
        found = from_registry().map(|p| (p, InstallSource::Registry));
    }
    if found.is_none() && opts.scan_drives {
        found = from_scan().map(|p| (p, InstallSource::Scan));
    }

    let riot_client = find_riot_client(
        &opts.program_data,
        found.as_ref().map(|(p, _)| p.as_path()),
        opts.scan_drives,
    );
    let Some((live, source)) = found else {
        return InstallInfo {
            riot_client: riot_client.map(|p| to_display(&p)),
            ..InstallInfo::none()
        };
    };
    let menu = menu_dir_of(&live);
    let writable = opts.probe_writable && probe_writable(&menu);
    InstallInfo {
        install_dir: Some(to_display(&live)),
        menu_dir: Some(to_display(&menu)),
        riot_client: riot_client.map(|p| to_display(&p)),
        source,
        writable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_product_settings_line() {
        let yaml = "foo: 1\n  product_install_full_path: \"D:/Riot Games/VALORANT/live\"\nproduct_install_root: \"D:/Riot Games\"\n";
        assert_eq!(
            parse_product_settings(yaml).as_deref(),
            Some("D:/Riot Games/VALORANT/live")
        );
        assert_eq!(parse_product_settings("nothing: here"), None);
    }

    #[test]
    fn parses_riot_client_installs() {
        let json = r#"{
            "associated_client": {"D:/Riot Games/VALORANT/live/": "D:/Riot Games/Riot Client/RiotClientServices.exe"},
            "rc_default": "C:/Other/RiotClientServices.exe",
            "rc_live": "D:/Riot Games/Riot Client/RiotClientServices.exe"
        }"#;
        let c = parse_riot_client_installs(json, Some("D:/Riot Games/VALORANT/live"));
        assert_eq!(
            c,
            vec![
                "D:/Riot Games/Riot Client/RiotClientServices.exe",
                "C:/Other/RiotClientServices.exe"
            ]
        );
        let c = parse_riot_client_installs(json, None);
        assert_eq!(c[0], "D:/Riot Games/Riot Client/RiotClientServices.exe");
    }

    fn fake_install(root: &Path) -> PathBuf {
        let live = root.join("Riot Games").join("VALORANT").join("live");
        fs::create_dir_all(menu_dir_of(&live)).unwrap();
        live
    }

    #[test]
    fn normalizes_override_variants() {
        let dir = tempfile::tempdir().unwrap();
        let live = fake_install(dir.path());
        assert_eq!(normalize_live_dir(&live), Some(live.clone()));
        assert_eq!(
            normalize_live_dir(live.parent().unwrap()),
            Some(live.clone())
        );
        assert_eq!(normalize_live_dir(&menu_dir_of(&live)), Some(live.clone()));
        assert_eq!(normalize_live_dir(dir.path()), None);
    }

    #[test]
    fn discovery_prefers_valid_override_then_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let live = fake_install(dir.path());
        let pd = dir.path().join("ProgramData");
        let meta = pd.join("Riot Games").join("Metadata").join("valorant.live");
        fs::create_dir_all(&meta).unwrap();
        fs::write(
            meta.join("valorant.live.product_settings.yaml"),
            format!("product_install_full_path: \"{}\"\n", to_display(&live)),
        )
        .unwrap();
        let rc = dir.path().join("Riot Games").join("Riot Client");
        fs::create_dir_all(&rc).unwrap();
        fs::write(rc.join("RiotClientServices.exe"), b"").unwrap();

        let mut opts = DiscoveryOptions {
            install_override: None,
            probe_writable: true,
            program_data: pd.clone(),
            use_registry: false,
            scan_drives: false,
        };
        let info = discover(&opts);
        assert_eq!(info.source, InstallSource::Metadata);
        assert_eq!(
            info.install_dir.as_deref(),
            Some(to_display(&live).as_str())
        );
        assert!(
            info.menu_dir
                .unwrap()
                .ends_with("ShooterGame/Content/Movies/Menu")
        );
        assert!(
            info.riot_client
                .unwrap()
                .ends_with("Riot Client/RiotClientServices.exe")
        );
        assert!(info.writable);

        // Invalid override falls back to metadata.
        opts.install_override = Some(to_display(&dir.path().join("nope")));
        assert_eq!(discover(&opts).source, InstallSource::Metadata);

        // Valid override wins.
        let other = fake_install(&dir.path().join("X"));
        opts.install_override = Some(to_display(&other));
        let info = discover(&opts);
        assert_eq!(info.source, InstallSource::Override);
        assert_eq!(
            info.install_dir.as_deref(),
            Some(to_display(&other).as_str())
        );
    }

    #[test]
    fn nothing_found() {
        let dir = tempfile::tempdir().unwrap();
        let info = discover(&DiscoveryOptions {
            install_override: None,
            probe_writable: false,
            program_data: dir.path().to_path_buf(),
            use_registry: false,
            scan_drives: false,
        });
        assert_eq!(info.source, InstallSource::None);
        assert!(info.menu_dir.is_none() && !info.writable);
    }
}
