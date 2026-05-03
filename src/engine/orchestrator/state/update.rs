use super::{StructuredState, EvidenceChunk};
use crate::db::Database;
use std::collections::HashMap;
use serde_json::Value;

pub async fn preview_state_update(
    state: &StructuredState,
    db: &Database,
    findings: &HashMap<String, Value>,
    summary: &str,
    evidence_raw: &[String]
) -> StructuredState {
    let mut cloned = state.clone();
    update_state_from_findings(&mut cloned, db, findings, summary, evidence_raw).await;
    cloned
}

pub async fn update_state_from_findings(
    state: &mut StructuredState,
    db: &Database,
    findings: &HashMap<String, Value>,
    summary: &str,
    evidence_raw: &[String]
) {
    for e in evidence_raw {
        let mut text = e.clone();
        let mut source = "collected_data".to_string();

        if let Ok(v) = serde_json::from_str::<Value>(e) {
            if let Some(s) = v.get("snippet").or(v.get("text")).and_then(|v| v.as_str()) {
                text = s.to_string();
            }
            if let Some(src) = v.get("url").or(v.get("source")).and_then(|v| v.as_str()) {
                source = src.to_string();
            }
        }

        if !state.evidence.iter().any(|c| c.text == text && c.source == source) {
            let id = format!("E{}", state.next_id);
            let snowflake_id = db.next_id().await;
            state.evidence.push(EvidenceChunk { id, snowflake_id, text, source });
            state.next_id += 1;
        }
    }

    process_insights(state, findings, summary);
}

fn process_insights(state: &mut StructuredState, findings: &HashMap<String, Value>, summary: &str) {
    for (key, val) in findings {
        if let Some(s) = val.as_str() {
            let clean_s = crate::engine::orchestrator::utils::format_helpers::strip_grounding_markers(s);
            let insight = format!("{}: {}", key, clean_s);
            if !state.derived_insights.contains(&insight) { state.derived_insights.push(insight); }
        }
    }
    if !summary.is_empty() && summary.len() > 20 {
        let clean = crate::engine::orchestrator::utils::format_helpers::strip_grounding_markers(summary);
        let snippet = if clean.len() > 100 { format!("{}...", &clean[..100]) } else { clean };
        let entry = format!("Context: {}", snippet);
        if !state.derived_insights.contains(&entry) { state.derived_insights.push(entry); }
    }
}
