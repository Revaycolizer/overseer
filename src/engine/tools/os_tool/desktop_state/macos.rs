use serde_json::json;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub async fn macos_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let active_app_out = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run osascript for active app: {}", e),
        })?;
    let active_title = String::from_utf8_lossy(&active_app_out.stdout)
        .trim()
        .to_string();

    let windows_out = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of every window of (every process whose background only is false)")
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run osascript for windows: {}", e),
        })?;
    let windows_raw = String::from_utf8_lossy(&windows_out.stdout)
        .trim()
        .to_string();

    let visible_windows: Vec<serde_json::Value> = windows_raw
        .split(", ")
        .take(12)
        .filter(|s| !s.is_empty() && *s != "missing value")
        .map(|name| {
            json!({
                "ProcessName": "unknown",
                "Id": "0",
                "MainWindowTitle": name
            })
        })
        .collect();

    Ok(json!({
        "action": "list_windows",
        "visible_windows": visible_windows,
        "active_title": active_title,
        "summary": format!("macOS desktop reported. Active app: {}. Detected {} window(s).", active_title, visible_windows.len())
    }).to_string())
}
