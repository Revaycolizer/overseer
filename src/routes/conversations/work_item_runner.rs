// src/routes/conversations/work_item_runner.rs
use super::helpers::build_query_with_context;
use super::work_item_ops::*;
use super::work_item_runner_finalize::{finalize_work_item_error, finalize_work_item_run};
use crate::backend;
use dioxus::prelude::*;

pub fn update_work_item_manual_status(
    db: std::sync::Arc<crate::db::Database>,
    work_item: backend::WorkItem,
    status: String,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    mut work_items_resource: Resource<Vec<backend::WorkItem>>,
    mut work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
) {
    spawn(async move {
        let now = chrono::Utc::now().naive_utc().to_string();
        let note = match status.as_str() {
            "blocked" => "Marked blocked manually from the workspace.",
            "done" => "Marked done manually from the workspace.",
            "queued" => "Queued again from the workspace.",
            "in_progress" => "Marked in progress manually from the workspace.",
            _ => "Status updated manually from the workspace.",
        };
        let metadata_json = build_work_item_runtime_metadata(
            None, Some(status.clone()), vec![], note.to_string(), now,
        );
        let _ = backend::update_work_item_runtime(&db, work_item.id, &status, Some(&metadata_json)).await;
        let _ = backend::log_work_item_event(
            &db, work_item.id, None, "status_changed", Some(&status), Some(note), Some(&metadata_json),
        ).await;
        set_selected_work_item_runtime(selected_work_item, work_item.id, &status, Some(metadata_json));
        work_items_resource.restart();
        work_item_events_resource.restart();
    });
}

pub fn run_work_item_now(
    db: std::sync::Arc<crate::db::Database>,
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    work_item: backend::WorkItem,
    mut messages_resource: Resource<Vec<backend::Message>>,
    mut visible_messages: Signal<Vec<backend::Message>>,
    mut selected_work_item: Signal<Option<backend::WorkItem>>,
    mut is_thinking: Signal<bool>,
    mut work_items_resource: Resource<Vec<backend::WorkItem>>,
    mut execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    mut artifacts_resource: Resource<Vec<backend::Artifact>>,
    mut work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    mut selected_execution_run_id: Signal<Option<i64>>,
) {
    let optimistic_id = -chrono::Utc::now().timestamp_micros();
    visible_messages.with_mut(|messages| {
        messages.push(backend::Message {
            id: optimistic_id,
            conversation_id: work_item.conversation_id,
            role: "user".to_string(),
            content: work_item.prompt.clone(),
            created_at: chrono::Utc::now().naive_utc(),
            attachments: vec![],
            hide_evidence: false,
        });
    });

    spawn(async move {
        selected_work_item.set(Some(work_item.clone()));
        let in_progress_metadata = build_work_item_runtime_metadata(
            None, Some("running".to_string()), vec![],
            "Work item execution started.".to_string(),
            chrono::Utc::now().naive_utc().to_string(),
        );
        let _ = backend::update_work_item_runtime(&db, work_item.id, "in_progress", Some(&in_progress_metadata)).await;
        set_selected_work_item_runtime(selected_work_item, work_item.id, "in_progress", Some(in_progress_metadata));
        let _ = backend::log_work_item_event(&db, work_item.id, None, "run_started", Some("in_progress"), Some("Work item execution started."), None).await;
        work_items_resource.restart();
        work_item_events_resource.restart();

        let source_artifact_ids = merged_work_item_artifact_ids(&work_item);
        let source_artifacts = backend::get_artifacts_by_ids(&db, &source_artifact_ids).await.unwrap_or_default();
        let query = build_query_with_context(&work_item.prompt, &source_artifacts, Some(&work_item));
        let add_user = backend::add_message(&db, work_item.conversation_id, "user", &query, false).await;

        if let Err(err) = add_user {
            finalize_work_item_error(&db, &work_item, optimistic_id, &err, visible_messages, selected_work_item, work_items_resource, work_item_events_resource).await;
            return;
        }

        messages_resource.restart();
        is_thinking.set(true);
        let response = agent_system.orchestrate(&db, work_item.conversation_id, &query).await;
        is_thinking.set(false);

        let _ = backend::add_message(&db, work_item.conversation_id, "assistant", &response, false).await;
        messages_resource.restart();

        finalize_work_item_run(&db, &work_item, &response, selected_work_item, work_items_resource, execution_runs_resource, artifacts_resource, work_item_events_resource, selected_execution_run_id).await;
    });
}
