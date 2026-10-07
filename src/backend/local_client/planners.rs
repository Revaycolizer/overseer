use super::LocalClient;
use crate::backend::client::*;
use crate::backend::*;

#[async_trait::async_trait]
impl PlannerClient for LocalClient {
    async fn get_planners(&self) -> anyhow::Result<Vec<Planner>> {
        crate::backend::get_planners(&self.db).await
    }
    async fn add_planner(&self, n: &str, i: &str, m: &str) -> anyhow::Result<i64> {
        crate::backend::add_planner(&self.db, n, i, m).await
    }
    async fn update_planner(&self, p: Planner) -> anyhow::Result<()> {
        crate::backend::update_planner(&self.db, p).await
    }
    async fn delete_planner(&self, id: i64) -> anyhow::Result<()> {
        crate::backend::delete_planner(&self.db, id).await
    }
}

#[async_trait::async_trait]
impl VersionClient for LocalClient {
    async fn publish_version(&self, t: &str, id: i64, tag: &str, d: &str) -> anyhow::Result<()> {
        crate::backend::publish_version(&self.db, t, id, tag, d, "").await
    }
    async fn publish_agent_version(&self, id: i64, tag: &str, d: &str) -> anyhow::Result<()> {
        crate::backend::publish_agent_version(&self.db, id, tag, d).await
    }
    async fn publish_tool_version(&self, id: i64, tag: &str, d: &str) -> anyhow::Result<()> {
        crate::backend::publish_tool_version(&self.db, id, tag, d).await
    }
    async fn rollback_version(&self, t: &str, id: i64, tag: &str) -> anyhow::Result<()> {
        crate::backend::rollback_version(&self.db, t, id, tag).await
    }
    async fn get_version_content(&self, t: &str, id: i64, tag: &str) -> anyhow::Result<String> {
        let opt = crate::backend::get_version_content(&self.db, t, id, tag).await?;
        Ok(opt.unwrap_or_default())
    }
    async fn get_version_history(&self, t: &str, id: i64) -> anyhow::Result<Vec<EntityVersion>> {
        crate::backend::get_version_history(&self.db, t, id).await
    }
}
