use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::read_string_field;
use serde_json::json;

pub async fn handle_focus_element(
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let name_hint = read_string_field(input, &["name", "selector", "automation_id", "title"])
        .unwrap_or_default();
    if name_hint.is_empty() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: "focus_element on Linux requires a `name` or `title` field to search for the window.".to_string(),
        });
    }
    let output = tokio::process::Command::new("xdotool")
        .args(["search", "--name", &name_hint, "windowactivate"])
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!(
                "xdotool not found or failed: {}. Install with: sudo apt install xdotool",
                e
            ),
        })?;
    let success = output.status.success();
    Ok(json!({
        "action": "focus_element",
        "method": "xdotool_search_activate",
        "name": name_hint,
        "ok": success,
        "summary": if success { format!("Focused window matching '{}' using xdotool.", name_hint) } else { format!("Could not find window matching '{}' via xdotool.", name_hint) }
    }).to_string())
}

pub async fn handle_invoke_element(
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let name_hint = read_string_field(input, &["name", "selector", "automation_id", "title"])
        .unwrap_or_default();
    if name_hint.is_empty() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: "invoke_element on Linux requires a `name` to locate the target.".to_string(),
        });
    }
    let focus_output = tokio::process::Command::new("xdotool")
        .args(["search", "--name", &name_hint, "windowactivate"])
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("xdotool failed: {}", e),
        })?;
    if focus_output.status.success() {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = tokio::process::Command::new("xdotool")
            .args(["key", "Return"])
            .output()
            .await;
    }
    Ok(json!({
        "action": "invoke_element",
        "method": "xdotool_focus_enter",
        "name": name_hint,
        "ok": focus_output.status.success(),
        "summary": format!("Attempted invoke on '{}' using xdotool focus + Enter.", name_hint)
    })
    .to_string())
}

pub async fn handle_type_text(
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let name_hint = read_string_field(input, &["name", "selector", "automation_id", "title"])
        .unwrap_or_default();
    let text = read_string_field(input, &["text", "value", "content"]).unwrap_or_default();
    if !name_hint.is_empty() {
        let _ = tokio::process::Command::new("xdotool")
            .args(["search", "--name", &name_hint, "windowactivate"])
            .output()
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    if !text.is_empty() {
        let output = tokio::process::Command::new("xdotool")
            .args(["type", "--clearmodifiers", &text])
            .output()
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: format!("xdotool type failed: {}", e),
            })?;
        return Ok(json!({
            "action": "type_text",
            "method": "xdotool_type",
            "typed_chars": text.chars().count(),
            "ok": output.status.success(),
            "summary": format!("Typed {} characters using xdotool.", text.chars().count())
        })
        .to_string());
    }
    Ok(json!({ "action": "type_text", "method": "xdotool_focus", "ok": true }).to_string())
}
