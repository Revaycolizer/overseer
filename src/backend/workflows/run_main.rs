use crate::db::Database;

pub async fn run_workflow(
    db: &Database,
    agent_system: &crate::engine::system::AgentSystem,
    workflow_id: i64,
) -> anyhow::Result<i64> {
    let workflow = sqlx::query_as::<_, super::models::Workflow>(
        "SELECT id, name, description, trigger_type, trigger_config, status, created_at, updated_at
         FROM workflows
         WHERE id = ?",
    )
    .bind(workflow_id)
    .fetch_one(&db.pool)
    .await?;

    let steps = super::crud::get_workflow_steps(db, workflow_id)
        .await?
        .into_iter()
        .filter(|step| step.enabled)
        .collect::<Vec<_>>();

    let run_id = db.next_id().await;
    sqlx::query("INSERT INTO workflow_runs (id, workflow_id, status) VALUES (?, ?, 'running')")
        .bind(run_id)
        .bind(workflow_id)
        .execute(&db.pool)
        .await?;

    if steps.is_empty() {
        super::run_helper::fail_workflow_run(db, run_id, "Workflow has no enabled steps.").await?;
        return Ok(run_id);
    }

    let conversation_id =
        match crate::backend::add_conversation(db, &format!("Workflow Run: {}", workflow.name))
            .await
        {
            Ok(id) => id,
            Err(err) => {
                let message = format!("Failed to create workflow conversation: {}", err);
                super::run_helper::fail_workflow_run(db, run_id, &message).await?;
                return Ok(run_id);
            }
        };
    sqlx::query("UPDATE workflow_runs SET conversation_id = ? WHERE id = ?")
        .bind(conversation_id)
        .bind(run_id)
        .execute(&db.pool)
        .await?;

    let mut last_output = String::new();
    for step in steps {
        match super::run_step::execute_step(
            db,
            agent_system,
            conversation_id,
            run_id,
            &step,
            &workflow.name,
        )
        .await?
        {
            Ok(output) => {
                last_output = output;
            }
            Err(err_msg) => {
                super::run_helper::fail_workflow_run(db, run_id, &err_msg).await?;
                return Ok(run_id);
            }
        }
    }

    sqlx::query(
        "UPDATE workflow_runs SET status = 'completed', summary = ?, finished_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(last_output)
    .bind(run_id)
    .execute(&db.pool)
    .await?;

    Ok(run_id)
}
