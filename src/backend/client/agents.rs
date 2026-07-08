use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use crate::db::Supervisor;

#[async_trait::async_trait]
pub trait AgentClient {
    async fn get_agents(&self) -> anyhow::Result<Vec<AgentInfo>>;
    async fn get_all_agents(&self) -> anyhow::Result<Vec<AgentInfo>>;
    async fn add_agent(&self, name: &str, description: &str, domain: &str) -> anyhow::Result<i64>;
    async fn update_agent(&self, agent: AgentInfo) -> anyhow::Result<()>;
    async fn get_agent_by_domain(&self, domain: &str) -> anyhow::Result<Option<AgentInfo>>;
}

#[async_trait::async_trait]
pub trait ToolClient {
    async fn get_tools(&self) -> anyhow::Result<Vec<ToolInfo>>;
    async fn add_tool(&self, name: &str, description: &str, language: &str) -> anyhow::Result<i64>;
    async fn update_tool(&self, tool: ToolInfo) -> anyhow::Result<()>;
    async fn add_tool_to_agent(&self, agent_id: i64, tool_id: i64) -> anyhow::Result<()>;
    async fn remove_tool_from_agent(&self, agent_id: i64, tool_id: i64) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait SupervisorClient {
    async fn get_supervisors(&self) -> anyhow::Result<Vec<Supervisor>>;
    async fn get_supervisor_tools(&self, supervisor_id: i64) -> anyhow::Result<Vec<ToolInfo>>;
    async fn add_supervisor(&self, name: &str, description: &str, domain: &str) -> anyhow::Result<i64>;
    async fn update_supervisor(&self, supervisor: Supervisor) -> anyhow::Result<()>;
    async fn delete_supervisor(&self, id: i64) -> anyhow::Result<()>;
    async fn get_supervisor_agents(&self, supervisor_id: i64) -> anyhow::Result<Vec<AgentInfo>>;
    async fn link_agent_to_supervisor(&self, supervisor_id: i64, agent_id: i64) -> anyhow::Result<()>;
    async fn unlink_agent_from_supervisor(&self, supervisor_id: i64, agent_id: i64) -> anyhow::Result<()>;
}
