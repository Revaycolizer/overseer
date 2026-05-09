// src/routes/conversations/work_item_ops.rs
use crate::backend;
use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WorkItemRuntimeMetadata {
    pub latest_run_id: Option<i64>,
    pub latest_run_status: Option<String>,
    pub latest_artifact_ids: Option<Vec<i64>>,
    pub latest_response_excerpt: Option<String>,
    pub last_executed_at: Option<String>,
}

pub fn parse_work_item_runtime_metadata(work_item: &backend::WorkItem) -> WorkItemRuntimeMetadata {
    work_item
        .metadata_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<WorkItemRuntimeMetadata>(json).ok())
        .unwrap_or_default()
}

pub fn set_selected_work_item_runtime(
    mut selected_work_item: Signal<Option<backend::WorkItem>>,
    work_item_id: i64,
    status: &str,
    metadata_json: Option<String>,
) {
    if let Some(mut current) = selected_work_item() {
        if current.id == work_item_id {
            current.status = status.to_string();
            current.metadata_json = metadata_json;
            selected_work_item.set(Some(current));
        }
    }
}

pub fn build_work_item_runtime_metadata(
    latest_run_id: Option<i64>,
    latest_run_status: Option<String>,
    latest_artifact_ids: Vec<i64>,
    latest_response_excerpt: String,
    last_executed_at: String,
) -> String {
    serde_json::json!({
        "latest_run_id": latest_run_id,
        "latest_run_status": latest_run_status,
        "latest_artifact_ids": latest_artifact_ids,
        "latest_response_excerpt": latest_response_excerpt.chars().take(220).collect::<String>(),
        "last_executed_at": last_executed_at,
    })
    .to_string()
}

pub fn merged_work_item_artifact_ids(work_item: &backend::WorkItem) -> Vec<i64> {
    let mut artifact_ids = work_item
        .source_artifact_ids_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Vec<i64>>(json).ok())
        .unwrap_or_default();

    if let Some(latest_artifacts) = parse_work_item_runtime_metadata(work_item).latest_artifact_ids
    {
        for artifact_id in latest_artifacts {
            if !artifact_ids.contains(&artifact_id) {
                artifact_ids.push(artifact_id);
            }
        }
    }

    artifact_ids
}

pub fn build_work_item_handoff_prompt(
    work_item: &backend::WorkItem,
    runtime_metadata: &WorkItemRuntimeMetadata,
) -> String {
    let latest_excerpt = runtime_metadata
        .latest_response_excerpt
        .clone()
        .unwrap_or_else(|| "No latest result summary captured yet.".to_string());
    format!(
        "Continue this work item as a handoff.\n\nOriginal title: {}\nOriginal prompt: {}\nCurrent status: {}\nLatest result: {}\n\nTake over from here, use the attached work-item context and artifacts, and propose or execute the next best step.",
        work_item.title,
        work_item.prompt,
        work_item.status,
        latest_excerpt
    )
}
