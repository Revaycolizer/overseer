use crate::engine::orchestrator::planning::planner::MacroTask;

pub fn build_native_workflow_plan(query: &str) -> String {
    let lower = query.to_lowercase();
    let traceability = true;
    let mut response = String::new();

    response.push_str("1. Confirm the request context and relevant artifacts.\n");
    if lower.contains("check") || lower.contains("verify") || lower.contains("validate") {
        response.push_str("2. Perform the specified validation check using available tools.\n");
        response.push_str("3. Capture the evidence of the check as an artifact.\n");
        response.push_str("4. Log the result with a verified status.\n");
        response.push_str(
            "5. If the check fails or content changes unexpectedly, create a blocked work item and alert the operator.\n",
        );
    } else {
        response
            .push_str("2. Execute the first task step and capture the output as an artifact.\n");
        response.push_str("3. Validate the result before continuing to the next step.\n");
        response.push_str("4. Hand off the artifact or create a work item for the next actor.\n");
        response.push_str("5. Mark the run done or blocked with a clear reason.\n");
    }

    if traceability {
        response.push_str("\nTraceability layer:\n");
        response
            .push_str("- Record each run with a start time, finish time, status, and summary.\n");
        response.push_str("- Attach evidence artifacts for every important step.\n");
        response.push_str("- Keep failure reasons explicit so retries and handoffs are easy.\n");
    }
    response
}

pub fn sanitize_plan(plan: &str, query: &str) -> Vec<MacroTask> {
    if let Ok(tasks) = serde_json::from_str::<Vec<MacroTask>>(plan) {
        if !tasks.is_empty() {
            return tasks;
        }
    }

    // Fallback if parsing fails
    vec![MacroTask {
        domain: "chat".to_string(),
        task: query.to_string(),
    }]
}
