use serde_json::json;
use enigo::{Axis, Mouse};
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::input_helpers::with_enigo;
use crate::engine::tools::input::{read_i64_field, read_string_field, stringify_tool_input};

pub async fn handle_scroll(input: &serde_json::Value) -> Result<String, DynamicToolError> {
    let mut enigo = with_enigo()?;
    let amount = read_i64_field(input, &["amount", "delta", "lines"])
        .unwrap_or_else(|| {
            let direction = read_string_field(input, &["direction"])
                .unwrap_or_else(|| stringify_tool_input(input))
                .to_lowercase();
            if direction.contains("up") {
                -3
            } else {
                3
            }
        }) as i32;
    let axis = match read_string_field(input, &["axis"])
        .unwrap_or_else(|| "vertical".to_string())
        .to_lowercase()
        .as_str()
    {
        "horizontal" | "x" => Axis::Horizontal,
        _ => Axis::Vertical,
    };
    enigo
        .scroll(amount, axis)
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Failed to scroll: {}", err),
        })?;
    Ok(json!({
        "action": "scroll",
        "amount": amount,
        "axis": if matches!(axis, Axis::Horizontal) { "horizontal" } else { "vertical" },
        "ok": true
    })
    .to_string())
}
