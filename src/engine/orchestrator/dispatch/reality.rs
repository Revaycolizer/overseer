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

    // ARCHITECTURE V3.6: GROUNDING RESILIENCE
    // Only fire if the LLM claims "completed" on an action domain without ever
    // calling a tool AND without producing any real artifacts AND without 
    // any existing grounding evidence in the context.
    let has_grounding = res.evidence.iter().any(|e| e.contains("Evidence") || e.contains("Vision Insights"));
    let no_tool_no_artifact = !action_tool_executed && last_artifacts.is_empty() && !has_grounding;

    if res.status == "completed" && is_action_domain && no_tool_no_artifact {
        tracing::warn!(?domain, "Physical reality failure — no tool execution, forcing stalled");
        res.status = "stalled".to_string();
    }
}

