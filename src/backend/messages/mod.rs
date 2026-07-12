pub mod types;
pub mod extraction;
pub mod attachment_utils;
pub mod crud;

pub use types::*;
pub use crud::*;

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

pub async fn get_messages_page(
    db: &Database,
    conversation_id: i64,
    before_id: Option<i64>,
    limit: u32,
) -> anyhow::Result<Vec<Message>> {
    use sqlx::Row;
    let limit = limit.min(200) as i64;

    let rows = if let Some(bid) = before_id {
        sqlx::query(
            "SELECT id, conversation_id, role, content, created_at, hide_evidence
             FROM messages
             WHERE conversation_id = ?1 AND id < ?2
             ORDER BY id DESC LIMIT ?3"
        )
        .bind(conversation_id).bind(bid).bind(limit)
        .fetch_all(&db.pool).await?
    } else {
        sqlx::query(
            "SELECT id, conversation_id, role, content, created_at, hide_evidence
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY id DESC LIMIT ?2"
        )
        .bind(conversation_id).bind(limit)
        .fetch_all(&db.pool).await?
    };

    if rows.is_empty() { return Ok(Vec::new()); }

    let ids: Vec<i64> = rows.iter().map(|r| r.get::<i64, _>("id")).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let att_sql = format!(
        "SELECT id, message_id, filename, file_path, content_type, size, extracted_content
         FROM attachments WHERE message_id IN ({}) ORDER BY id ASC", placeholders);
    let mut q = sqlx::query_as::<_, Attachment>(&att_sql);
    for id in &ids { q = q.bind(*id); }
    let attachments = q.fetch_all(&db.pool).await.unwrap_or_default();

    let mut attachments_by_message: HashMap<i64, Vec<Attachment>> = HashMap::new();
    for att in attachments {
        attachments_by_message.entry(att.message_id).or_default().push(att);
    }

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows.into_iter().rev() {
        let msg_id: i64 = row.get("id");
        let created_at_str: String = row.get("created_at");
        let hide_evidence: i64 = row.get("hide_evidence");
        let created_at = chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc());
        messages.push(Message {
            id: msg_id, conversation_id: row.get("conversation_id"),
            role: row.get("role"), content: row.get("content"), created_at,
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
