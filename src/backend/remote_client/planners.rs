use crate::backend::client::*;
use crate::backend::*;
use super::RemoteClient;

#[async_trait::async_trait]
impl PlannerClient for RemoteClient {
    async fn get_planners(&self) -> anyhow::Result<Vec<Planner>> { Ok(self.http_client.get(format!("{}/api/planners", self.base_url)).send().await?.json().await?) }
    async fn add_planner(&self, n: &str, i: &str, m: &str) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/planners", self.base_url)).json(&serde_json::json!({ "name": n, "instructions": i, "model": m })).send().await?.json().await?) }
    async fn update_planner(&self, p: Planner) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/planners/update", self.base_url)).json(&p).send().await?; Ok(()) }
    async fn delete_planner(&self, id: i64) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/planners/delete", self.base_url)).json(&serde_json::json!({ "id": id })).send().await?; Ok(()) }
}

#[async_trait::async_trait]
impl VersionClient for RemoteClient {
    async fn publish_version(&self, t: &str, id: i64, tag: &str, d: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/version/publish", self.base_url)).json(&serde_json::json!({ "entity_type": t, "entity_id": id, "tag": tag, "description": d })).send().await?; Ok(()) }
    async fn publish_agent_version(&self, id: i64, tag: &str, d: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/version/publish_agent", self.base_url)).json(&serde_json::json!({ "agent_id": id, "tag": tag, "description": d })).send().await?; Ok(()) }
    async fn publish_tool_version(&self, id: i64, tag: &str, d: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/version/publish_tool", self.base_url)).json(&serde_json::json!({ "tool_id": id, "tag": tag, "description": d })).send().await?; Ok(()) }
    async fn rollback_version(&self, t: &str, id: i64, tag: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/version/rollback", self.base_url)).json(&serde_json::json!({ "entity_type": t, "entity_id": id, "tag": tag })).send().await?; Ok(()) }
    async fn get_version_content(&self, t: &str, id: i64, tag: &str) -> anyhow::Result<String> { Ok(self.http_client.get(format!("{}/api/version/content?entity_type={}&entity_id={}&tag={}", self.base_url, t, id, tag)).send().await?.json().await?) }
    async fn get_version_history(&self, t: &str, id: i64) -> anyhow::Result<Vec<EntityVersion>> { Ok(self.http_client.get(format!("{}/api/version/history?entity_type={}&entity_id={}", self.base_url, t, id)).send().await?.json().await?) }
}
