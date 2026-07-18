use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::Mutex;

use super::cdp::CdpConnection;
use super::port_alloc;
use super::session::{BrowserSession, SessionState};
use super::spawner;

pub struct PoolConfig {
    pub max_sessions: usize,
    pub stealth: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self { max_sessions: 4, stealth: false }
    }
}

struct Inner {
    sessions: HashMap<i64, BrowserSession>,
    next_id: i64,
}

pub struct BrowserPool {
    config: PoolConfig,
    inner: Mutex<Inner>,
    data_dir: PathBuf,
}

impl BrowserPool {
    pub fn new(config: PoolConfig, data_dir: PathBuf) -> Self {
        Self { config, inner: Mutex::new(Inner { sessions: HashMap::new(), next_id: 1 }), data_dir }
    }

    pub async fn create_session(&self, conversation_id: Option<i64>) -> anyhow::Result<i64> {
        let mut inner = self.inner.lock().await;
        if inner.sessions.len() >= self.config.max_sessions {
            anyhow::bail!("Max browser sessions ({}) reached", self.config.max_sessions);
        }

        let id = inner.next_id;
        inner.next_id += 1;
        let port = port_alloc::allocate_port();
        let profile_path = self.data_dir.join("browser-profiles").join(id.to_string());

        let (process, cdp_ws_url) = spawner::spawn_obscura(port, &profile_path, self.config.stealth).await?;

        let mut session = BrowserSession::new(id, port, profile_path, Some(process), cdp_ws_url);

        // Establish persistent CDP connection
        match CdpConnection::connect(&session.cdp_ws_url).await {
            Ok(cdp) => {
                let cdp = Arc::new(cdp);
                cdp.clone().start_heartbeat();
                // Background init: block resources + randomize viewport
                let bg_cdp = cdp.clone();
                let sid = id;
                tokio::spawn(async move {
                    let _ = bg_cdp.call("Network.enable", serde_json::json!({})).await;
                    let _ = bg_cdp.call("Network.setBlockedURLs", serde_json::json!({
                        "urls": [
                            "*.png", "*.jpg", "*.gif", "*.svg", "*.ico",
                            "*.woff", "*.woff2", "*.ttf", "*.eot",
                            "*.mp4", "*.mp3", "*.webm",
                            "*/analytics/*", "*/tracking/*", "*/ads/*", "*/beacon/*",
                        ]
                    })).await;
                    let width = 1280u32 + (sid as u8 as u32 % 86);
                    let height = 800u32 + ((sid >> 8) as u8 as u32 % 68);
                    let _ = bg_cdp.call("Emulation.setDeviceMetricsOverride", serde_json::json!({
                        "width": width, "height": height,
                        "deviceScaleFactor": 1, "mobile": false,
                    })).await;
                });
                session.cdp = Some(cdp);
            }
            Err(e) => tracing::warn!(?e, "Failed to connect CDP for session {id}"),
        }

        inner.sessions.insert(id, session);

        Ok(id)
    }

    pub async fn session_exists(&self, id: i64) -> bool {
        self.inner.lock().await.sessions.contains_key(&id)
    }

    pub async fn close_session(&self, id: i64) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        if let Some(mut session) = inner.sessions.remove(&id) {
            drop(session.cdp.take());
            if let Some(mut child) = session.process.take() {
                let _ = child.kill().await;
            }
        }
        Ok(())
    }

    pub async fn cdp_call(&self, session_id: i64, method: &str, params: Value) -> anyhow::Result<Value> {
        let inner = self.inner.lock().await;
        if let Some(s) = inner.sessions.get(&session_id) {
            if let Some(ref cdp) = s.cdp {
                return cdp.call(method, params).await;
            }
            // No persistent connection: fall back to one-shot
            drop(inner);
            return super::cdp::cdp_call(
                &format!("ws://127.0.0.1:{}/devtools/browser", 9300 + session_id as u16),
                method, params,
            ).await;
        }
        anyhow::bail!("Session {session_id} not found")
    }

    pub async fn session_count(&self) -> usize {
        self.inner.lock().await.sessions.len()
    }
}
