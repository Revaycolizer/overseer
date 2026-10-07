use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::accessibility::windows_accessibility_state_json;
use crate::engine::tools::os_tool::browser_state::windows_browser_state_json;
use crate::engine::tools::os_tool::desktop_state::windows_desktop_state_json;
use crate::engine::tools::os_tool::process_list::windows_process_list_json;

pub async fn handle_state_action(
    action: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    match action {
        "get_state" | "list_windows" => windows_desktop_state_json("os_tool").await,
        "inspect_accessibility" => windows_accessibility_state_json("os_tool", input).await,
        "browser_state" => windows_browser_state_json("os_tool").await,
        "list_processes" => windows_process_list_json("os_tool").await,
        _ => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Unsupported state action '{}'", action),
        }),
    }
}
