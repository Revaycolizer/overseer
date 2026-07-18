use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EntityVersion {
    pub id: i64,
    pub version_tag: String,
    pub changelog: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn get_version_content(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
    version_tag: &str,
) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT content FROM versions WHERE entity_type = ? AND entity_id = ? AND version_tag = ? LIMIT 1"
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(version_tag)
    .fetch_optional(&db.pool)
    .await?;

    Ok(row.map(|(content,)| content))
}

pub async fn get_version_history(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
) -> anyhow::Result<Vec<EntityVersion>> {
    let rows = sqlx::query_as::<_, EntityVersion>(
        "SELECT id, version_tag, changelog, created_at FROM versions WHERE entity_type = ? AND entity_id = ? ORDER BY created_at DESC"
    )
    .bind(entity_type).bind(entity_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}
