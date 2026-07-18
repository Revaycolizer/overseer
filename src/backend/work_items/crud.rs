use crate::db::Database;

pub async fn create_work_item(
    db: &Database,
    conversation_id: i64,
    title: &str,
    brief: Option<&str>,
    status: &str,
    surface_kind: &str,
    assigned_domain: Option<&str>,
    prompt: &str,
    source_artifact_ids_json: Option<&str>,
    metadata_json: Option<&str>,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO work_items (
            id,
            conversation_id,
            title,
            brief,
            status,
            surface_kind,
            assigned_domain,
            prompt,
            source_artifact_ids_json,
            metadata_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(conversation_id)
    .bind(title)
    .bind(brief)
    .bind(status)
    .bind(surface_kind)
    .bind(assigned_domain)
    .bind(prompt)
    .bind(source_artifact_ids_json)
    .bind(metadata_json)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_work_item_status(
    db: &Database,
    work_item_id: i64,
    status: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE work_items
         SET status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(status)
    .bind(work_item_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn update_work_item_runtime(
    db: &Database,
    work_item_id: i64,
    status: &str,
    metadata_json: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE work_items
         SET status = ?, metadata_json = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(status)
    .bind(metadata_json)
    .bind(work_item_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}
