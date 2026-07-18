use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::Value;
use tokio::sync::Mutex;

use super::port_alloc;
use super::session::{BrowserSession, SessionState};
use super::session_init;
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

    pub async fn create_session(&self, _conversation_id: Option<i64>) -> anyhow::Result<i64> {
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
        session_init::init_session(&mut session).await;
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

    pub async fn session_count(&self) -> usize {
        self.inner.lock().await.sessions.len()
    }
}
