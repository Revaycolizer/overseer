use crate::db::Database;

pub async fn add_tool_to_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query("INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id) VALUES (?, ?, ?)")
        .bind(id)
        .bind(agent_id)
        .bind(tool_id)
        .execute(&db.pool)
        .await?;
    super::super::refresh_runtime_registry(db).await;
    Ok(())
}

pub async fn remove_tool_from_agent(
    db: &Database,
    agent_id: i64,
    tool_id: i64,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM agent_tools WHERE agent_id = ? AND tool_id = ?")
        .bind(agent_id)
        .bind(tool_id)
        .execute(&db.pool)
        .await?;
    super::super::refresh_runtime_registry(db).await;
    Ok(())
}
