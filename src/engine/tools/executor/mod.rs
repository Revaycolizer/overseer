pub mod native;
pub mod python;
pub mod rust;

use crate::engine::tools::error::DynamicToolError;
use crate::engine::utils::discovery::ToolInfo;
use native::call_native_builtin_tool;
use python::execute_python_tool;
use rust::execute_rust_tool;

// Kernel v0.1: the low-level executor is crate-internal. The single governed
// entry point is `crate::engine::kernel::kernel_execute`; nothing outside the
// crate may reach tool execution. A static test (overseer-kernel
// `no_ungoverned_execution_calls`) enforces the in-crate call allowlist.
pub(crate) async fn call_dynamic_tool(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    call_dynamic_tool_logic(info, args).await
}

pub(crate) async fn call_dynamic_tool_logic(
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
