use crate::engine::tools::error::DynamicToolError;
use serde_json::json;
use tokio::process::Command;

pub async fn native_clipboard_read() -> Result<String, DynamicToolError> {
    #[cfg(windows)]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard"])
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: err.to_string(),
            })?;

        if !output.status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(json!({
            "action": "clipboard_read",
            "text": text,
        })
        .to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("pbpaste")
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: err.to_string(),
            })?;

        if !output.status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(json!({
            "action": "clipboard_read",
            "text": text,
        })
        .to_string());
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let candidates = [
            ("wl-paste", vec!["-n"]),
            ("xclip", vec!["-o", "-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--output"]),
        ];

        for (cmd, args) in candidates {
            let mut command = Command::new(cmd);
            command.args(args).kill_on_drop(true);
            if let Ok(output) = command.output().await {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return Ok(json!({
                        "action": "clipboard_read",
                        "text": text,
                    })
                    .to_string());
                }
            }
        }

        Err(DynamicToolError::ExecutionFailed {
            tool: "clipboard_read".to_string(),
            details: "No supported clipboard reader found (tried wl-paste, xclip, xsel)."
                .to_string(),
        })
    }
}
