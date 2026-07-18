use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkItem {
    pub id: i64,
    pub conversation_id: i64,
    pub title: String,
    pub brief: Option<String>,
    pub status: String,
    pub surface_kind: String,
    pub assigned_domain: Option<String>,
    pub prompt: String,
    pub source_artifact_ids_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkItemEvent {
    pub id: i64,
    pub work_item_id: i64,
    pub execution_run_id: Option<i64>,
    pub event_type: String,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}
