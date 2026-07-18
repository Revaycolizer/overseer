use crate::engine::tools::error::DynamicToolError;

pub async fn windows_uia_interaction_json(
    tool_name: &str,
    action: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: "Native UI Automation interaction is not yet supported on macOS.".to_string(),
        });
    }

    match action {
        "focus_element" => super::unix_handler::handle_focus_element(tool_name, input).await,
        "invoke_element" => super::unix_handler::handle_invoke_element(tool_name, input).await,
        "type_text" => super::unix_handler::handle_type_text(tool_name, input).await,
        _ => Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("UIA action '{}' is not supported on Linux.", action),
        }),
    }
}
