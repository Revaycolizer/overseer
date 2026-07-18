use crate::db::Database;

pub async fn create_artifact(
    db: &Database,
    conversation_id: i64,
    execution_run_id: Option<i64>,
    domain: Option<&str>,
    step_text: Option<&str>,
    kind: &str,
    label: &str,
    content: Option<&str>,
    file_path: Option<&str>,
    content_type: Option<&str>,
    metadata_json: Option<&str>,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO artifacts (
            id,
            conversation_id,
            execution_run_id,
            domain,
            step_text,
            kind,
            label,
            content,
            file_path,
            content_type,
            metadata_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(conversation_id)
    .bind(execution_run_id)
    .bind(domain)
    .bind(step_text)
    .bind(kind)
    .bind(label)
    .bind(content)
    .bind(file_path)
    .bind(content_type)
    .bind(metadata_json)
    .execute(&db.pool)
    .await?;
    Ok(id)
}
