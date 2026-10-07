pub async fn export_to_downloads(asset_path: &str) -> anyhow::Result<String> {
    let args = serde_json::json!({ "path": asset_path });
    match crate::engine::tools::native_export_asset(&args).await {
        Ok(res) => {
            let json: serde_json::Value = serde_json::from_str(&res)?;
            Ok(json["message"]
                .as_str()
                .unwrap_or("File exported.")
                .to_string())
        }
        Err(e) => Err(anyhow::anyhow!("Export failed: {}", e)),
    }
}

pub async fn save_chart_export(
    filename: String,
    data: String, // String for SVG or Base64 for PNG
    format: String,
) -> Result<String, String> {
    // RESOLVE: System Downloads Folder (Cross-Platform)
    let download_dir = dirs::download_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("assets/exports"))
        .join("Overseer")
        .join("Charts");

    if !download_dir.exists() {
        let _ = std::fs::create_dir_all(&download_dir);
    }

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let safe_filename =
        filename.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
    let full_filename = format!("{}_{}.{}", safe_filename, timestamp, format.to_lowercase());
    let path = download_dir.join(&full_filename);

    if format.to_lowercase() == "svg" {
        std::fs::write(&path, data).map_err(|e| e.to_string())?;
    } else {
        // Assume Base64-encoded binary (PNG/JPG)
        let clean_base64 = data.split(',').last().unwrap_or(&data);
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(clean_base64.trim())
            .map_err(|e| format!("Base64 decode error: {e}"))?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }

    // CROSS-PLATFORM: Open destination folder
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(&download_dir)
        .spawn();

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg(&download_dir)
        .spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(&download_dir)
        .spawn();

    Ok(full_filename)
}
