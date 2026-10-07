pub mod json_repair;
pub mod mixed_content;
pub mod result_extractor;
pub mod system;
pub mod uia;
pub mod uia_blocked;

use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use serde_json::Value;
pub use system::*;
pub use uia::*;
pub use uia_blocked::*;

pub fn parse_supervisor_result(
    raw: String,
    hide_evidence: bool,
    report_style: Option<String>,
) -> SupervisorExecutionResult {
    let parsed = mixed_content::parse_mixed_content(&raw);
    let summary = parsed
        .summary
        .replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string();

    let plan_re = regex::Regex::new(r"(?i)\b(I will|I plan to|I need to|Next steps:|I'll perform|I am going to|I should|we could|will be searching|will try to)\b").unwrap();
    let contains_plan = plan_re.is_match(&summary);

    if parsed.json_found || !summary.is_empty() {
        let mut res = result_extractor::extract_result_from_value(
            Value::Object(parsed.merged_value),
            summary,
            raw.clone(),
            hide_evidence,
            report_style,
        );

        let mentions_gen = res.summary.to_lowercase().contains("generated")
            || res.summary.to_lowercase().contains("created")
            || res.summary.to_lowercase().contains(".pdf")
            || res.summary.to_lowercase().contains("document");
        let is_phantom = mentions_gen
            && res.artifacts.is_empty()
            && (res.summary.to_lowercase().contains(".pdf")
                || res.summary.to_lowercase().contains("document"));

        if (contains_plan || is_phantom) && res.status == "completed" {
            res.status = "stalled".to_string();
        }
        return res;
    }

    let is_stalled = contains_plan
        || raw.contains("\"status\": \"stalled\"")
        || raw.contains("\"status\":\"stalled\"");
    let final_status = if is_stalled { "stalled" } else { "completed" };
    let final_summary = if !summary.is_empty() {
        summary
    } else if is_stalled {
        "Worker process is stalled (planning without action).".to_string()
    } else {
        "Task completed.".to_string()
    };

    SupervisorExecutionResult {
        status: final_status.to_string(),
        summary: final_summary,
        messages: vec![],
        evidence: vec![],
        artifacts: vec![],
        charts: vec![],
        verified: is_stalled,
        hide_evidence: true,
        report_style,
        error: None,
        raw_response: raw,
        findings: std::collections::HashMap::new(),
    }
}

pub fn user_facing_runtime_error(details: &str) -> Option<String> {
    let lower = details.to_lowercase();
    let ollama_related = lower.contains("localhost:11434")
        || lower.contains("/api/chat")
        || lower.contains("ollama");
    let connection_related = lower.contains("error sending request")
        || lower.contains("connection refused")
        || lower.contains("failed to connect");

    if ollama_related || connection_related {
        return Some("Model backend unavailable. Overseer could not reach Ollama on `http://localhost:11434`.".to_string());
    }
    None
}
