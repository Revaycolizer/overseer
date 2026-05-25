use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::stringify_tool_input;

pub async fn native_clipboard_write(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let text = stringify_tool_input(args);
    if text.trim().is_empty() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "clipboard_write".to_string(),
            details: "clipboard_write requires text input.".to_string(),
        });
    }

    #[cfg(windows)]
    {
        let escaped = text.replace('\'', "''");
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard -Value '{}'", escaped),
            ])
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: err.to_string(),
            })?;

        if !output.status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: err.to_string(),
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await.map_err(|err| {
                DynamicToolError::ExecutionFailed {
                    tool: "clipboard_write".to_string(),
                    details: err.to_string(),
                }
            })?;
        }

        let status = child
            .wait()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: err.to_string(),
            })?;

        if !status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: "pbcopy failed.".to_string(),
            });
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let candidates = [
            ("wl-copy", Vec::<&str>::new()),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
        ];

        let mut succeeded = false;
        for (cmd, args) in candidates {
            let spawned = Command::new(cmd)
                .args(args)
                .stdin(Stdio::piped())
                .kill_on_drop(true)
                .spawn();

            let Ok(mut child) = spawned else {
                continue;
            };

            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                if stdin.write_all(text.as_bytes()).await.is_err() {
                    continue;
                }
            }

            if let Ok(status) = child.wait().await {
                if status.success() {
                    succeeded = true;
                    break;
                }
            }
        }

        if !succeeded {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: "No supported clipboard writer found (tried wl-copy, xclip, xsel)."
                    .to_string(),
            });
        }
    }

    Ok(json!({
        "action": "clipboard_write",
        "written": true,
        "characters": text.chars().count(),
    })
    .to_string())
}
