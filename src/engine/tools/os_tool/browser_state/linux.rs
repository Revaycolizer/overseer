use crate::engine::tools::error::DynamicToolError;
use serde_json::json;
use tokio::process::Command;

pub async fn linux_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let output = Command::new("wmctrl")
        .arg("-l")
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run wmctrl: {}", e),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut browser_windows = Vec::new();

    let browser_keywords = [
        "chrome", "firefox", "chromium", "brave", "edge", "safari", "opera", "vivaldi",
    ];

    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if browser_keywords.iter().any(|&k| lower.contains(k)) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let title = if parts.len() >= 4 {
                parts[3..].join(" ")
            } else {
                line.to_string()
            };
            browser_windows.push(json!({
                "title": title,
                "ProcessName": parts.get(2).unwrap_or(&"unknown"),
                "Id": parts.first().unwrap_or(&"0")
            }));
        }
    }

    Ok(json!({
        "action": "browser_state",
        "browser_windows": browser_windows,
        "summary": format!("Detected {} browser window(s) on Linux.", browser_windows.len())
    })
    .to_string())
}
