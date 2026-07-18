use crate::db::Database;

pub async fn execute_step(
    db: &Database,
    agent_system: &crate::engine::system::AgentSystem,
    conversation_id: i64,
    run_id: i64,
    step: &super::models::WorkflowStep,
    workflow_name: &str,
) -> anyhow::Result<Result<String, String>> {
    let step_run_id = db.next_id().await;
    sqlx::query(
        "INSERT INTO workflow_step_runs (id, workflow_run_id, workflow_step_id, execution_run_id, step_order, status)
         VALUES (?, ?, ?, ?, ?, 'running')"
    )
    .bind(step_run_id)
    .bind(run_id)
    .bind(step.id)
    .bind(None::<i64>)
    .bind(step.step_order)
    .execute(&db.pool)
    .await?;

    let user_prompt = format!(
        "Workflow: {}\nStep {} - {}\n\n{}",
        workflow_name, step.step_order, step.name, step.prompt
    );

    if let Err(err) = crate::backend::add_message(db, conversation_id, "user", &user_prompt, false).await {
        let message = format!("Failed to persist workflow step {} input: {}", step.step_order, err);
        super::run_helper::mark_step_failed(db, step_run_id, None, "", &message).await?;
        return Ok(Err(message));
    }

    let output = agent_system.orchestrate(db, conversation_id, &user_prompt, None).await;
    if let Err(err) = crate::backend::add_message(db, conversation_id, "assistant", &output, false).await {
        let message = format!("Failed to persist workflow step {} output: {}", step.step_order, err);
        super::run_helper::mark_step_failed(db, step_run_id, None, "", &message).await?;
        return Ok(Err(message));
    }

    let execution_run = crate::backend::get_latest_execution_run_for_conversation(db, conversation_id).await?;
    let execution_run_id = execution_run.as_ref().map(|run| run.id);
    let execution_status = execution_run.as_ref().map(|run| run.status.as_str()).unwrap_or("completed");
    let execution_error = execution_run.as_ref().and_then(|run| run.error_message.clone()).filter(|value| !value.trim().is_empty());
    let final_output = execution_run.as_ref().and_then(|run| run.final_response.clone()).filter(|value| !value.trim().is_empty()).unwrap_or_else(|| output.clone());

    if execution_status == "failed" {
        let message = execution_error.unwrap_or_else(|| {
            format!("Workflow step {} failed without a detailed execution error.", step.step_order)
        });
        super::run_helper::mark_step_failed(db, step_run_id, execution_run_id, &final_output, &message).await?;
        return Ok(Err(message));
    }

    super::run_helper::mark_step_completed(db, step_run_id, execution_run_id, &final_output).await?;
    Ok(Ok(final_output))
}
