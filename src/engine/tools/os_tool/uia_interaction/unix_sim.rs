use crate::engine::tools::error::DynamicToolError;
use std::process::Command;

pub async fn run_unix_command(cmd_str: &str) -> Result<String, DynamicToolError> {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "unix_shell".to_string(),
            details: "Empty command".to_string(),
        });
    }
    let output = tokio::process::Command::new(parts[0])
        .args(&parts[1..])
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: "unix_shell".to_string(),
            details: format!("Command failed: {}", e),
        })?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(DynamicToolError::ExecutionFailed {
            tool: "unix_shell".to_string(),
            details: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub async fn simulate_unix_click(x: i32, y: i32) -> Result<String, DynamicToolError> {
    if crate::engine::tools::os_tool::wayland_compat::is_wayland() {
        if Command::new("ydotool").arg("-h").output().is_ok() {
            let _ = Command::new("ydotool").args(["mousemove", &x.to_string(), &y.to_string()]).output();
            let out = Command::new("ydotool").args(["click", "0xC0"]).output();
            if let Ok(o) = out {
                if o.status.success() { return Ok("Click simulated via ydotool".to_string()); }
            }
        }
    }
    let cmd = format!("xdotool mousemove {} {} click 1", x, y);
    run_unix_command(&cmd).await
}

pub async fn simulate_unix_type_text(text: &str) -> Result<String, DynamicToolError> {
    if crate::engine::tools::os_tool::wayland_compat::is_wayland() {
        match crate::engine::tools::os_tool::wayland_compat::wayland_type_text(text) {
            Ok(_) => return Ok("Text typed via Wayland simulator".to_string()),
            Err(e) => eprintln!("Wayland type_text failed: {}. Falling back to xdotool...", e),
        }
    }
    let escaped = text.replace("'", "'\\''");
    let cmd = format!("xdotool type '{}'", escaped);
    run_unix_command(&cmd).await
}

pub async fn simulate_unix_press_key(key: &str) -> Result<String, DynamicToolError> {
    if crate::engine::tools::os_tool::wayland_compat::is_wayland() {
        match crate::engine::tools::os_tool::wayland_compat::wayland_press_key(key) {
            Ok(_) => return Ok("Key pressed via Wayland simulator".to_string()),
            Err(e) => eprintln!("Wayland press_key failed: {}. Falling back to xdotool...", e),
        }
    }
    let cmd = format!("xdotool key '{}'", key);
    run_unix_command(&cmd).await
}
