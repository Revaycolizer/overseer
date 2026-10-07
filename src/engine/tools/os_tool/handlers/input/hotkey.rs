use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::input_helpers::{
    extract_key_tokens, key_from_token, with_enigo,
};
use enigo::{Direction, Key, Keyboard};
use serde_json::json;

pub async fn handle_hotkey(input: &serde_json::Value) -> Result<String, DynamicToolError> {
    let mut enigo = with_enigo()?;
    let keys = extract_key_tokens(input);
    if keys.len() < 2 {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: "Hotkey requires at least two key tokens, like `Ctrl+L`.".to_string(),
        });
    }
    let resolved: Vec<Key> = keys
        .iter()
        .map(|token| {
            key_from_token(token).ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Unsupported key token '{}'.", token),
            })
        })
        .collect::<Result<_, _>>()?;

    for key in &resolved[..resolved.len() - 1] {
        enigo
            .key(*key, Direction::Press)
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Failed to press hotkey modifier: {}", err),
            })?;
    }
    let final_key = *resolved.last().expect("checked len");
    enigo
        .key(final_key, Direction::Click)
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Failed to trigger hotkey: {}", err),
        })?;
    for key in resolved[..resolved.len() - 1].iter().rev() {
        enigo
            .key(*key, Direction::Release)
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Failed to release hotkey modifier: {}", err),
            })?;
    }
    Ok(json!({
        "action": "hotkey",
        "keys": keys,
        "ok": true
    })
    .to_string())
}
