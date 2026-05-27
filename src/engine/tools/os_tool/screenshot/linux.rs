use serde_json::json;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub async fn capture_linux_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("overseer_capture_{}.png", uuid::Uuid::new_v4()));
    let path_str = temp_path.to_string_lossy().to_string();

    let mut command_args = vec![
        ("scrot", vec![path_str.clone()]),
        ("gnome-screenshot", vec!["-f".to_string(), path_str.clone()]),
        (
            "import",
            vec!["-window".to_string(), "root".to_string(), path_str.clone()],
        ),
    ];

    let mut captured = false;
    if crate::engine::tools::os_tool::wayland_compat::is_wayland() {
        if let Ok(_) = crate::engine::tools::os_tool::wayland_compat::capture_wayland_screenshot(&path_str) {
            captured = true;
        }
    }

    if !captured {
        for (cmd, args) in command_args {
            if Command::new(cmd).arg("--version").output().await.is_ok() {
                let output = Command::new(cmd).args(args).output().await.map_err(|e| {
                    DynamicToolError::ExecutionFailed {
                        tool: tool_name.to_string(),
                        details: e.to_string(),
                    }
                })?;
                if output.status.success() {
                    captured = true;
                    break;
                }
            }
        }
    }

    if !captured {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: "No supported Linux screenshot utility found (tried scrot, gnome-screenshot, import).".to_string(),
        });
    }

    let mut width = 1920;
    let mut height = 1080;
    if let Ok(output) = Command::new("xdpyinfo").output().await {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("dimensions:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let dims: Vec<&str> = parts[1].split('x').collect();
                    if dims.len() >= 2 {
                        width = dims[0].parse().unwrap_or(1920);
                        height = dims[1].parse().unwrap_or(1080);
                    }
                }
            }
        }
    }

    Ok(json!({
        "action": "screenshot",
        "path": path_str,
        "width": width,
        "height": height,
        "summary": format!("Captured a Linux desktop screenshot at {}x{}.", width, height)
    })
    .to_string())
}
