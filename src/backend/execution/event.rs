use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ExecutionEvent {
    pub id: i64,
    pub run_id: i64,
    pub sequence: i64,
    pub event_type: String,
    pub status: String,
    pub domain: Option<String>,
    pub step_text: Option<String>,
    pub tool_name: Option<String>,
    pub model: Option<String>,
    pub message: Option<String>,
    pub details: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn log_execution_event(
    db: &Database,
    run_id: i64,
    sequence: i64,
    event_type: &str,
    status: &str,
    domain: Option<&str>,
    step_text: Option<&str>,
    tool_name: Option<&str>,
    model: Option<&str>,
    message: Option<&str>,
    details: Option<&str>,
) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO execution_events (id, run_id, sequence, event_type, status, domain, step_text, tool_name, model, message, details)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(run_id)
    .bind(sequence)
    .bind(event_type)
    .bind(status)
    .bind(domain)
    .bind(step_text)
    .bind(tool_name)
    .bind(model)
    .bind(message)
    .bind(details)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn get_execution_events(
    db: &Database,
    run_id: i64,
) -> anyhow::Result<Vec<ExecutionEvent>> {
    let rows = sqlx::query_as::<_, ExecutionEvent>(
        "SELECT id, run_id, sequence, event_type, status, domain, step_text, tool_name, model, message, details, created_at
         FROM execution_events
         WHERE run_id = ?
         ORDER BY sequence ASC, created_at ASC"
    )
    .bind(run_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}
