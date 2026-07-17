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
        "SELECT id, title, created_at FROM conversations ORDER BY id DESC",
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

/// V4.17.10: render a conversation as a self-contained Markdown
/// document. Reads the conversation metadata and all
/// user/assistant messages and renders a portable `.md` string
/// suitable for sharing or archiving. The conversion reuses
/// `engine::orchestrator::utils::formatter::format_for_markdown`
/// to normalise message content (charts, evidence, tool blocks) so
/// the output reads as plain Markdown.
pub async fn export_conversation_markdown(
    db: &Database,
    conversation_id: i64,
) -> anyhow::Result<String> {
    let conv = sqlx::query_as::<_, Conversation>(
        "SELECT id, title, created_at FROM conversations WHERE id = ?1",
    )
    .bind(conversation_id)
    .fetch_optional(&db.pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("conversation {conversation_id} not found"))?;

    let messages = crate::backend::routing_wrappers::conversations::get_messages_page(
        db, conversation_id, None, 1_000_000,
    )
    .await
    .unwrap_or_default();

    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", conv.title));
    out.push_str(&format!(
        "_Exported {} from Overseer._\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    out.push_str("---\n\n");

    for (i, m) in messages.iter().enumerate() {
        let header = match m.role.as_str() {
            "user" => format!("## {}. {}", i + 1, "User"),
            "assistant" => format!("## {}. {}", i + 1, "Assistant"),
            other => format!("## {}. {}", i + 1, other),
        };
        out.push_str(&header);
        out.push('\n');
        if let Ok(structured) =
            serde_json::from_str::<crate::engine::orchestrator::StructuredResponse>(&m.content)
        {
            out.push_str(&crate::engine::orchestrator::utils::formatter::format_for_markdown(
                &structured,
            ));
        } else {
            out.push_str(&m.content);
        }
        out.push_str("\n\n---\n\n");
    }

    Ok(out)
}
