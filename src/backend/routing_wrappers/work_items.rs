use crate::db::Database;
use crate::backend::{WorkItem, WorkItemEvent};
use crate::backend::client::WorkItemClient;

pub async fn create_work_item(db: &Database, conversation_id: i64, title: &str, brief: Option<&str>, status: &str, surface_kind: &str, assigned_domain: Option<&str>, prompt: &str, source_artifact_ids_json: Option<&str>, metadata_json: Option<&str>) -> anyhow::Result<i64> {
    if db.is_remote() { return db.remote_client().create_work_item(conversation_id, title, brief, status, surface_kind, assigned_domain, prompt, source_artifact_ids_json, metadata_json).await; }
    crate::backend::work_items::create_work_item(db, conversation_id, title, brief, status, surface_kind, assigned_domain, prompt, source_artifact_ids_json, metadata_json).await
}

pub async fn get_work_items_for_conversation(db: &Database, conversation_id: i64, limit: i64) -> anyhow::Result<Vec<WorkItem>> {
    if db.is_remote() { return db.remote_client().get_work_items_for_conversation(conversation_id, limit).await; }
    crate::backend::work_items::get_work_items_for_conversation(db, conversation_id, limit).await
}

pub async fn update_work_item_status(db: &Database, id: i64, status: &str) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().update_work_item_status(id, status).await; }
    crate::backend::work_items::update_work_item_status(db, id, status).await
}

pub async fn update_work_item_runtime(db: &Database, id: i64, status: &str, metadata_json: Option<&str>) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().update_work_item_runtime(id, status, metadata_json).await; }
    crate::backend::work_items::update_work_item_runtime(db, id, status, metadata_json).await
}

pub async fn log_work_item_event(db: &Database, work_item_id: i64, execution_run_id: Option<i64>, event_type: &str, status: Option<&str>, summary: Option<&str>, metadata_json: Option<&str>) -> anyhow::Result<i64> {
    if db.is_remote() { return db.remote_client().log_work_item_event(work_item_id, execution_run_id, event_type, status, summary, metadata_json).await; }
    crate::backend::work_items::log_work_item_event(db, work_item_id, execution_run_id, event_type, status, summary, metadata_json).await
}

pub async fn get_work_item_events(db: &Database, work_item_id: i64, limit: i64) -> anyhow::Result<Vec<WorkItemEvent>> {
    if db.is_remote() { return db.remote_client().get_work_item_events(work_item_id, limit).await; }
    crate::backend::work_items::get_work_item_events(db, work_item_id, limit).await
}
