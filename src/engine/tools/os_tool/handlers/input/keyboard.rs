use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::input_helpers::{
    extract_key_tokens, extract_text_value, key_from_token, with_enigo,
};
use crate::engine::tools::os_tool::uia_interaction::windows_uia_interaction_json;
use enigo::{Direction, Keyboard};
use serde_json::json;

pub async fn handle_keyboard_action(
    action: &str,
    input: &serde_json::Value,
    selector_present: bool,
) -> Result<String, DynamicToolError> {
    match action {
        "type_text" => {
            let text =
                extract_text_value(input).ok_or_else(|| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Type text requires a `text` value.".to_string(),
                })?;
            if selector_present {
                let raw = windows_uia_interaction_json("os_tool", "type_text", input).await?;
                let mut value: serde_json::Value =
                    serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
                let method = value.get("method").and_then(|v| v.as_str()).unwrap_or("");
                if method != "value_pattern" {
                    let mut enigo = with_enigo()?;
                    enigo
                        .text(&text)
                        .map_err(|err| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to type text after focusing element: {}", err),
                        })?;
                    if let Some(object) = value.as_object_mut() {
                        object.insert(
                            "method".to_string(),
                            serde_json::Value::String("uia_focus_then_type".to_string()),
                        );
                    }
                }
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "typed_chars".to_string(),
                        serde_json::Value::Number((text.chars().count() as u64).into()),
                    );
                    object.insert("ok".to_string(), serde_json::Value::Bool(true));
                }
                return Ok(value.to_string());
            }

            let mut enigo = with_enigo()?;
            enigo
                .text(&text)
                .map_err(|err| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Failed to type text: {}", err),
                })?;
            Ok(json!({
                "action": "type_text",
                "typed_chars": text.chars().count(),
                "method": "keyboard_text",
                "ok": true
            })
            .to_string())
        }
        "press_key" => {
            let mut enigo = with_enigo()?;
            let key = extract_key_tokens(input)
                .into_iter()
                .next()
                .ok_or_else(|| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Press key requires a `key` value.".to_string(),
                })?;
            let resolved =
                key_from_token(&key).ok_or_else(|| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Unsupported key token '{}'.", key),
                })?;
            enigo.key(resolved, Direction::Click).map_err(|err| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Failed to press key: {}", err),
                }
            })?;
            Ok(json!({
                "action": "press_key",
                "key": key,
                "ok": true
            })
            .to_string())
        }
        _ => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Unsupported keyboard action '{}'", action),
        }),
    }
}
