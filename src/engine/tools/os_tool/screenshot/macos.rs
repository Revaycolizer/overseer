use crate::engine::tools::error::DynamicToolError;
use serde_json::json;
use tokio::process::Command;

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

    // Read the PNG header instead of launching system_profiler on every capture.
    // This reports the actual captured pixels, including Retina/multi-display sizes.
    let (width, height) =
        image::image_dimensions(&temp_path).map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to inspect captured screenshot: {}", e),
        })?;

    Ok(json!({
        "action": "screenshot",
        "path": path_str,
        "width": width,
        "height": height,
        "summary": format!("Captured a macOS desktop screenshot at {}x{}.", width, height)
    })
    .to_string())
}
