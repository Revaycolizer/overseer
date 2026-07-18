use crate::db::Database;

pub async fn log_work_item_event(
    db: &Database,
    work_item_id: i64,
    execution_run_id: Option<i64>,
    event_type: &str,
    status: Option<&str>,
    summary: Option<&str>,
    metadata_json: Option<&str>,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO work_item_events (
            id,
            work_item_id,
            execution_run_id,
            event_type,
            status,
            summary,
            metadata_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(work_item_id)
    .bind(execution_run_id)
    .bind(event_type)
    .bind(status)
    .bind(summary)
    .bind(metadata_json)
    .execute(&db.pool)
    .await?;
    Ok(id)
}
