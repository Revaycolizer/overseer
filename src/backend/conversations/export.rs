use crate::backend::conversations::crud::Conversation;
use serde::Serialize;
pub async fn export_conversation_markdown(
    db: &crate::db::Database,
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
pub async fn export_conversation_json(
    db: &crate::db::Database,
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
