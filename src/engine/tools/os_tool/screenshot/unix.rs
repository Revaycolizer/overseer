use crate::engine::tools::error::DynamicToolError;

pub async fn capture_windows_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        crate::engine::tools::os_tool::screenshot::macos::capture_macos_screenshot_json(tool_name)
            .await
    } else {
        crate::engine::tools::os_tool::screenshot::linux::capture_linux_screenshot_json(tool_name)
            .await
    }
}
