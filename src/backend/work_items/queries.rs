use crate::db::Database;
use super::types::{WorkItem, WorkItemEvent};

pub async fn get_work_items_for_conversation(
    db: &Database,
    conversation_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<WorkItem>> {
    let rows = sqlx::query_as::<_, WorkItem>(
        "SELECT
            id,
            conversation_id,
            title,
            brief,
            status,
            surface_kind,
            assigned_domain,
            prompt,
            source_artifact_ids_json,
            metadata_json,
            created_at,
            updated_at
         FROM work_items
         WHERE conversation_id = ?
         ORDER BY updated_at DESC, created_at DESC
         LIMIT ?",
    )
    .bind(conversation_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_work_item_events(
    db: &Database,
    work_item_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<WorkItemEvent>> {
    let rows = sqlx::query_as::<_, WorkItemEvent>(
        "SELECT
            id,
            work_item_id,
            execution_run_id,
            event_type,
            status,
            summary,
            metadata_json,
            created_at
         FROM work_item_events
         WHERE work_item_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(work_item_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}
