use crate::engine::tools::error::DynamicToolError;

pub async fn windows_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        crate::engine::tools::os_tool::desktop_state::macos::macos_desktop_state_json(tool_name).await
    } else {
        crate::engine::tools::os_tool::desktop_state::linux::linux_desktop_state_json(tool_name).await
    }
}
