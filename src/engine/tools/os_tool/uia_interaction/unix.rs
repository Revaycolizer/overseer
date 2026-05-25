use crate::engine::tools::error::DynamicToolError;

pub async fn windows_uia_interaction_json(
    tool_name: &str,
    _action: &str,
    _input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        details: "Native UI Automation interaction is not supported on Linux.".to_string(),
    })
}
