use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Artifact {
    pub id: i64,
    pub conversation_id: i64,
    pub execution_run_id: Option<i64>,
    pub domain: Option<String>,
    pub step_text: Option<String>,
    pub kind: String,
    pub label: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub content_type: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn get_artifacts_for_conversation(
    db: &Database,
    conversation_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<Artifact>> {
    let rows = sqlx::query_as::<_, Artifact>(
        "SELECT
            id,
            conversation_id,
            execution_run_id,
            domain,
            step_text,
            kind,
            label,
            content,
            file_path,
            content_type,
            metadata_json,
            created_at
         FROM artifacts
         WHERE conversation_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(conversation_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_artifacts_for_execution_run(
    db: &Database,
    execution_run_id: i64,
) -> anyhow::Result<Vec<Artifact>> {
    let rows = sqlx::query_as::<_, Artifact>(
        "SELECT
            id,
            conversation_id,
            execution_run_id,
            domain,
            step_text,
            kind,
            label,
            content,
            file_path,
            content_type,
            metadata_json,
            created_at
         FROM artifacts
         WHERE execution_run_id = ?
         ORDER BY created_at DESC",
    )
    .bind(execution_run_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}
