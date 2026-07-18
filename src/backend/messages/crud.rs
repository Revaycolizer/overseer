use crate::db::Database;

pub async fn add_message(
    db: &Database,
    conversation_id: i64,
    role: &str,
    content: &str,
    hide_evidence: bool,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, role, content, hide_evidence) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(hide_evidence)
    .execute(&db.pool)
    .await?;

    if role == "assistant" {
        let _ = super::attachment_utils::attach_assistant_files_from_content(db, id, content).await;
    }

    Ok(id)
}

pub async fn add_attachment(
    db: &Database,
    message_id: i64,
    filename: &str,
    file_path: &str,
    content_type: &str,
    size: i64,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO attachments (id, message_id, filename, file_path, content_type, size) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(message_id)
    .bind(filename)
    .bind(file_path)
    .bind(content_type)
    .bind(size)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_attachment_content(
    db: &Database,
    attachment_id: i64,
    extracted_content: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE attachments
         SET extracted_content = ?
         WHERE id = ?",
    )
    .bind(extracted_content)
    .bind(attachment_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_message(db: &Database, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM messages WHERE id = ?1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

