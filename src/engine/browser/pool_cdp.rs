use serde_json::Value;
use super::pool::BrowserPool;

impl BrowserPool {
    pub async fn get_cdp_url(&self, session_id: i64) -> Option<String> {
        self.inner.lock().await.sessions.get(&session_id).map(|s| s.cdp_ws_url.clone())
    }

    pub async fn cdp_call(&self, session_id: i64, method: &str, params: Value) -> anyhow::Result<Value> {
        let inner = self.inner.lock().await;
        if let Some(s) = inner.sessions.get(&session_id) {
            if let Some(ref cdp) = s.cdp {
                return cdp.call(method, params).await;
            }
            drop(inner);
            return super::cdp::cdp_call(
                &format!("ws://127.0.0.1:{}/devtools/browser", 9300 + session_id as u16),
                method, params,
            ).await;
        }
        anyhow::bail!("Session {session_id} not found")
    }
}
