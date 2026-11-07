//! Launch VALORANT through the Riot Client.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Stdio;

use super::elevate::hidden_command;

pub const LAUNCH_ARGS: [&str; 2] = ["--launch-product=valorant", "--launch-patchline=live"];

pub fn launch_valorant(riot_client: Option<&str>) -> Result<()> {
    let Some(rc) = riot_client.filter(|p| Path::new(p).is_file()) else {
        bail!("Riot Client not found. Is VALORANT installed?");
    };
    hidden_command(rc)
        .args(LAUNCH_ARGS)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("starting {rc}"))?;
    log::info!("launched VALORANT via {rc}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_client_is_an_error() {
        assert!(launch_valorant(None).is_err());
        assert!(launch_valorant(Some("Z:/definitely/not/RiotClientServices.exe")).is_err());
    }
}
