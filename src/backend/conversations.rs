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

/// V4.17.10 (json variant): same data as the Markdown export but
/// emitted as a single JSON object. Useful for re-importing into
/// another Overseer install, a tool, or a custom analyzer. The
/// format mirrors what the chat pipeline already stores in
/// `messages.content` (a `StructuredResponse` JSON), so no
/// information is lost.
pub async fn export_conversation_json(
    db: &Database,
    conversation_id: i64,
) -> anyhow::Result<String> {
    use serde::Serialize;

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

    #[derive(Serialize)]
    struct ExportedMessage {
        role: String,
        structured: Option<serde_json::Value>,
        raw: String,
    }

    let exported: Vec<ExportedMessage> = messages
        .iter()
        .map(|m| {
            let structured = serde_json::from_str::<serde_json::Value>(&m.content).ok();
            ExportedMessage {
                role: m.role.clone(),
                structured,
                raw: m.content.clone(),
            }
        })
        .collect();

    #[derive(Serialize)]
    struct ExportedConversation {
        id: i64,
        title: String,
        exported_at: String,
        messages: Vec<ExportedMessage>,
    }

    let payload = ExportedConversation {
        id: conv.id,
        title: conv.title,
        exported_at: chrono::Local::now().to_rfc3339(),
        messages: exported,
    };

    serde_json::to_string_pretty(&payload).map_err(Into::into)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ConversationSearchResult {
    pub conversation_id: i64,
    pub conversation_title: String,
    pub message_id: i64,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
}

pub fn extract_clean_text(content: &str) -> String {
    let trimmed = content.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}')) || (trimmed.starts_with('[') && trimmed.ends_with(']')) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(arr) = val.get("messages").and_then(|v| v.as_array()) {
                let prose = arr.iter()
                    .filter_map(|m| m.get("message").and_then(|v| v.as_str()))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !prose.is_empty() {
                    return clean_text_markers(&prose);
                }
            }
            if let Some(msg) = val.get("message").and_then(|v| v.as_str()) {
                return clean_text_markers(msg);
            }
            if let Some(ans) = val.get("answer").and_then(|v| v.as_str()) {
                return clean_text_markers(ans);
            }
        }
    }
    clean_text_markers(content)
}

fn clean_text_markers(text: &str) -> String {
    let re = regex::Regex::new(r"\[E[0-9]+\]").unwrap();
    let text = re.replace_all(text, "").to_string();
    let text = text.lines().filter(|line| {
        let t = line.trim();
        if t == "ARTIFACT_CONTEXT:" || t == "WORK_ITEM_CONTEXT:" { return false; }
        !(t.contains("[[ATTACH_IMAGE:") || t.contains("[[ATTACH_FILE:") || t.contains("[[ARTIFACT_REF:") || t.contains("[[WORK_ITEM_REF:"))
    }).collect::<Vec<_>>().join("\n");
    text.trim().to_string()
}

pub async fn search_messages(
    db: &Database,
    query: &str,
) -> anyhow::Result<Vec<ConversationSearchResult>> {
    let sql = r#"
        SELECT 
            m.conversation_id as conversation_id,
            c.title as conversation_title,
            m.id as message_id,
            m.content as content,
            m.created_at as created_at
        FROM messages m
        JOIN conversations c ON m.conversation_id = c.id
        WHERE m.content LIKE ?1
        ORDER BY m.created_at DESC
        LIMIT 50
    "#;

    let search_pattern = format!("%{}%", query);
    let mut results = sqlx::query_as::<_, ConversationSearchResult>(sql)
        .bind(search_pattern)
        .fetch_all(&db.pool)
        .await?;

    for r in &mut results {
        r.content = extract_clean_text(&r.content);
    }

    Ok(results)
}

