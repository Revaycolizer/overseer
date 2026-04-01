use crate::db::Database;
use serde_json::json;
use crate::engine::orchestrator::utils::artifact_utils::*;

pub async fn record_event(
    db: &Database,
    execution_run_id: Option<i64>,
    sequence: &mut i64,
    event_type: &str,
    status: &str,
    domain: Option<&str>,
    step_text: Option<&str>,
    tool_name: Option<&str>,
    model_name: Option<&str>,
    message: Option<&str>,
    details: Option<&str>,
) {
    if let Some(run_id) = execution_run_id {
        let _ = crate::backend::log_execution_event(
            db,
            run_id,
            *sequence,
            event_type,
            status,
            domain,
            step_text,
            tool_name,
            model_name,
            message,
            details,
        ).await;
        *sequence += 1;
    }
}

pub async fn persist_step_artifacts(
    db: &Database,
    conversation_id: i64,
    execution_run_id: Option<i64>,
    event_sequence: &mut i64,
    domain: &str,
    step: &str,
    summary: &str,
    artifacts: &[String],
) {
    let mut seen = std::collections::HashSet::new();
    let mut recorded = Vec::new();

    for artifact in artifacts {
        let trimmed = artifact.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) { continue; }

        let (kind, content_type) = infer_artifact_kind_and_content_type(trimmed);
        let label = infer_artifact_label(&kind, step, trimmed);
        let is_file_like = matches!(kind.as_str(), "image" | "file");
        let file_path = is_file_like.then_some(trimmed);
        let content = if is_file_like { Some(summary) } else { Some(trimmed) };
        let metadata = json!({ "source": "supervisor_result", "domain": domain, "step_text": step, "raw_value": trimmed }).to_string();

        if let Ok(_) = crate::backend::create_artifact(db, conversation_id, execution_run_id, Some(domain), Some(step), &kind, &label, content, file_path, content_type.as_deref(), Some(&metadata)).await {
            recorded.push(json!({ "kind": kind, "label": label, "value": trimmed }));
        }
    }

    if !recorded.is_empty() {
        let details = json!({ "count": recorded.len(), "artifacts": recorded }).to_string();
        record_event(db, execution_run_id, event_sequence, "artifacts_recorded", "completed", Some(domain), Some(step), None, None, Some(&format!("Recorded {} artifacts for this step.", recorded.len())), Some(&details)).await;
    }
}
