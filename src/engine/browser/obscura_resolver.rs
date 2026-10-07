use anyhow::Result;
use std::path::PathBuf;

const GITHUB_REPO: &str = "h4ckf0r0day/obscura";
const CACHE_DIR: &str = "overseer/bin";

fn data_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(CACHE_DIR);
    path
}

fn platform_triple() -> Option<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

pub fn resolve_obscura() -> Result<PathBuf> {
    if let Ok(path) = which::which("obscura") {
        return Ok(path);
    }

    let cached = data_dir().join("obscura");
    if cached.exists() {
        return Ok(cached);
    }

    let triple = platform_triple().unwrap_or("your-platform");
    anyhow::bail!(
        "Obscura not found. Install it via:\n  \
         curl -LO https://github.com/{}/releases/latest/download/obscura-{}.tar.gz\n  \
         tar xzf obscura-*.tar.gz\n  \
         sudo mv obscura /usr/local/bin/",
        GITHUB_REPO,
        triple
    );
}

pub async fn download_obscura() -> Result<PathBuf> {
    let triple = platform_triple().ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;

    let url = format!(
        "https://github.com/{GITHUB_REPO}/releases/latest/download/obscura-{triple}.tar.gz",
    );

    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let archive_path = dir.join("obscura.tar.gz");

    tracing::info!("Downloading Obscura from {url}...");
    let response = reqwest::get(&url).await?;
    let bytes = response.bytes().await?;
    tokio::fs::write(&archive_path, &bytes).await?;

    let archive = std::fs::File::open(&archive_path)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    tar.unpack(&dir)?;

    std::fs::remove_file(&archive_path)?;

    let binary = dir.join("obscura");
    if !binary.exists() {
        anyhow::bail!("Downloaded archive did not contain 'obscura' binary");
    }

    Ok(binary)
}
