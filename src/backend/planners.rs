use crate::db::Database;

pub async fn get_planners(db: &Database) -> anyhow::Result<Vec<crate::db::Planner>> {
    let rows = sqlx::query_as::<_, crate::db::Planner>(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code, response_format FROM planners ORDER BY name ASC"
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_planner(
    db: &Database,
    name: &str,
    description: &str,
    domain: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let model =
        super::resolve_model_name(db, None, "default_chat_model", "qwen2.5:7b-instruct").await;
    sqlx::query(
        "INSERT INTO planners (id, name, description, domain, model, version, instructions, code, response_format) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind(domain).bind(model).bind("1.0.0").bind("").bind("").bind("json")
    .execute(&db.pool)
    .await?;
    super::refresh_runtime_registry(db).await;
    Ok(id)
}

pub async fn update_planner(db: &Database, planner: crate::db::Planner) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE planners SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ?, response_format = ? WHERE id = ?"
    )
    .bind(planner.name).bind(planner.description).bind(planner.instructions).bind(planner.model)
    .bind(planner.domain).bind(planner.version).bind(planner.code).bind(planner.response_format).bind(planner.id)
    .execute(&db.pool)
    .await?;
    super::refresh_runtime_registry(db).await;
    Ok(())
}

pub async fn delete_planner(db: &Database, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM planners WHERE id = ?")
        .bind(id)
        .execute(&db.pool)
        .await?;
    super::refresh_runtime_registry(db).await;
    Ok(())
}
