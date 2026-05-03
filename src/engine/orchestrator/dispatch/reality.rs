use std::collections::HashSet;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;

pub fn enforce_physical_reality(
    res: &mut SupervisorExecutionResult,
    _visited_urls: &HashSet<String>,
    last_artifacts: &[String],
    action_tool_executed: bool,
    domain: &str,
) {
    // Ground truth: real artifacts come from actual tool executions, not LLM text.
    // Always replace LLM-reported artifact paths with the verified ones.
    res.artifacts.clear();
    for art in last_artifacts {
        res.artifacts.push(art.clone());
    }

    let is_action_domain = ["stealth", "web", "office", "computer", "desktop", "code_executor"]
        .contains(&domain);

    // Only fire if the LLM claims "completed" on an action domain without ever
    // calling a tool AND without producing any real artifacts.
    let no_tool_no_artifact = !action_tool_executed && last_artifacts.is_empty();

    if res.status == "completed" && is_action_domain && no_tool_no_artifact {
        println!(
            "⚠️ PHYSICAL REALITY FAILURE in domain '{}': Task completed without physical tool execution. Forcing status to 'stalled'.",
            domain
        );
        res.status = "stalled".to_string();
    }
}

