pub mod extraction;
pub mod attachment_utils;
pub mod crud;
pub mod pagination;
pub mod titling;
pub mod types;

pub use crud::*;
pub use pagination::get_messages_page;
pub use types::*;

use crate::db::Database;
use std::collections::HashMap;

pub async fn get_messages(db: &Database, conversation_id: i64) -> anyhow::Result<Vec<Message>> {
    use sqlx::Row;
    let message_rows = sqlx::query(
        "SELECT id, conversation_id, role, content, created_at, hide_evidence
         FROM messages
         WHERE conversation_id = ?
         ORDER BY created_at ASC"
    )
    .bind(conversation_id)
    .fetch_all(&db.pool)
    .await?;

    let attachment_rows = sqlx::query_as::<_, Attachment>(
        "SELECT a.id, a.message_id, a.filename, a.file_path, a.content_type, a.size, a.extracted_content
         FROM attachments a
         JOIN messages m ON m.id = a.message_id
         WHERE m.conversation_id = ?
         ORDER BY a.id ASC",
    )
    .bind(conversation_id)
    .fetch_all(&db.pool)
    .await?;

    let mut attachments_by_message: HashMap<i64, Vec<Attachment>> = HashMap::new();
    for att in attachment_rows {
        attachments_by_message
            .entry(att.message_id)
            .or_default()
            .push(att);
    }

    let mut messages = Vec::new();
    for row in message_rows {
        let msg_id: i64 = row.get("id");
        let created_at_str: String = row.get("created_at");
        let hide_evidence: i64 = row.get("hide_evidence");

        let created_at = chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc());

        messages.push(Message {
            id: msg_id,
            conversation_id: row.get("conversation_id"),
            role: row.get("role"),
            content: row.get("content"),
            created_at,
            hide_evidence: hide_evidence != 0,
            attachments: attachments_by_message.remove(&msg_id).unwrap_or_default(),
        });
    }

    Ok(messages)
}

pub async fn get_attachments(db: &Database, message_id: i64) -> anyhow::Result<Vec<Attachment>> {
    let rows = sqlx::query_as::<_, Attachment>(
        "SELECT id, message_id, filename, file_path, content_type, size, extracted_content
         FROM attachments
         WHERE message_id = ?
         ORDER BY id ASC",
    )
    .bind(message_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}
