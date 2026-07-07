use crate::db::Database;

pub async fn publish_agent_version(
    db: &Database,
    agent_id: i64,
    version_tag: &str,
    changelog: &str,
) -> anyhow::Result<()> {
    let agent = crate::backend::get_agents(db)
        .await?
        .into_iter()
        .find(|a| a.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    let content = serde_json::to_string(&agent)?;
    publish_version(db, "agent", agent_id, version_tag, &content, changelog).await
}

pub async fn publish_tool_version(
    db: &Database,
    tool_id: i64,
    version_tag: &str,
    changelog: &str,
) -> anyhow::Result<()> {
    let tool = crate::backend::get_tools(db)
        .await?
        .into_iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| anyhow::anyhow!("Tool not found"))?;

    let content = tool.code;
    publish_version(db, "tool", tool_id, version_tag, &content, changelog).await
}

pub async fn publish_version(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
    version_tag: &str,
    content: &str,
    changelog: &str,
) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO versions (id, entity_type, entity_id, version_tag, content, changelog) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(entity_type).bind(entity_id).bind(version_tag).bind(content).bind(changelog)
    .execute(&db.pool)
    .await?;

    if entity_type == "agent" {
        sqlx::query("UPDATE agents SET version = ? WHERE id = ?")
            .bind(version_tag)
            .bind(entity_id)
            .execute(&db.pool)
            .await?;
    } else if entity_type == "tool" {
        sqlx::query("UPDATE tools SET version = ? WHERE id = ?")
            .bind(version_tag)
            .bind(entity_id)
            .execute(&db.pool)
            .await?;
    }

    Ok(())
}
