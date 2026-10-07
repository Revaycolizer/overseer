use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::{object_tool_input, read_string_field};
use crate::engine::tools::os_tool::desktop_state::windows_desktop_state_json;
use crate::engine::tools::os_tool::screenshot::capture_windows_screenshot_json;
use serde_json::json;
use std::path::PathBuf;

pub fn runtime_artifacts_dir() -> Result<PathBuf, DynamicToolError> {
    let dir = crate::engine::utils::discovery::runtime_dir().join("artifacts");
    std::fs::create_dir_all(&dir).map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "vision_analysis".to_string(),
        details: format!("Failed to prepare runtime artifacts directory: {}", err),
    })?;
    Ok(dir)
}

pub async fn native_vision_analysis(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let desktop_state = windows_desktop_state_json("vision_analysis").await?;
    let screenshot = capture_windows_screenshot_json("vision_analysis").await?;

    let source = read_string_field(&input, &["image", "path", "url", "input"])
        .unwrap_or_else(|| "live desktop session".to_string());

    Ok(json!({
        "mode": "desktop_snapshot",
        "source": source,
        "note": "Native vision analysis reports desktop state, visible windows, and a screenshot artifact. OCR and full UI semantic understanding are still limited.",
        "state": serde_json::from_str::<serde_json::Value>(&desktop_state).unwrap_or_else(|_| json!({ "raw": desktop_state })),
        "screenshot": serde_json::from_str::<serde_json::Value>(&screenshot).unwrap_or_else(|_| json!({ "raw": screenshot })),
    })
    .to_string())
}
