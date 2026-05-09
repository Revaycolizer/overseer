// src/routes/conversations/work_item_runner_finalize.rs
use super::work_item_ops::*;
use crate::backend;
use dioxus::prelude::*;

pub async fn finalize_work_item_error(
    db: &std::sync::Arc<crate::db::Database>,
    work_item: &backend::WorkItem,
    optimistic_id: i64,
    err: &anyhow::Error,
    mut visible_messages: Signal<Vec<backend::Message>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    mut work_items_resource: Resource<Vec<backend::WorkItem>>,
    mut work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
) {
    visible_messages.write().retain(|message| message.id != optimistic_id);
    let metadata_json = build_work_item_runtime_metadata(
        None, Some("failed".to_string()), vec![],
        format!("Failed to queue the work item prompt: {err}"),
        chrono::Utc::now().naive_utc().to_string(),
    );
    let _ = backend::update_work_item_runtime(db, work_item.id, "blocked", Some(&metadata_json)).await;
    let _ = backend::log_work_item_event(
        db, work_item.id, None, "run_blocked", Some("blocked"),
        Some(&format!("Failed to queue the work item prompt: {err}")),
        Some(&metadata_json),
    ).await;
    set_selected_work_item_runtime(selected_work_item, work_item.id, "blocked", Some(metadata_json));
    work_items_resource.restart();
    work_item_events_resource.restart();
}

pub async fn finalize_work_item_run(
    db: &std::sync::Arc<crate::db::Database>,
    work_item: &backend::WorkItem,
    response: &str,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    mut work_items_resource: Resource<Vec<backend::WorkItem>>,
    mut execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    mut artifacts_resource: Resource<Vec<backend::Artifact>>,
    mut work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    mut selected_execution_run_id: Signal<Option<i64>>,
) {
    let latest_run = backend::get_latest_execution_run_for_conversation(db, work_item.conversation_id).await.ok().flatten();
    let (status, metadata_json, latest_run_id) = if let Some(run) = latest_run {
        let latest_artifacts = backend::get_artifacts_for_execution_run(db, run.id).await.unwrap_or_default();
        let work_status = if run.status == "completed" { "done" } else { "blocked" };
        let excerpt = run.error_message.clone().filter(|v: &String| !v.trim().is_empty())
            .or_else(|| run.final_response.clone().filter(|v: &String| !v.trim().is_empty()))
            .unwrap_or_else(|| response.to_string());
        let metadata_json = build_work_item_runtime_metadata(
            Some(run.id), Some(run.status.clone()),
            latest_artifacts.iter().map(|a| a.id).collect::<Vec<_>>(), excerpt,
            run.finished_at.map(|f: chrono::NaiveDateTime| f.to_string()).unwrap_or_else(|| run.started_at.to_string()),
        );
        (work_status.to_string(), Some(metadata_json), Some(run.id))
    } else {
        let metadata_json = build_work_item_runtime_metadata(
            None, Some("missing_run".to_string()), vec![],
            response.to_string(), chrono::Utc::now().naive_utc().to_string(),
        );
        ("blocked".to_string(), Some(metadata_json), None)
    };

    let _ = backend::update_work_item_runtime(db, work_item.id, &status, metadata_json.as_deref()).await;
    let _ = backend::log_work_item_event(
        db, work_item.id, latest_run_id,
        if status == "done" { "run_completed" } else { "run_blocked" },
        Some(&status),
        metadata_json.as_deref()
            .and_then(|json| serde_json::from_str::<WorkItemRuntimeMetadata>(json).ok())
            .and_then(|m| m.latest_response_excerpt).as_deref(),
        metadata_json.as_deref(),
    ).await;
    set_selected_work_item_runtime(selected_work_item, work_item.id, &status, metadata_json);
    if let Some(run_id) = latest_run_id { selected_execution_run_id.set(Some(run_id)); }
    work_items_resource.restart();
    execution_runs_resource.restart();
    artifacts_resource.restart();
    work_item_events_resource.restart();
}
