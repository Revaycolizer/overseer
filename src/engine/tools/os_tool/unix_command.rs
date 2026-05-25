use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub async fn run_command_simple(cmd: &str, tool_name: &str) -> Result<String, DynamicToolError> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(String::new());
    }

    let output = Command::new(parts[0])
        .args(&parts[1..])
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: e.to_string(),
        })?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
