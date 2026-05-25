use serde_json::json;
use enigo::{Button, Coordinate, Direction, Mouse};
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::input_helpers::{with_enigo, extract_coordinates};
use crate::engine::tools::os_tool::uia_interaction::windows_uia_interaction_json;

pub async fn handle_mouse_action(
    action: &str,
    input: &serde_json::Value,
    selector_present: bool,
) -> Result<String, DynamicToolError> {
    match action {
        "click" | "double_click" | "right_click" => {
            if action == "click" && selector_present && extract_coordinates(input).is_none() {
                return windows_uia_interaction_json("os_tool", "invoke_element", input).await;
            }
            let mut enigo = with_enigo()?;
            let (x, y) = extract_coordinates(input).ok_or_else(|| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!(
                        "{} action requires `x` and `y` coordinates.",
                        action
                    ),
                }
            })?;
            enigo
                .move_mouse(x, y, Coordinate::Abs)
                .map_err(|err| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Failed to move mouse: {}", err),
                })?;
            let button = if action == "right_click" {
                Button::Right
            } else {
                Button::Left
            };
            enigo
                .button(button, Direction::Click)
                .map_err(|err| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Failed to click mouse: {}", err),
                })?;
            if action == "double_click" {
                enigo
                    .button(button, Direction::Click)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to complete double click: {}", err),
                    })?;
            }
            Ok(json!({
                "action": action,
                "x": x,
                "y": y,
                "button": if action == "right_click" { "right" } else { "left" },
                "ok": true
            })
            .to_string())
        }
        "move_mouse" => {
            let mut enigo = with_enigo()?;
            let (x, y) = extract_coordinates(input).ok_or_else(|| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Move mouse requires `x` and `y` coordinates.".to_string(),
                }
            })?;
            enigo
                .move_mouse(x, y, Coordinate::Abs)
                .map_err(|err| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Failed to move mouse: {}", err),
                })?;
            Ok(json!({
                "action": "move_mouse",
                "x": x,
                "y": y,
                "ok": true
            })
            .to_string())
        }
        _ => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Unsupported mouse action '{}'", action),
        }),
    }
}
