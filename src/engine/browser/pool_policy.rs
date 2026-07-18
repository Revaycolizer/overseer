use super::action_types::ActionPolicy;
use super::pool::BrowserPool;
use super::session::SessionState;

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

    pub async fn hitl_pause(&self, id: i64, reason: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        let session = inner.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("Session {id} not found"))?;
        session.state = SessionState::Paused;
        if let Some(ref cdp) = session.cdp {
            let _ = cdp.call("Page.stopScreencast", serde_json::json!({})).await;
        }
        tracing::info!(?id, ?reason, "Browser session paused for HITL");
        Ok(())
    }

    pub async fn hitl_resume(&self, id: i64) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        let session = inner.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("Session {id} not found"))?;
        session.state = SessionState::Active;
        tracing::info!(?id, "Browser session resumed after HITL");
        Ok(())
    }
}
