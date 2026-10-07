use crate::backend::client::WorkflowClient;
use crate::backend::{Workflow, WorkflowRun, WorkflowStep, WorkflowStepRun};
use crate::db::Database;

pub async fn get_workflows(db: &Database) -> anyhow::Result<Vec<Workflow>> {
    if db.is_remote() {
        return db.remote_client().get_workflows().await;
    }
    crate::backend::workflows::crud::get_workflows(db).await
}

pub async fn add_workflow(db: &Database, name: &str) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db.remote_client().add_workflow(name).await;
    }
    crate::backend::workflows::crud::add_workflow(db, name).await
}

pub async fn update_workflow(db: &Database, workflow: Workflow) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().update_workflow(workflow).await;
    }
    crate::backend::workflows::crud::update_workflow(db, workflow).await
}

pub async fn delete_workflow(db: &Database, id: i64) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().delete_workflow(id).await;
    }
    crate::backend::workflows::crud::delete_workflow(db, id).await
}

pub async fn get_workflow_steps(
    db: &Database,
    workflow_id: i64,
) -> anyhow::Result<Vec<WorkflowStep>> {
    if db.is_remote() {
        return db.remote_client().get_workflow_steps(workflow_id).await;
    }
    crate::backend::workflows::crud::get_workflow_steps(db, workflow_id).await
}

pub async fn add_workflow_step(db: &Database, workflow_id: i64) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db.remote_client().add_workflow_step(workflow_id).await;
    }
    crate::backend::workflows::crud::add_workflow_step(db, workflow_id).await
}

pub async fn update_workflow_step(db: &Database, step: WorkflowStep) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().update_workflow_step(step).await;
    }
    crate::backend::workflows::crud::update_workflow_step(db, step).await
}

pub async fn delete_workflow_step(db: &Database, step_id: i64) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().delete_workflow_step(step_id).await;
    }
    crate::backend::workflows::crud::delete_workflow_step(db, step_id).await
}

pub async fn get_workflow_runs(
    db: &Database,
    workflow_id: i64,
) -> anyhow::Result<Vec<WorkflowRun>> {
    if db.is_remote() {
        return db.remote_client().get_workflow_runs(workflow_id).await;
    }
    crate::backend::workflows::crud::get_workflow_runs(db, workflow_id).await
}

pub async fn get_workflow_step_runs(
    db: &Database,
    run_id: i64,
) -> anyhow::Result<Vec<WorkflowStepRun>> {
    if db.is_remote() {
        return db.remote_client().get_workflow_step_runs(run_id).await;
    }
    crate::backend::workflows::crud::get_workflow_step_runs(db, run_id).await
}

pub async fn run_workflow(
    db: &Database,
    agent_system: &crate::engine::system::AgentSystem,
    workflow_id: i64,
) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db.remote_client().run_workflow(workflow_id).await;
    }
    crate::backend::workflows::run::run_workflow(db, agent_system, workflow_id).await
}
