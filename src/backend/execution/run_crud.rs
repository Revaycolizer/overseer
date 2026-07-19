use crate::db::Database;
use crate::engine::orchestrator::state::EvidenceDag;

/// Did this run actually execute a tool / action successfully?
///
/// Used by the truthfulness guard: an assistant claim of a side effect is only
/// allowed if a real tool returned success (a completed `tool_result` or a kernel
/// `action_completed` event). Returns false when `run_id` is `None` or nothing
/// ran (e.g. a direct-chat reply with no tools).
pub async fn run_produced_action(db: &Database, run_id: Option<i64>) -> bool {
    let Some(rid) = run_id else {
        return false;
    };
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM execution_events \
         WHERE run_id = ? \
           AND ((event_type = 'tool_result' AND status = 'completed') \
                OR event_type = 'action_completed')",
    )
    .bind(rid)
    .fetch_one(&db.pool)
    .await
    .unwrap_or(0);
    count > 0
}

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

/// Build an Evidence DAG from a run's execution trace.
///
/// Queries the execution events for tool results and artefact records,
/// converting them into verified evidence nodes. Returns `None` when
/// `run_id` is `None` or no evidence is found.
pub async fn build_evidence_dag_from_run(
    db: &Database,
    run_id: Option<i64>,
) -> Option<EvidenceDag> {
    let rid = run_id?;

    // Collect tool result summaries as evidence chunks
    let tool_results: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_type, summary FROM execution_events \
         WHERE run_id = ? \
           AND event_type IN ('tool_result', 'action_completed', 'artifact_created') \
           AND summary IS NOT NULL \
         ORDER BY event_id",
    )
    .bind(rid)
    .fetch_all(&db.pool)
    .await
    .ok()?;

    if tool_results.is_empty() {
        return None;
    }

    let chunks: Vec<crate::engine::orchestrator::state::EvidenceChunk> = tool_results
        .into_iter()
        .enumerate()
        .map(|(i, (event_type, summary))| crate::engine::orchestrator::state::EvidenceChunk {
            id: format!("E{}", i + 1),
            snowflake_id: (i + 1) as i64,
            text: summary,
            source: format!("execution_run:{}:{}", rid, event_type),
        })
        .collect();

    Some(EvidenceDag::from_evidence_chunks(&chunks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_produced_action_reflects_trace() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let run = start_execution_run(&db, conv, "q").await.unwrap();

        // No tool events yet -> the truthfulness gate must report no action.
        assert!(!run_produced_action(&db, Some(run)).await, "no events -> false");

        // A completed tool result flips it to true.
        crate::backend::log_execution_event(
            &db, run, 1, "tool_result", "completed", Some("computer"), Some("step"),
            Some("os_tool"), None, Some("ok"), None,
        )
        .await
        .unwrap();
        assert!(run_produced_action(&db, Some(run)).await, "completed tool_result -> true");

        // No run at all -> false (direct-chat with no tools).
        assert!(!run_produced_action(&db, None).await, "no run -> false");
    }
}
