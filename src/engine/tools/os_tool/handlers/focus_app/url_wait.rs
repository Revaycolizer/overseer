use crate::engine::tools::error::DynamicToolError;

#[cfg(windows)]
pub async fn handle_open_url(url: &str) -> Result<String, DynamicToolError> {
    use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;
    use crate::engine::tools::os_tool::powershell::run_powershell_hidden;

    let escaped = escape_powershell_single_quoted(url);
    let script = format!(
        "Start-Process '{url}'; [pscustomobject]@{{ action='open_url'; url='{url}'; ok=$true }} | ConvertTo-Json -Compress",
        url = escaped
    );
    run_powershell_hidden(&script, "os_tool").await
}

#[cfg(not(windows))]
pub async fn handle_open_url(url: &str) -> Result<String, DynamicToolError> {
    use serde_json::json;
    use tokio::process::Command;

    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(cmd).arg(url).status().await.map_err(|e| {
        DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Failed to open URL using {}: {}", cmd, e),
        }
    })?;
    if !status.success() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("{} failed with status {} for URL: {}.", cmd, status, url),
        });
    }
    Ok(json!({
        "action": "open_url",
        "url": url,
        "success": true,
        "summary": format!("Opened URL '{}' using {}.", url, cmd)
    })
    .to_string())
}

pub async fn handle_wait(duration_ms: u64) -> Result<String, DynamicToolError> {
    use serde_json::json;

    tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;
    Ok(json!({
        "action": "wait",
        "duration_ms": duration_ms,
        "ok": true
    })
    .to_string())
}
