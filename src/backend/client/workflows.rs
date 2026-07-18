use crate::backend::{Workflow, WorkflowStep, WorkflowRun, WorkflowStepRun};

#[async_trait::async_trait]
pub trait WorkflowClient {
    async fn get_workflows(&self) -> anyhow::Result<Vec<Workflow>>;
    async fn add_workflow(&self, name: &str) -> anyhow::Result<i64>;
    async fn update_workflow(&self, workflow: Workflow) -> anyhow::Result<()>;
    async fn delete_workflow(&self, id: i64) -> anyhow::Result<()>;
    async fn get_workflow_steps(&self, workflow_id: i64) -> anyhow::Result<Vec<WorkflowStep>>;
    async fn add_workflow_step(&self, workflow_id: i64) -> anyhow::Result<i64>;
    async fn update_workflow_step(&self, step: WorkflowStep) -> anyhow::Result<()>;
    async fn delete_workflow_step(&self, step_id: i64) -> anyhow::Result<()>;
    async fn get_workflow_runs(&self, workflow_id: i64) -> anyhow::Result<Vec<WorkflowRun>>;
    async fn get_workflow_step_runs(&self, run_id: i64) -> anyhow::Result<Vec<WorkflowStepRun>>;
    async fn run_workflow(&self, workflow_id: i64) -> anyhow::Result<i64>;
}
