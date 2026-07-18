use crate::db::Database;

/// Returns `true` if this is the first user message on the conversation
/// (the LLM-refined title should be spawned for this turn).
///
/// We intentionally do NOT write a placeholder. New conversations start
/// with the title "New Conversation" set by `add_conversation`; the
/// LLM refinement in `agent_system::orchestrate` writes the proper
/// title within 1-3 s. An earlier 4-word placeholder write raced with
/// the LLM and overwrote the refined title when the LLM returned
/// empty / errored.
pub async fn is_first_user_message(db: &Database, conversation_id: i64) -> bool {
    let count_row: Result<(i64,), _> = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = ? AND role = 'user'",
    )
    .bind(conversation_id)
    .fetch_one(&db.pool)
    .await;
    count_row.map(|row| row.0 == 1).unwrap_or(false)
}
