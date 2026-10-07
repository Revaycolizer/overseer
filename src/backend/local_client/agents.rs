use super::LocalClient;
use crate::backend::client::{AgentClient, SupervisorClient, ToolClient};
use crate::db::Supervisor;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};

#[async_trait::async_trait]
impl AgentClient for LocalClient {
    async fn get_agents(&self) -> anyhow::Result<Vec<AgentInfo>> {
        crate::backend::get_agents(&self.db).await
    }
    async fn get_all_agents(&self) -> anyhow::Result<Vec<AgentInfo>> {
        crate::backend::get_all_agents(&self.db).await
    }
    async fn add_agent(&self, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
        crate::backend::add_agent(&self.db, name, description, domain).await
    }
    async fn update_agent(&self, agent: AgentInfo) -> anyhow::Result<()> {
        crate::backend::update_agent(&self.db, agent).await
    }
    async fn get_agent_by_domain(&self, domain: &str) -> anyhow::Result<Option<AgentInfo>> {
        crate::backend::get_agent_by_domain(&self.db, domain).await
    }
}

#[async_trait::async_trait]
impl ToolClient for LocalClient {
    async fn get_tools(&self) -> anyhow::Result<Vec<ToolInfo>> {
        crate::backend::get_tools(&self.db).await
    }
    async fn add_tool(&self, name: &str, description: &str, language: &str) -> anyhow::Result<i64> {
        crate::backend::add_tool(&self.db, name, description, language).await
    }
    async fn update_tool(&self, tool: ToolInfo) -> anyhow::Result<()> {
        crate::backend::update_tool(&self.db, tool).await
    }
    async fn add_tool_to_agent(&self, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
        crate::backend::add_tool_to_agent(&self.db, agent_id, tool_id).await
    }
    async fn remove_tool_from_agent(&self, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
        crate::backend::remove_tool_from_agent(&self.db, agent_id, tool_id).await
    }
}

#[async_trait::async_trait]
impl SupervisorClient for LocalClient {
    async fn get_supervisors(&self) -> anyhow::Result<Vec<Supervisor>> {
        crate::backend::get_supervisors(&self.db).await
    }
    async fn get_supervisor_tools(&self, id: i64) -> anyhow::Result<Vec<ToolInfo>> {
        crate::backend::get_supervisor_tools(&self.db, id).await
    }
    async fn add_supervisor(&self, name: &str, desc: &str, domain: &str) -> anyhow::Result<i64> {
        crate::backend::add_supervisor(&self.db, name, desc, domain).await
    }
    async fn update_supervisor(&self, s: Supervisor) -> anyhow::Result<()> {
        crate::backend::update_supervisor(&self.db, s).await
    }
    async fn delete_supervisor(&self, id: i64) -> anyhow::Result<()> {
        crate::backend::delete_supervisor(&self.db, id).await
    }
    async fn get_supervisor_agents(&self, id: i64) -> anyhow::Result<Vec<AgentInfo>> {
        crate::backend::get_supervisor_agents(&self.db, id).await
    }
    async fn link_agent_to_supervisor(&self, sid: i64, aid: i64) -> anyhow::Result<()> {
        crate::backend::link_agent_to_supervisor(&self.db, sid, aid).await
    }
    async fn unlink_agent_from_supervisor(&self, sid: i64, aid: i64) -> anyhow::Result<()> {
        crate::backend::unlink_agent_from_supervisor(&self.db, sid, aid).await
    }
}
