use super::models::Workflow;
use crate::db::Database;

pub async fn get_workflows(db: &Database) -> anyhow::Result<Vec<Workflow>> {
    let rows = sqlx::query_as::<_, Workflow>(
        "SELECT id, name, description, trigger_type, trigger_config, status, created_at, updated_at
         FROM workflows
         ORDER BY updated_at DESC",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_workflow(db: &Database, name: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO workflows (id, name, description, trigger_type, trigger_config, status)
         VALUES (?, ?, ?, 'manual', ?, 'draft')",
    )
    .bind(id)
    .bind(name)
    .bind("New workflow")
    .bind(None::<String>)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_workflow(db: &Database, workflow: Workflow) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflows
         SET name = ?, description = ?, trigger_type = ?, trigger_config = ?, status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(workflow.name)
    .bind(workflow.description)
    .bind(workflow.trigger_type)
    .bind(workflow.trigger_config)
    .bind(workflow.status)
    .bind(workflow.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_workflow(db: &Database, workflow_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM workflows WHERE id = ?")
        .bind(workflow_id)
        .execute(&db.pool)
        .await?;
    Ok(())
}
