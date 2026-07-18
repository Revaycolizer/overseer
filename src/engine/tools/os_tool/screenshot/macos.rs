use serde_json::json;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub async fn capture_macos_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("overseer_capture_{}.png", uuid::Uuid::new_v4()));
    let path_str = temp_path.to_string_lossy().to_string();

    let output = Command::new("screencapture")
        .arg("-x")
        .arg(&path_str)
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run screencapture: {}", e),
        })?;

    if !output.status.success() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let mut width = 1440;
    let mut height = 900;
    if let Ok(res_out) = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&res_out.stdout);
        for line in stdout.lines() {
            if line.contains("Resolution:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let dims: Vec<&str> = parts[1].split('x').collect();
                    if dims.len() >= 2 {
                        width = dims[0].parse().unwrap_or(1440);
                        height = dims[1].parse().unwrap_or(900);
                    }
                }
            }
        }
    }

    Ok(json!({
        "action": "screenshot",
        "path": path_str,
        "width": width,
        "height": height,
        "summary": format!("Captured a macOS desktop screenshot at {}x{}.", width, height)
    })
    .to_string())
}
