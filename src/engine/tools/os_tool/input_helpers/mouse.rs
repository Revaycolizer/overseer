use enigo::{Enigo, Settings};
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::read_i64_field;

pub fn extract_coordinates(input: &serde_json::Value) -> Option<(i32, i32)> {
    let x = read_i64_field(input, &["x", "left"]).map(|value| value as i32);
    let y = read_i64_field(input, &["y", "top"]).map(|value| value as i32);
    match (x, y) {
        (Some(x), Some(y)) => return Some((x, y)),
        _ => {}
    }

    if let Some(coords) = input.get("coords").and_then(|value| value.as_array()) {
        if coords.len() >= 2 {
            let x = coords.first().and_then(|value| value.as_i64())? as i32;
            let y = coords.get(1).and_then(|value| value.as_i64())? as i32;
            return Some((x, y));
        }
    }

    None
}

pub fn with_enigo() -> Result<Enigo, DynamicToolError> {
    Enigo::new(&Settings::default()).map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "os_tool".to_string(),
        details: format!("Failed to initialize native desktop input: {}", err),
    })
}
