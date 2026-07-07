use crate::db::Database;
use crate::engine::utils::discovery::AgentInfo;

pub async fn get_supervisor_agents(
    db: &Database,
    supervisor_id: i64,
) -> anyhow::Result<Vec<AgentInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT a.id, a.name, a.description, a.instructions, a.model, a.domain, a.version, CAST(a.code AS TEXT) as code, a.response_format 
         FROM agents a
         JOIN supervisor_agents sa ON a.id = sa.agent_id
         WHERE sa.supervisor_id = ?"
    )
    .bind(supervisor_id)
    .fetch_all(&db.pool)
    .await?;

    let mut agents = Vec::new();
    for r in rows {
        let agent_id: i64 = r.get("id");

        let domain: String = r.get::<Option<String>, _>("domain").unwrap_or_default();

        agents.push(AgentInfo {
            id: agent_id,
            name: r.get("name"),
            description: r.get::<Option<String>, _>("description").unwrap_or_default(),
            instructions: r.get::<Option<String>, _>("instructions").unwrap_or_default(),
            model: r.get::<Option<String>, _>("model").unwrap_or_default(),
            domain: domain.clone(),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
            code: r.get("code"),
            tools: vec![], // Empty placeholders as per original
            source_files: vec![],
            response_format: r.get::<Option<String>, _>("response_format").unwrap_or_else(|| "json".to_string()),
        });
    }

    Ok(agents)
}

pub async fn link_agent_to_supervisor(
    db: &Database,
    supervisor_id: i64,
    agent_id: i64,
) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(supervisor_id)
    .bind(agent_id)
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(())
}

pub async fn unlink_agent_from_supervisor(
    db: &Database,
    supervisor_id: i64,
    agent_id: i64,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM supervisor_agents WHERE supervisor_id = ? AND agent_id = ?")
        .bind(supervisor_id)
        .bind(agent_id)
        .execute(&db.pool)
        .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(())
}
