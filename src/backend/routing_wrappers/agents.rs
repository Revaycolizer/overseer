use crate::db::{Database, Supervisor};
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use crate::backend::client::{AgentClient, ToolClient, SupervisorClient};

pub async fn get_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    if db.is_remote() { return db.remote_client().get_agents().await; }
    crate::backend::agents::get_agents(db).await
}

pub async fn get_all_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    if db.is_remote() { return db.remote_client().get_all_agents().await; }
    crate::backend::agents::get_all_agents(db).await
}

pub async fn add_agent(db: &Database, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
    if db.is_remote() { return db.remote_client().add_agent(name, description, domain).await; }
    crate::backend::agents::add_agent(db, name, description, domain).await
}

pub async fn update_agent(db: &Database, agent: AgentInfo) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().update_agent(agent).await; }
    crate::backend::agents::update_agent(db, agent).await
}

pub async fn get_agent_by_domain(db: &Database, domain: &str) -> anyhow::Result<Option<AgentInfo>> {
    if db.is_remote() { return db.remote_client().get_agent_by_domain(domain).await; }
    crate::backend::agents::get_agent_by_domain(db, domain).await
}

pub async fn get_tools(db: &Database) -> anyhow::Result<Vec<ToolInfo>> {
    if db.is_remote() { return db.remote_client().get_tools().await; }
    crate::backend::tools::get_tools(db).await
}

pub async fn add_tool(db: &Database, name: &str, description: &str, language: &str) -> anyhow::Result<i64> {
    if db.is_remote() { return db.remote_client().add_tool(name, description, language).await; }
    crate::backend::tools::add_tool(db, name, description, language).await
}

pub async fn update_tool(db: &Database, tool: ToolInfo) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().update_tool(tool).await; }
    crate::backend::tools::update_tool(db, tool).await
}

pub async fn add_tool_to_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().add_tool_to_agent(agent_id, tool_id).await; }
    crate::backend::tools::add_tool_to_agent(db, agent_id, tool_id).await
}

pub async fn remove_tool_from_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().remove_tool_from_agent(agent_id, tool_id).await; }
    crate::backend::tools::remove_tool_from_agent(db, agent_id, tool_id).await
}

pub async fn get_supervisors(db: &Database) -> anyhow::Result<Vec<Supervisor>> {
    if db.is_remote() { return db.remote_client().get_supervisors().await; }
    crate::backend::supervisors::get_supervisors(db).await
}

pub async fn get_supervisor_tools(db: &Database, supervisor_id: i64) -> anyhow::Result<Vec<ToolInfo>> {
    if db.is_remote() { return db.remote_client().get_supervisor_tools(supervisor_id).await; }
    crate::backend::supervisors::get_supervisor_tools(db, supervisor_id).await
}

pub async fn add_supervisor(db: &Database, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
    if db.is_remote() { return db.remote_client().add_supervisor(name, description, domain).await; }
    crate::backend::supervisors::add_supervisor(db, name, description, domain).await
}

pub async fn update_supervisor(db: &Database, supervisor: Supervisor) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().update_supervisor(supervisor).await; }
    crate::backend::supervisors::update_supervisor(db, supervisor).await
}

pub async fn delete_supervisor(db: &Database, id: i64) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().delete_supervisor(id).await; }
    crate::backend::supervisors::delete_supervisor(db, id).await
}

pub async fn get_supervisor_agents(db: &Database, supervisor_id: i64) -> anyhow::Result<Vec<AgentInfo>> {
    if db.is_remote() { return db.remote_client().get_supervisor_agents(supervisor_id).await; }
    crate::backend::supervisors::get_supervisor_agents(db, supervisor_id).await
}

pub async fn link_agent_to_supervisor(db: &Database, supervisor_id: i64, agent_id: i64) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().link_agent_to_supervisor(supervisor_id, agent_id).await; }
    crate::backend::supervisors::link_agent_to_supervisor(db, supervisor_id, agent_id).await
}

pub async fn unlink_agent_from_supervisor(db: &Database, supervisor_id: i64, agent_id: i64) -> anyhow::Result<()> {
    if db.is_remote() { return db.remote_client().unlink_agent_from_supervisor(supervisor_id, agent_id).await; }
    crate::backend::supervisors::unlink_agent_from_supervisor(db, supervisor_id, agent_id).await
}
