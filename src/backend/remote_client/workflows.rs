use crate::backend::client::WorkflowClient;
use crate::backend::{Workflow, WorkflowStep, WorkflowRun, WorkflowStepRun};
use super::RemoteClient;

#[async_trait::async_trait]
impl WorkflowClient for RemoteClient {
    async fn get_workflows(&self) -> anyhow::Result<Vec<Workflow>> { Ok(self.http_client.get(format!("{}/api/workflows", self.base_url)).send().await?.json().await?) }
    async fn add_workflow(&self, name: &str) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/workflows", self.base_url)).json(&serde_json::json!({ "name": name })).send().await?.json().await?) }
    async fn update_workflow(&self, w: Workflow) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/workflows/update", self.base_url)).json(&w).send().await?; Ok(()) }
    async fn delete_workflow(&self, id: i64) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/workflows/delete", self.base_url)).json(&serde_json::json!({ "id": id })).send().await?; Ok(()) }
    async fn get_workflow_steps(&self, id: i64) -> anyhow::Result<Vec<WorkflowStep>> { Ok(self.http_client.get(format!("{}/api/workflows/{}/steps", self.base_url, id)).send().await?.json().await?) }
    async fn add_workflow_step(&self, id: i64) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/workflows/{}/steps", self.base_url, id)).send().await?.json().await?) }
    async fn update_workflow_step(&self, s: WorkflowStep) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/workflows/steps/update", self.base_url)).json(&s).send().await?; Ok(()) }
    async fn delete_workflow_step(&self, id: i64) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/workflows/steps/delete", self.base_url)).json(&serde_json::json!({ "id": id })).send().await?; Ok(()) }
    async fn get_workflow_runs(&self, id: i64) -> anyhow::Result<Vec<WorkflowRun>> { Ok(self.http_client.get(format!("{}/api/workflows/{}/runs", self.base_url, id)).send().await?.json().await?) }
    async fn get_workflow_step_runs(&self, id: i64) -> anyhow::Result<Vec<WorkflowStepRun>> { Ok(self.http_client.get(format!("{}/api/workflows/runs/{}/steps", self.base_url, id)).send().await?.json().await?) }
    async fn run_workflow(&self, id: i64) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/workflows/{}/run", self.base_url, id)).send().await?.json().await?) }
}
