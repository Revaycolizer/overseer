use crate::db::Database;
use super::models::WorkflowStep;

pub async fn get_workflow_steps(
    db: &Database,
    workflow_id: i64,
) -> anyhow::Result<Vec<WorkflowStep>> {
    let rows = sqlx::query_as::<_, WorkflowStep>(
        "SELECT id, workflow_id, step_order, name, prompt, enabled, created_at, updated_at
         FROM workflow_steps
         WHERE workflow_id = ?
         ORDER BY step_order ASC, created_at ASC",
    )
    .bind(workflow_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_workflow_step(db: &Database, workflow_id: i64) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let next_order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(step_order), 0) + 1 FROM workflow_steps WHERE workflow_id = ?",
    )
    .bind(workflow_id)
    .fetch_one(&db.pool)
    .await?;

    sqlx::query(
        "INSERT INTO workflow_steps (id, workflow_id, step_order, name, prompt, enabled)
         VALUES (?, ?, ?, ?, ?, 1)",
    )
    .bind(id)
    .bind(workflow_id)
    .bind(next_order.0)
    .bind(format!("Step {}", next_order.0))
    .bind("Describe the action this workflow step should perform.")
    .execute(&db.pool)
    .await?;

    sqlx::query("UPDATE workflows SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(workflow_id)
        .execute(&db.pool)
        .await?;

    Ok(id)
}

pub async fn update_workflow_step(db: &Database, step: WorkflowStep) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_steps
         SET step_order = ?, name = ?, prompt = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(step.step_order)
    .bind(step.name)
    .bind(step.prompt)
    .bind(step.enabled)
    .bind(step.id)
    .execute(&db.pool)
    .await?;

    sqlx::query("UPDATE workflows SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(step.workflow_id)
        .execute(&db.pool)
        .await?;

    Ok(())
}

pub async fn delete_workflow_step(db: &Database, step_id: i64) -> anyhow::Result<()> {
    let workflow: Option<(i64,)> =
        sqlx::query_as("SELECT workflow_id FROM workflow_steps WHERE id = ?")
            .bind(step_id)
            .fetch_optional(&db.pool)
            .await?;

    sqlx::query("DELETE FROM workflow_steps WHERE id = ?")
        .bind(step_id)
        .execute(&db.pool)
        .await?;

    if let Some((workflow_id,)) = workflow {
        let steps = get_workflow_steps(db, workflow_id).await?;
        for (index, step) in steps.into_iter().enumerate() {
            sqlx::query("UPDATE workflow_steps SET step_order = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind((index + 1) as i64)
                .bind(step.id)
                .execute(&db.pool)
                .await?;
        }
        sqlx::query("UPDATE workflows SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(workflow_id)
            .execute(&db.pool)
            .await?;
    }

    Ok(())
}
