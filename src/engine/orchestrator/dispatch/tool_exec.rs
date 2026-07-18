use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::state::StructuredState;
use crate::engine::utils::discovery::ToolInfo;
use overseer_kernel::ExecutionOutcome;
use serde_json::Value;

use super::grounding::expand_markers;

/// Execute a tool the agent asked for — but only ever THROUGH the Overseer
/// Kernel. Policy is evaluated at this single chokepoint: capability membership,
/// allowlist, denials, action cap, timeout and emergency-stop are enforced, and
/// computer/shell actions in approval mode pause for a real human decision. The
/// tool is never called directly from here.
#[allow(clippy::too_many_arguments)]
pub async fn execute_tool(
    db: &Database,
    run_id: Option<i64>,
    domain: &str,
    step: &str,
    tname: &str,
    input: Value,
    tools: &[ToolInfo],
    state: &StructuredState,
) -> Result<(String, bool, Value), SupervisorExecutionResult> {
    let tool = match tools.iter().find(|t| t.name == tname) {
        Some(t) => t,
        None => return Err(missing_tool_error(tname, tools)),
    };

    // --- ARCHITECTURE V3.3: AUTOMATIC GROUNDING EXPANSION ---
    let expanded_input = expand_markers(input, state);
    println!("🛡️ KERNEL AUTHORIZING TOOL: {}", tname);

    // The agent's capability set for this domain is exactly the tool list it was
    // given. All authorization runs inside the kernel.
    let capabilities: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

    let outcome = crate::engine::kernel::kernel_execute(
        db,
        run_id,
        domain,
        step,
        tool,
        expanded_input.clone(),
        &capabilities,
    )
    .await;

    let is_action = [
        "stealth_search",
        "web_search",
        "document_writer",
        "charting",
        "os_tool",
        "code_executor",
        "stealth_browse",
        "web_browse",
        "http_request",
        "web_extract",
    ]
    .iter()
    .any(|&t| t == tname);

    match outcome {
        ExecutionOutcome::Executed { output } => Ok((output, is_action, expanded_input)),
        // Denied / rejected / cancelled / timed out / emergency-stopped: the tool
        // did NOT run. Return a structured decision so the agent can react, and
        // mark it as a non-action so downstream logic does not treat it as work.
        other => Ok((other.agent_response(), false, expanded_input)),
    }
}

fn missing_tool_error(tname: &str, tools: &[ToolInfo]) -> SupervisorExecutionResult {
    let tool_list = tools
        .iter()
        .map(|t| t.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    SupervisorExecutionResult::stalled(
        format!(
            "REJECTED | REASON: Tool '{}' not in capability set. Available: [{}]",
            tname, tool_list
        ),
        format!("Tool {} missing", tname),
    )
}
