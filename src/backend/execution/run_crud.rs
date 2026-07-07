use crate::db::Database;

pub async fn start_execution_run(
    db: &Database,
    conversation_id: i64,
    user_query: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO execution_runs (id, conversation_id, user_query, status) VALUES (?, ?, ?, 'running')"
    )
    .bind(id)
    .bind(conversation_id)
    .bind(user_query)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn finish_execution_run(
    db: &Database,
    run_id: i64,
    status: &str,
    final_response: Option<&str>,
    error_message: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE execution_runs SET status = ?, final_response = ?, error_message = ?, finished_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(status)
    .bind(final_response)
    .bind(error_message)
    .bind(run_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn cancel_latest_execution_run_for_conversation(
    db: &Database,
    conversation_id: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE execution_runs
         SET status = 'cancelled',
             error_message = COALESCE(error_message, 'Cancelled by user.'),
             finished_at = CURRENT_TIMESTAMP
         WHERE id = (
             SELECT id
             FROM execution_runs
             WHERE conversation_id = ? AND status = 'running'
             ORDER BY created_at DESC
             LIMIT 1
         )",
    )
    .bind(conversation_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}
