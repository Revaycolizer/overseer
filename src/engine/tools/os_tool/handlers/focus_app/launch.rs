use crate::engine::tools::error::DynamicToolError;

#[cfg(windows)]
pub async fn handle_launch(app: &str) -> Result<String, DynamicToolError> {
    use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
    use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;

    let escaped = escape_powershell_single_quoted(app);
    let script = format!(
        "Start-Process '{app}'; [pscustomobject]@{{ action='launch'; app='{app}'; ok=$true }} | ConvertTo-Json -Compress",
        app = escaped
    );
    run_powershell_hidden(&script, "os_tool").await
}

#[cfg(not(windows))]
pub async fn handle_launch(app: &str) -> Result<String, DynamicToolError> {
    use serde_json::json;
    use tokio::process::Command;
    use crate::engine::tools::os_tool::handlers::focus_app::desktop_file::find_linux_desktop_file;

    if cfg!(target_os = "macos") {
        let status = Command::new("open")
            .arg("-a")
            .arg(app)
            .status()
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Failed to launch Mac app {} using open -a: {}", app, e),
            })?;
        if !status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("'open -a' failed to launch Mac app '{}'.", app),
            });
        }
    } else {
        let mut launched = false;
        if let Ok(_) = Command::new(app).spawn() {
            launched = true;
        }
        if !launched {
            if let Some(desktop_file) = find_linux_desktop_file(app).await {
                 if Command::new("gtk-launch").arg(&desktop_file).spawn().is_ok() {
                     launched = true;
                 }
            }
        }
        if !launched {
            let _ = Command::new("xdg-open").arg(app).spawn();
        }
    }

    Ok(json!({
        "action": "launch",
        "app": app,
        "success": true,
        "summary": format!("Submitted launch request for '{}' on {}.", app, if cfg!(target_os = "macos") { "macOS" } else { "Linux" })
    }).to_string())
}
