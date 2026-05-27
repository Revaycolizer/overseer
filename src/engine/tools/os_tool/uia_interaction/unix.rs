use serde_json::json;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::read_string_field;

/// Linux fallback for UIA interaction using xdotool/xprop.
/// Full Windows UIA Automation is not available, but we can handle
/// basic focus, invoke (click), and type operations via xdotool.
pub async fn windows_uia_interaction_json(
    tool_name: &str,
    action: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: "Native UI Automation interaction is not yet supported on macOS. Use coordinate-based mouse actions instead.".to_string(),
        });
    }

    // Linux: attempt xdotool-based fallbacks
    let name_hint = read_string_field(input, &["name", "selector", "automation_id", "title"])
        .unwrap_or_default();

    match action {
        "focus_element" => {
            // Try to focus a window by name using xdotool
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
                    details: format!("xdotool not found or failed: {}. Install with: sudo apt install xdotool", e),
                })?;
            let success = output.status.success();
            Ok(json!({
                "action": "focus_element",
                "method": "xdotool_search_activate",
                "name": name_hint,
                "ok": success,
                "summary": if success {
                    format!("Focused window matching '{}' using xdotool.", name_hint)
                } else {
                    format!("Could not find window matching '{}' via xdotool.", name_hint)
                }
            }).to_string())
        }
        "invoke_element" => {
            // On Linux without UIA, invoke is best approximated by focusing + clicking
            // If we have coordinates, use enigo; otherwise try xdotool key press
            if name_hint.is_empty() {
                return Err(DynamicToolError::ExecutionFailed {
                    tool: tool_name.to_string(),
                    details: "invoke_element on Linux requires a `name` to locate the target. Use coordinate-based click for precise element interaction.".to_string(),
                });
            }
            // Focus the window first, then send Enter as a basic invoke
            let focus_output = tokio::process::Command::new("xdotool")
                .args(["search", "--name", &name_hint, "windowactivate"])
                .output()
                .await
                .map_err(|e| DynamicToolError::ExecutionFailed {
                    tool: tool_name.to_string(),
                    details: format!("xdotool failed: {}. Install with: sudo apt install xdotool", e),
                })?;
            if focus_output.status.success() {
                // Brief pause then press Enter
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
            }).to_string())
        }
        "type_text" => {
            // Focus window by name, then type using xdotool
            let text = read_string_field(input, &["text", "value", "content"])
                .unwrap_or_default();
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
                        details: format!("xdotool type failed: {}. Install with: sudo apt install xdotool", e),
                    })?;
                return Ok(json!({
                    "action": "type_text",
                    "method": "xdotool_type",
                    "typed_chars": text.chars().count(),
                    "ok": output.status.success(),
                    "summary": format!("Typed {} characters using xdotool.", text.chars().count())
                }).to_string());
            }
            // Fallback: use value_pattern style return so keyboard.rs handles it
            Ok(json!({
                "action": "type_text",
                "method": "xdotool_focus",
                "ok": true
            }).to_string())
        }
        _ => {
            Err(DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: format!(
                    "UIA action '{}' is not supported on Linux. Supported Linux actions: focus_element, invoke_element, type_text. For precise element interaction, use coordinate-based mouse/keyboard actions instead.",
                    action
                ),
            })
        }
    }
}
