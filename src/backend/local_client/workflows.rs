use crate::backend::client::WorkflowClient;
use crate::backend::{Workflow, WorkflowStep, WorkflowRun, WorkflowStepRun};
use super::LocalClient;

#[async_trait::async_trait]
impl WorkflowClient for LocalClient {
    async fn get_workflows(&self) -> anyhow::Result<Vec<Workflow>> { crate::backend::workflows::crud::get_workflows(&self.db).await }
    async fn add_workflow(&self, name: &str) -> anyhow::Result<i64> { crate::backend::workflows::crud::add_workflow(&self.db, name).await }
    async fn update_workflow(&self, workflow: Workflow) -> anyhow::Result<()> { crate::backend::workflows::crud::update_workflow(&self.db, workflow).await }
    async fn delete_workflow(&self, id: i64) -> anyhow::Result<()> { crate::backend::workflows::crud::delete_workflow(&self.db, id).await }
    async fn get_workflow_steps(&self, workflow_id: i64) -> anyhow::Result<Vec<WorkflowStep>> { crate::backend::workflows::crud::get_workflow_steps(&self.db, workflow_id).await }
    async fn add_workflow_step(&self, workflow_id: i64) -> anyhow::Result<i64> { crate::backend::workflows::crud::add_workflow_step(&self.db, workflow_id).await }
    async fn update_workflow_step(&self, step: WorkflowStep) -> anyhow::Result<()> { crate::backend::workflows::crud::update_workflow_step(&self.db, step).await }
    async fn delete_workflow_step(&self, step_id: i64) -> anyhow::Result<()> { crate::backend::workflows::crud::delete_workflow_step(&self.db, step_id).await }
    async fn get_workflow_runs(&self, workflow_id: i64) -> anyhow::Result<Vec<WorkflowRun>> { crate::backend::workflows::crud::get_workflow_runs(&self.db, workflow_id).await }
    async fn get_workflow_step_runs(&self, run_id: i64) -> anyhow::Result<Vec<WorkflowStepRun>> { crate::backend::workflows::crud::get_workflow_step_runs(&self.db, run_id).await }
    async fn run_workflow(&self, workflow_id: i64) -> anyhow::Result<i64> { crate::backend::workflows::run::run_workflow(&self.db, &self.agent_system, workflow_id).await }
}
