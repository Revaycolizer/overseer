use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::state::StructuredState;
use crate::engine::utils::discovery::ToolInfo;
use super::grounding::expand_markers;

pub async fn execute_tool(
    tname: &str,
    input: Value,
    tools: &[ToolInfo],
    state: &StructuredState,
    token_sender: Option<&UnboundedSender<String>>,
) -> Result<(String, bool, Value), SupervisorExecutionResult> {
    let tool = match tools.iter().find(|t| t.name == tname) {
        Some(t) => t,
        None => return Err(missing_tool_error(tname, tools)),
    };

    // --- ARCHITECTURE V3.3: AUTOMATIC GROUNDING EXPANSION ---
    let expanded_input = expand_markers(input, state);
    tracing::debug!(?tname, "Executing tool");

    let output = match crate::engine::tools::call_dynamic_tool(tool, expanded_input.clone()).await {
        Ok(o) => {
            crate::engine::streaming::stream_tool_output(tname, &o, token_sender);
            o
        }
        Err(e) => return Ok((format!("ERROR: {}", e), false, expanded_input)),
    };

    let is_action = ["stealth_search", "web_search", "document_writer", "charting", "os_tool", "code_executor", "stealth_browse", "web_browse", "http_request", "web_extract"].iter().any(|&t| t == tname);
    
    Ok((output, is_action, expanded_input))
}

fn missing_tool_error(tname: &str, tools: &[ToolInfo]) -> SupervisorExecutionResult {
    let tool_list = tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ");
    SupervisorExecutionResult::stalled(
        format!("REJECTED | REASON: Tool '{}' not in capability set. Available: [{}]", tname, tool_list),
        format!("Tool {} missing", tname)
    )
}
