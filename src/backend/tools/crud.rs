use crate::db::Database;
use crate::engine::utils::discovery::ToolInfo;

pub async fn get_tools(db: &Database) -> anyhow::Result<Vec<ToolInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, description, CAST(code AS TEXT) as code, language, script_path, required_intent, safety_level, version FROM tools"
    )
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

pub async fn add_tool(
    db: &Database,
    name: &str,
    description: &str,
    language: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO tools (id, name, description, code, language, script_path, required_intent, safety_level, version)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind("")
    .bind(language)
    .bind("")
    .bind(None::<String>)
    .bind("standard")
    .bind("1.0.0")
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(id)
}

pub async fn update_tool(db: &Database, tool: ToolInfo) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tools
         SET name = ?, description = ?, code = ?, language = ?, script_path = ?, required_intent = ?, safety_level = ?, version = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(tool.name)
    .bind(tool.description)
    .bind(tool.code)
    .bind(tool.language)
    .bind(tool.script_path)
    .bind(tool.required_intent)
    .bind(tool.safety_level)
    .bind(tool.version)
    .bind(tool.id)
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(())
}
