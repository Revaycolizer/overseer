use crate::db::Database;

/// If this is the first user message on the conversation, write an
/// optimistic title (first four words of the query) and return it. The
/// chat_input Dioxus flow layers a later LLM-refined title on top via
/// the channels; the TUI uses just this optimistic value.
///
/// Returns Some(simple_title) when the write happened (callers can use
/// the presence of the value to decide whether to spawn LLM refinement
/// and avoid double-counting the conversation).
pub async fn maybe_set_initial_title(db: &Database, conversation_id: i64, query: &str) -> Option<String> {
    let count_row: Result<(i64,), _> = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = ? AND role = 'user'",
    )
    .bind(conversation_id)
    .fetch_one(&db.pool)
    .await;
    let count = count_row.ok()?.0;
    if count != 1 {
        return None;
    }

    let simple_title = query
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    let _ = crate::backend::update_conversation_title(db, conversation_id, &simple_title).await;
    Some(simple_title)
}