use crate::engine::tools::error::DynamicToolError;

pub async fn windows_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        crate::engine::tools::os_tool::browser_state::macos::macos_browser_state_json(tool_name).await
    } else {
        crate::engine::tools::os_tool::browser_state::linux::linux_browser_state_json(tool_name).await
    }
}
