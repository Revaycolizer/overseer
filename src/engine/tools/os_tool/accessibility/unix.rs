use serde_json::json;
use crate::engine::tools::error::DynamicToolError;

pub async fn windows_accessibility_state_json(
    _tool_name: &str,
    _input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    Ok(json!({
        "action": "inspect_accessibility",
        "matches": [],
        "match_count": 0,
        "focused_element": null,
        "summary": "Native accessibility inspection is not supported on Linux. Consider using browser tools."
    }).to_string())
}
