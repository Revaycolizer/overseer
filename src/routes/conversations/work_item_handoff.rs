// src/routes/conversations/work_item_handoff.rs
use super::work_item_ops::*;
use crate::backend;
use dioxus::prelude::*;

pub fn create_work_item_handoff(
    db: std::sync::Arc<crate::db::Database>,
    work_item: backend::WorkItem,
    mut selected_work_item: Signal<Option<backend::WorkItem>>,
    mut work_items_resource: Resource<Vec<backend::WorkItem>>,
    mut work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
) {
    spawn(async move {
        let runtime_metadata = parse_work_item_runtime_metadata(&work_item);
        let title = format!("Handoff: {}", work_item.title);
        let prompt = build_work_item_handoff_prompt(&work_item, &runtime_metadata);
        let source_artifact_ids_json =
            serde_json::to_string(&merged_work_item_artifact_ids(&work_item)).ok();
        if let Ok(id) = backend::create_work_item(
            &db, work_item.conversation_id, &title,
            Some("Created as a handoff from an existing work item."),
            "queued", "handoff", work_item.assigned_domain.as_deref(),
            &prompt, source_artifact_ids_json.as_deref(), None,
        ).await {
            let now = chrono::Utc::now().naive_utc();
            let new_item = backend::WorkItem {
                id, conversation_id: work_item.conversation_id, title,
                brief: Some("Created as a handoff from an existing work item.".to_string()),
                status: "queued".to_string(), surface_kind: "handoff".to_string(),
                assigned_domain: work_item.assigned_domain.clone(),
                prompt: prompt.clone(), source_artifact_ids_json,
                metadata_json: None, created_at: now, updated_at: now,
            };
            let _ = backend::log_work_item_event(
                &db, work_item.id, None, "handoff_created", Some("queued"),
                Some("Created a follow-up handoff work item."), None,
            ).await;
            let _ = backend::log_work_item_event(
                &db, id, None, "created", Some("queued"),
                Some("This work item was created as a handoff."), None,
            ).await;
            selected_work_item.set(Some(new_item));
            work_items_resource.restart();
            work_item_events_resource.restart();
        }
    });
}
