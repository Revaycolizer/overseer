use crate::db::Database;

pub(crate) async fn fail_workflow_run(
    db: &Database,
    workflow_run_id: i64,
    error_message: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_runs
         SET status = 'failed', error_message = ?, finished_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(error_message)
    .bind(workflow_run_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub(crate) async fn mark_step_failed(
    db: &Database,
    step_run_id: i64,
    execution_run_id: Option<i64>,
    output: &str,
    error_message: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_step_runs
         SET execution_run_id = ?, status = 'failed', output = ?, error_message = ?, finished_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(execution_run_id)
    .bind(output)
    .bind(error_message)
    .bind(step_run_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub(crate) async fn mark_step_completed(
    db: &Database,
    step_run_id: i64,
    execution_run_id: Option<i64>,
    output: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_step_runs
         SET execution_run_id = ?, status = 'completed', output = ?, error_message = NULL, finished_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(execution_run_id)
    .bind(output)
    .bind(step_run_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}
