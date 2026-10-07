use serde::{Deserialize, Serialize};

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
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(arr) = val.get("messages").and_then(|v| v.as_array()) {
                let prose = arr
                    .iter()
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
    let text = text
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t == "ARTIFACT_CONTEXT:" || t == "WORK_ITEM_CONTEXT:" {
                return false;
            }
            !(t.contains("[[ATTACH_IMAGE:")
                || t.contains("[[ATTACH_FILE:")
                || t.contains("[[ARTIFACT_REF:")
                || t.contains("[[WORK_ITEM_REF:"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    text.trim().to_string()
}

fn fts5_escape(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let escaped: String = trimmed
        .chars()
        .map(|c| match c {
            '"' | '^' | '*' | '(' | ')' | '+' | '-' | '~' => ' ',
            _ => c,
        })
        .collect();
    format!("\"{}\"", escaped)
}

pub async fn search_messages(
    db: &crate::db::Database,
    query: &str,
) -> anyhow::Result<Vec<ConversationSearchResult>> {
    let fts_query = fts5_escape(query);
    if fts_query.is_empty() {
        return Ok(vec![]);
    }

    let sql = r#"
        SELECT 
            m.conversation_id as conversation_id,
            c.title as conversation_title,
            m.id as message_id,
            m.content as content,
            m.created_at as created_at
        FROM messages_fts
        JOIN messages m ON messages_fts.rowid = m.id
        JOIN conversations c ON m.conversation_id = c.id
        WHERE messages_fts MATCH ?1
        ORDER BY rank
        LIMIT 50
    "#;

    let mut results = sqlx::query_as::<_, ConversationSearchResult>(sql)
        .bind(&fts_query)
        .fetch_all(&db.pool)
        .await?;

    for r in &mut results {
        r.content = extract_clean_text(&r.content);
    }

    Ok(results)
}
