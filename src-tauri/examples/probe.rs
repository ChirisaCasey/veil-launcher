//! READ-ONLY: print InstallInfo + DetectionResult for this machine.
//! Never writes anything (the writable probe is disabled).
//!
//!     cargo run --example probe

use veil_launcher_lib::core::{detect, paths};

fn main() {
    let opts = paths::DiscoveryOptions {
        probe_writable: false,
        ..Default::default()
    };
    let info = paths::discover(&opts);
    println!(
        "InstallInfo (writable probe skipped):\n{}",
        serde_json::to_string_pretty(&info).unwrap()
    );
    let r = detect::detect(info.menu_dir.as_deref().map(std::path::Path::new), None);
    println!(
        "DetectionResult:\n{}",
        serde_json::to_string_pretty(&r).unwrap()
    );
    match &r.selected {
        Some(c) => println!("\nSELECTED: {}  ({})", c.file_name, c.reason),
        None => println!("\nSELECTED: <none>"),
    }
}
