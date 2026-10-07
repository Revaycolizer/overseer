use crate::engine::tools::error::DynamicToolError;

#[cfg(windows)]
pub async fn handle_focus(target: &str) -> Result<String, DynamicToolError> {
    use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;
    use crate::engine::tools::os_tool::powershell::run_powershell_hidden;

    let escaped = escape_powershell_single_quoted(target);
    let script = format!(
        "$wshell = New-Object -ComObject WScript.Shell; \
         $ok = $wshell.AppActivate('{target}'); \
         [pscustomobject]@{{ action='focus'; target='{target}'; activated=$ok }} | ConvertTo-Json -Compress",
        target = escaped
    );
    run_powershell_hidden(&script, "os_tool").await
}

#[cfg(not(windows))]
pub async fn handle_focus(target: &str) -> Result<String, DynamicToolError> {
    use serde_json::json;
    use tokio::process::Command;

    if cfg!(target_os = "macos") {
        let script = format!("tell application \"{}\" to activate", target);
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Failed to focus {} via AppleScript: {}", target, e),
            })?;
        if !status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("osascript failed to focus '{}'.", target),
            });
        }
        Ok(json!({
            "action": "focus",
            "target": target,
            "success": true,
            "summary": format!("Focused Mac app '{}' using AppleScript.", target)
        })
        .to_string())
    } else {
        let status = Command::new("wmctrl")
            .arg("-a")
            .arg(target)
            .status()
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Failed to run wmctrl for {}: {}", target, e),
            })?;
        if status.success() {
            return Ok(json!({
                "action": "focus",
                "target": target,
                "success": true,
                "summary": format!("Focused window matching '{}' using wmctrl.", target)
            })
            .to_string());
        }
        Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!(
                "wmctrl could not find or focus a window matching '{}'.",
                target
            ),
        })
    }
}
