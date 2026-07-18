use super::types::{Attachment, Message};
use crate::db::Database;
use std::collections::HashMap;

/// Cursor-paged fetch of messages for a conversation.
///
/// `before_id = None` returns the latest `limit` messages.
/// `before_id = Some(b)` returns up to `limit` messages whose id is < b.
/// Results are returned in chronological order (oldest first within the page).
/// `limit` is hard-capped at 200 to keep row payloads bounded.
pub async fn get_messages_page(
    db: &Database,
    conversation_id: i64,
    before_id: Option<i64>,
    limit: u32,
) -> anyhow::Result<Vec<Message>> {
    use sqlx::Row;

    let limit = limit.min(200) as i64;

    let rows = if let Some(before) = before_id {
        sqlx::query(
            "SELECT id, conversation_id, role, content, created_at, hide_evidence
             FROM messages
             WHERE conversation_id = ?1 AND id < ?2
             ORDER BY id DESC LIMIT ?3",
        )
        .bind(conversation_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&db.pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, conversation_id, role, content, created_at, hide_evidence
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY id DESC LIMIT ?2",
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(&db.pool)
        .await?
    };

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<i64> = rows.iter().map(|r| r.get::<i64, _>("id")).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let attachments_sql = format!(
        "SELECT id, message_id, filename, file_path, content_type, size, extracted_content
         FROM attachments WHERE message_id IN ({}) ORDER BY id ASC",
        placeholders
    );
    let mut q = sqlx::query_as::<_, Attachment>(&attachments_sql);
    for id in &ids {
        q = q.bind(*id);
    }
    let attachments = q.fetch_all(&db.pool).await.unwrap_or_default();

    let mut by_message: HashMap<i64, Vec<Attachment>> = HashMap::new();
    for att in attachments {
        by_message.entry(att.message_id).or_default().push(att);
    }

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows.into_iter().rev() {
        let id: i64 = row.get("id");
        let created_at_str: String = row.get("created_at");
        let hide_evidence: i64 = row.get("hide_evidence");
        let created_at = chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap_or_default().naive_utc());
        messages.push(Message {
            id,
            conversation_id: row.get("conversation_id"),
            role: row.get("role"),
            content: row.get("content"),
            created_at,
            hide_evidence: hide_evidence != 0,
            attachments: by_message.remove(&id).unwrap_or_default(),
        });
    }
    Ok(messages)
}