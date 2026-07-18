use serde_json::json;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub async fn macos_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = "tell application \"System Events\" to get name of every window of (every process whose background only is false and (name contains \"Chrome\" or name contains \"Firefox\" or name contains \"Safari\" or name contains \"Edge\" or name contains \"Brave\"))";

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run osascript for browser state: {}", e),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut browser_windows = Vec::new();

    for name in stdout.split(", ") {
        if !name.is_empty() && name != "missing value" {
            browser_windows.push(json!({
                "title": name,
                "ProcessName": "unknown",
                "Id": "0"
            }));
        }
    }

    Ok(json!({
        "action": "browser_state",
        "browser_windows": browser_windows,
        "summary": format!("Detected {} browser window(s) on macOS.", browser_windows.len())
    })
    .to_string())
}
