pub mod native;
pub mod python;
pub mod rust;

use crate::engine::tools::error::DynamicToolError;
use crate::engine::utils::discovery::ToolInfo;
use native::call_native_builtin_tool;
use python::execute_python_tool;
use rust::execute_rust_tool;

pub async fn call_dynamic_tool(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    call_dynamic_tool_logic(info, args).await
}

pub async fn call_dynamic_tool_logic(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    if let Some(result) = call_native_builtin_tool(info, &args).await? {
        return Ok(result);
    }

    match info.language.as_str() {
        "python" => execute_python_tool(info, args).await,
        "rust" => execute_rust_tool(info, args).await,
        _ => Err(DynamicToolError::UnsupportedLanguage(info.language.clone())),
    }
}
