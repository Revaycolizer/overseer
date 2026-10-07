use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::find_files::windows_find_files_json;
use crate::engine::tools::os_tool::open_file::windows_open_file_json;
use crate::engine::tools::os_tool::read_file::windows_read_file_json;
use crate::engine::tools::os_tool::screenshot::capture_windows_screenshot_json;
use crate::engine::tools::os_tool::target_extraction::extract_os_tool_target;

pub async fn handle_files_action(
    action: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    match action {
        "find_files" => {
            let query = extract_os_tool_target(input, "find_files");
            windows_find_files_json("os_tool", &query).await
        }
        "read_file" => {
            let path = extract_os_tool_target(input, "read_file");
            windows_read_file_json("os_tool", &path).await
        }
        "open_file" => {
            let path = extract_os_tool_target(input, "open_file");
            windows_open_file_json("os_tool", &path).await
        }
        "screenshot" => capture_windows_screenshot_json("os_tool").await,
        _ => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Unsupported files action '{}'", action),
        }),
    }
}
