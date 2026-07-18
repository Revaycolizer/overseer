use crate::db::Database;
use crate::engine::utils::discovery::ToolInfo;

pub async fn get_supervisor_tools(
    db: &Database,
    supervisor_id: i64,
) -> anyhow::Result<Vec<ToolInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT DISTINCT t.id, t.name, t.description, CAST(t.code AS TEXT) as code, t.language, t.script_path, t.required_intent, t.safety_level, t.version 
         FROM tools t
         JOIN agent_tools at ON t.id = at.tool_id
         JOIN supervisor_agents sa ON sa.agent_id = at.agent_id
         WHERE sa.supervisor_id = ?"
    )
    .bind(supervisor_id)
    .fetch_all(&db.pool)
    .await?;

    let mut tools = Vec::new();
    for r in rows {
        tools.push(ToolInfo {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get::<Option<String>, _>("description").unwrap_or_default(),
            code: r.get::<Option<String>, _>("code").unwrap_or_default(),
            language: r.get::<Option<String>, _>("language").unwrap_or_default(),
            script_path: r.get("script_path"),
            required_intent: r.get("required_intent"),
            safety_level: r.get::<Option<String>, _>("safety_level").unwrap_or_else(|| "standard".to_string()),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
        });
    }

    Ok(tools)
}
