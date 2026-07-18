use super::action_types::ActionPolicy;
use super::audit_log::AuditLog;
use super::pool::BrowserPool;

impl BrowserPool {
    pub async fn set_policy(&self, id: i64, policy: ActionPolicy) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        let session = inner.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("Session {id} not found"))?;
        session.policy = policy;
        Ok(())
    }

    pub async fn get_audit_log(&self, id: i64) -> anyhow::Result<Vec<serde_json::Value>> {
        let inner = self.inner.lock().await;
        let session = inner.sessions.get(&id).ok_or_else(|| anyhow::anyhow!("Session {id} not found"))?;
        let path = session.profile_path.join("audit_log.jsonl");
        drop(inner);
        let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let entries: Vec<serde_json::Value> = content.lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(entries)
    }
}
