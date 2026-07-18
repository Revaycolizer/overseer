use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::unix_command::run_command_simple;

pub async fn windows_process_list_json(tool_name: &str) -> Result<String, DynamicToolError> {
    run_command_simple("ps -eo pid,comm,%cpu,rss --sort=-%cpu", tool_name).await
}
