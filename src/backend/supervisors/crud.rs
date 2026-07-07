use crate::db::Database;
use crate::db::Supervisor;

pub async fn get_supervisors(db: &Database) -> anyhow::Result<Vec<Supervisor>> {
    let rows = sqlx::query_as::<_, Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code, bypass_synthesizer, fast_path_keywords, verification_agent_id, routing_priority, hide_evidence, report_style, skip_fast_path_if_artifacts, artifact_fast_path_keywords, response_format FROM supervisors"
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_supervisor(
    db: &Database,
    name: &str,
    description: &str,
    domain: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let model = crate::backend::resolve_model_name(db, None, "default_chat_model", "qwen2.5:7b-instruct").await;
    sqlx::query(
        "INSERT INTO supervisors (id, name, description, domain, model, version, instructions, code, bypass_synthesizer, fast_path_keywords, verification_agent_id, routing_priority, hide_evidence, report_style, skip_fast_path_if_artifacts, artifact_fast_path_keywords, response_format)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(domain)
    .bind(model)
    .bind("1.0.0")
    .bind("")
    .bind("")
    .bind(false)
    .bind(None::<String>)
    .bind(None::<i64>)
    .bind(0)
    .bind(false)
    .bind("detailed")
    .bind(false)
    .bind(None::<String>)
    .bind("json")
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(id)
}

pub async fn update_supervisor(db: &Database, supervisor: Supervisor) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE supervisors
         SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ?, bypass_synthesizer = ?, fast_path_keywords = ?, verification_agent_id = ?, routing_priority = ?, hide_evidence = ?, report_style = ?, skip_fast_path_if_artifacts = ?, artifact_fast_path_keywords = ?, response_format = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(supervisor.name)
    .bind(supervisor.description)
    .bind(supervisor.instructions)
    .bind(supervisor.model)
    .bind(supervisor.domain)
    .bind(supervisor.version)
    .bind(supervisor.code)
    .bind(supervisor.bypass_synthesizer)
    .bind(supervisor.fast_path_keywords)
    .bind(supervisor.verification_agent_id)
    .bind(supervisor.routing_priority)
    .bind(supervisor.hide_evidence)
    .bind(supervisor.report_style)
    .bind(supervisor.skip_fast_path_if_artifacts)
    .bind(supervisor.artifact_fast_path_keywords)
    .bind(supervisor.response_format)
    .bind(supervisor.id)
    .execute(&db.pool)
    .await?;

    super::super::refresh_runtime_registry(db).await;
    Ok(())
}

pub async fn delete_supervisor(db: &Database, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM supervisors WHERE id = ?")
        .bind(id)
        .execute(&db.pool)
        .await?;
    super::super::refresh_runtime_registry(db).await;
    Ok(())
}
