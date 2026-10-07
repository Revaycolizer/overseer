use super::models::{WorkflowRun, WorkflowStepRun};
use crate::db::Database;

pub async fn get_workflow_runs(
    db: &Database,
    workflow_id: i64,
) -> anyhow::Result<Vec<WorkflowRun>> {
    let rows = sqlx::query_as::<_, WorkflowRun>(
        "SELECT id, workflow_id, conversation_id, status, summary, error_message, created_at, finished_at
         FROM workflow_runs
         WHERE workflow_id = ?
         ORDER BY created_at DESC"
    )
    .bind(workflow_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_workflow_step_runs(
    db: &Database,
    workflow_run_id: i64,
) -> anyhow::Result<Vec<WorkflowStepRun>> {
    let rows = sqlx::query_as::<_, WorkflowStepRun>(
        "SELECT id, workflow_run_id, workflow_step_id, execution_run_id, step_order, status, output, error_message, created_at, finished_at
         FROM workflow_step_runs
         WHERE workflow_run_id = ?
         ORDER BY step_order ASC"
    )
    .bind(workflow_run_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}
