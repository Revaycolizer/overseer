use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::unix_command::run_command_simple;
use serde_json::json;

pub async fn linux_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let active_window_id = run_command_simple("xprop -root _NET_ACTIVE_WINDOW", tool_name).await?;
    let mut active_title = String::new();
    if let Some(id) = active_window_id.split_whitespace().last() {
        let title_out =
            run_command_simple(&format!("xprop -id {} _NET_WM_NAME", id), tool_name).await?;
        if let Some(pos) = title_out.find(" = \"") {
            active_title = title_out[pos + 4..].trim_end_matches('"').to_string();
        } else {
            let title_out_fallback =
                run_command_simple(&format!("xprop -id {} WM_NAME", id), tool_name).await?;
            if let Some(pos) = title_out_fallback.find(" = \"") {
                active_title = title_out_fallback[pos + 4..]
                    .trim_end_matches('"')
                    .to_string();
            }
        }
    }

    let windows_raw = run_command_simple("wmctrl -l", tool_name)
        .await
        .unwrap_or_default();
    let visible_windows: Vec<serde_json::Value> = windows_raw
        .lines()
        .take(12)
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let name = if parts.len() >= 4 {
                parts[3..].join(" ")
            } else {
                "Unknown Window".to_string()
            };
            json!({
                "ProcessName": "unknown",
                "Id": parts.get(0).copied().unwrap_or("0"),
                "MainWindowTitle": name
            })
        })
        .collect();

    Ok(json!({
        "action": "list_windows",
        "visible_windows": visible_windows,
        "active_title": active_title,
        "summary": format!("Linux desktop reported. Active window: {}. Detected {} window(s).", active_title, visible_windows.len())
    }).to_string())
}
