use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ExecutionRun {
    pub id: i64,
    pub conversation_id: Option<i64>,
    pub user_query: String,
    pub status: String,
    pub final_response: Option<String>,
    pub error_message: Option<String>,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: Option<chrono::NaiveDateTime>,
}

pub async fn get_execution_runs_for_conversation(
    db: &Database,
    conversation_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<ExecutionRun>> {
    let rows = sqlx::query_as::<_, ExecutionRun>(
        "SELECT id, conversation_id, user_query, status, final_response, error_message, started_at, finished_at
         FROM execution_runs
         WHERE conversation_id = ?
         ORDER BY started_at DESC
         LIMIT ?"
    )
    .bind(conversation_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_execution_runs_for_workflow_run(
    db: &Database,
    workflow_run_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<ExecutionRun>> {
    let rows = sqlx::query_as::<_, ExecutionRun>(
        "SELECT er.id, er.conversation_id, er.user_query, er.status, er.final_response, er.error_message, er.started_at, er.finished_at
         FROM execution_runs er
         INNER JOIN workflow_runs wr ON wr.conversation_id = er.conversation_id
         WHERE wr.id = ?
         ORDER BY er.started_at DESC
         LIMIT ?"
    )
    .bind(workflow_run_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_latest_execution_run_for_conversation(
    db: &Database,
    conversation_id: i64,
) -> anyhow::Result<Option<ExecutionRun>> {
    let row = sqlx::query_as::<_, ExecutionRun>(
        "SELECT id, conversation_id, user_query, status, final_response, error_message, started_at, finished_at
         FROM execution_runs
         WHERE conversation_id = ?
         ORDER BY started_at DESC
         LIMIT 1"
    )
    .bind(conversation_id)
    .fetch_optional(&db.pool)
    .await?;
    Ok(row)
}

pub async fn get_latest_execution_run(db: &Database) -> anyhow::Result<Option<ExecutionRun>> {
    let row = sqlx::query_as::<_, ExecutionRun>(
        "SELECT id, conversation_id, user_query, status, final_response, error_message, started_at, finished_at
         FROM execution_runs
         ORDER BY started_at DESC
         LIMIT 1"
    )
    .fetch_optional(&db.pool)
    .await?;
    Ok(row)
}
