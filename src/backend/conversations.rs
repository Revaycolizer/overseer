use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn get_conversations(db: &Database) -> anyhow::Result<Vec<Conversation>> {
    let convs = sqlx::query_as::<_, Conversation>(
        "SELECT id, title, created_at FROM conversations ORDER BY updated_at DESC",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(convs)
}

pub async fn add_conversation(db: &Database, title: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query("INSERT INTO conversations (id, title) VALUES (?, ?)")
        .bind(id)
        .bind(title)
        .execute(&db.pool)
        .await?;
    Ok(id)
}

pub async fn update_conversation_title(db: &Database, id: i64, title: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE conversations SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn delete_conversation(db: &Database, id: i64) -> anyhow::Result<()> {
    let attachment_paths: Vec<String> = sqlx::query_scalar(
        "SELECT a.file_path
         FROM attachments a
         JOIN messages m ON m.id = a.message_id
         WHERE m.conversation_id = ?",
    )
    .bind(id)
    .fetch_all(&db.pool)
    .await
    .unwrap_or_default();

    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(id)
        .execute(&db.pool)
        .await?;

    for path in attachment_paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }

        let _ = std::fs::remove_file(trimmed);
    }

    Ok(())
}
