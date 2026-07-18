use serde_json::json;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub async fn windows_open_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
    let mut target_path = path.to_string();

    if !cfg!(target_os = "macos") && !std::path::Path::new(&target_path).exists() {
        let lower = target_path.to_lowercase();
        let well_known = [
            "downloads",
            "documents",
            "pictures",
            "videos",
            "music",
            "desktop",
            "templates",
            "public",
        ];

        if well_known.iter().any(|&f| lower == f) {
            if let Ok(home) = std::env::var("HOME") {
                let capitalized = format!("{}{}", &lower[0..1].to_uppercase(), &lower[1..]);
                let home_path = std::path::PathBuf::from(home).join(&capitalized);
                if home_path.exists() {
                    target_path = home_path.to_string_lossy().to_string();
                }
            }
        }
    }

    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(cmd)
        .arg(&target_path)
        .status()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to launch {} for '{}': {}", cmd, target_path, e),
        })?;

    if !status.success() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!(
                "{} failed with status {} for '{}'.",
                cmd, status, target_path
            ),
        });
    }

    Ok(json!({
        "action": "open_file",
        "path": target_path,
        "success": true,
        "summary": format!("Launched '{}' using {}.", target_path, cmd)
    })
    .to_string())
}
