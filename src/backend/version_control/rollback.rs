use crate::db::Database;
use crate::engine::utils::discovery::AgentInfo;

pub async fn rollback_version(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
    version_tag: &str,
) -> anyhow::Result<()> {
    use sqlx::Row;
    let v = sqlx::query(
        "SELECT content FROM versions WHERE entity_type = ? AND entity_id = ? AND version_tag = ? LIMIT 1"
    )
    .bind(entity_type).bind(entity_id).bind(version_tag)
    .fetch_one(&db.pool)
    .await?;

    let content: String = v.get("content");

    if entity_type == "agent" {
        let agent: AgentInfo = serde_json::from_str(&content)?;
        sqlx::query(
            "UPDATE agents
             SET version = ?, name = ?, description = ?, instructions = ?, model = ?, domain = ?, code = ?, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?"
        )
        .bind(version_tag)
        .bind(agent.name)
        .bind(agent.description)
        .bind(agent.instructions)
        .bind(agent.model)
        .bind(agent.domain)
        .bind(agent.code)
        .bind(entity_id)
        .execute(&db.pool)
        .await?;
    } else if entity_type == "tool" {
        sqlx::query("UPDATE tools SET version = ?, code = ? WHERE id = ?")
            .bind(version_tag)
            .bind(content)
            .bind(entity_id)
            .execute(&db.pool)
            .await?;
    }

    Ok(())
}
