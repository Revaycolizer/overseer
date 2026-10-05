use super::RemoteClient;
use crate::backend::client::{AgentClient, SupervisorClient, ToolClient};
use crate::db::Supervisor;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};

#[async_trait::async_trait]
impl AgentClient for RemoteClient {
    async fn get_agents(&self) -> anyhow::Result<Vec<AgentInfo>> {
        Ok(self
            .http_client
            .get(format!("{}/api/agents", self.base_url))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn get_all_agents(&self) -> anyhow::Result<Vec<AgentInfo>> {
        Ok(self
            .http_client
            .get(format!("{}/api/agents/all", self.base_url))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn add_agent(&self, name: &str, desc: &str, dom: &str) -> anyhow::Result<i64> {
        Ok(self
            .http_client
            .post(format!("{}/api/agents", self.base_url))
            .json(&serde_json::json!({ "name": name, "description": desc, "domain": dom }))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn update_agent(&self, a: AgentInfo) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/agents/update", self.base_url))
            .json(&a)
            .send()
            .await?;
        Ok(())
    }
    async fn get_agent_by_domain(&self, dom: &str) -> anyhow::Result<Option<AgentInfo>> {
        Ok(self
            .http_client
            .get(format!(
                "{}/api/agents/by_domain?domain={}",
                self.base_url, dom
            ))
            .send()
            .await?
            .json()
            .await?)
    }
}

#[async_trait::async_trait]
impl ToolClient for RemoteClient {
    async fn get_tools(&self) -> anyhow::Result<Vec<ToolInfo>> {
        Ok(self
            .http_client
            .get(format!("{}/api/tools", self.base_url))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn add_tool(&self, name: &str, desc: &str, lang: &str) -> anyhow::Result<i64> {
        Ok(self
            .http_client
            .post(format!("{}/api/tools", self.base_url))
            .json(&serde_json::json!({ "name": name, "description": desc, "language": lang }))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn update_tool(&self, t: ToolInfo) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/tools/update", self.base_url))
            .json(&t)
            .send()
            .await?;
        Ok(())
    }
    async fn add_tool_to_agent(&self, aid: i64, tid: i64) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/tools/link", self.base_url))
            .json(&serde_json::json!({ "agent_id": aid, "tool_id": tid }))
            .send()
            .await?;
        Ok(())
    }
    async fn remove_tool_from_agent(&self, aid: i64, tid: i64) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/tools/unlink", self.base_url))
            .json(&serde_json::json!({ "agent_id": aid, "tool_id": tid }))
            .send()
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl SupervisorClient for RemoteClient {
    async fn get_supervisors(&self) -> anyhow::Result<Vec<Supervisor>> {
        Ok(self
            .http_client
            .get(format!("{}/api/supervisors", self.base_url))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn get_supervisor_tools(&self, id: i64) -> anyhow::Result<Vec<ToolInfo>> {
        Ok(self
            .http_client
            .get(format!("{}/api/supervisors/{}/tools", self.base_url, id))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn add_supervisor(&self, name: &str, desc: &str, dom: &str) -> anyhow::Result<i64> {
        Ok(self
            .http_client
            .post(format!("{}/api/supervisors", self.base_url))
            .json(&serde_json::json!({ "name": name, "description": desc, "domain": dom }))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn update_supervisor(&self, s: Supervisor) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/supervisors/update", self.base_url))
            .json(&s)
            .send()
            .await?;
        Ok(())
    }
    async fn delete_supervisor(&self, id: i64) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/supervisors/delete", self.base_url))
            .json(&serde_json::json!({ "id": id }))
            .send()
            .await?;
        Ok(())
    }
    async fn get_supervisor_agents(&self, id: i64) -> anyhow::Result<Vec<AgentInfo>> {
        Ok(self
            .http_client
            .get(format!("{}/api/supervisors/{}/agents", self.base_url, id))
            .send()
            .await?
            .json()
            .await?)
    }
    async fn link_agent_to_supervisor(&self, sid: i64, aid: i64) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/supervisors/link", self.base_url))
            .json(&serde_json::json!({ "supervisor_id": sid, "agent_id": aid }))
            .send()
            .await?;
        Ok(())
    }
    async fn unlink_agent_from_supervisor(&self, sid: i64, aid: i64) -> anyhow::Result<()> {
        self.http_client
            .post(format!("{}/api/supervisors/unlink", self.base_url))
            .json(&serde_json::json!({ "supervisor_id": sid, "agent_id": aid }))
            .send()
            .await?;
        Ok(())
    }
}
