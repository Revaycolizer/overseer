use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};

pub async fn spawn_obscura(
    port: u16,
    profile_path: &Path,
    stealth: bool,
) -> Result<(Child, String)> {
    let binary = super::obscura_resolver::resolve_obscura()
        .context("Obscura binary not found")?;

    std::fs::create_dir_all(profile_path)?;

    let mut cmd = Command::new(&binary);
    cmd.args(["serve", "--port", &port.to_string()])
        .env("OBSCURA_PROFILE_DIR", profile_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    if stealth {
        cmd.arg("--stealth");
    }

    let child = cmd.spawn().context("Failed to spawn Obscura process")?;

    let ws_url = format!("ws://127.0.0.1:{port}/devtools/browser");
    Ok((child, ws_url))
}
