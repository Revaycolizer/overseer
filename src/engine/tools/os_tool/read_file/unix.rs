use serde_json::json;
use crate::engine::tools::error::DynamicToolError;

pub async fn windows_read_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
    let content =
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: format!("Failed to read file '{}': {}", path, e),
            })?;

    let preview = if content.len() > 2000 {
        format!("{}...", &content[..2000])
    } else {
        content.clone()
    };

    Ok(json!({
        "action": "read_file",
        "path": path,
        "content_length": content.len(),
        "preview": preview,
        "summary": format!("Successfully read file '{}'. Captured {} bytes.", path, content.len())
    })
    .to_string())
}
