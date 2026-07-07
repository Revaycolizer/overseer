use crate::db::Database;
use crate::engine::utils::discovery::AgentInfo;

pub async fn add_agent(
    db: &Database,
    name: &str,
    description: &str,
    domain: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let model = crate::backend::resolve_model_name(db, None, "default_chat_model", "qwen2.5:7b-instruct").await;
    sqlx::query(
        "INSERT INTO agents (id, name, description, instructions, model, domain, version, code, response_format)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind("")
    .bind(model)
    .bind(domain)
    .bind("1.0.0")
    .bind("")
    .bind("json")
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(id)
}

pub async fn update_agent(db: &Database, agent: AgentInfo) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE agents
         SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ?, response_format = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(agent.name)
    .bind(agent.description)
    .bind(agent.instructions)
    .bind(agent.model)
    .bind(agent.domain)
    .bind(agent.version)
    .bind(agent.code)
    .bind(agent.response_format)
    .bind(agent.id)
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(())
}
