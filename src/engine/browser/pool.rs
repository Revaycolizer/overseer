use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::memory_monitor;
use super::pool_config::PoolConfig;
use super::port_alloc;
use super::session::BrowserSession;
use super::session_init;
use super::spawner;

pub(super) struct Inner {
    pub(super) sessions: HashMap<i64, BrowserSession>,
    pub(super) next_id: i64,
}

pub struct BrowserPool {
    config: PoolConfig,
    pub(super) inner: Arc<Mutex<Inner>>,
    data_dir: PathBuf,
}

impl BrowserPool {
    pub fn new(config: PoolConfig, data_dir: PathBuf) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(Inner {
                sessions: HashMap::new(),
                next_id: 1,
            })),
            data_dir,
        }
    }

    pub async fn create_session(&self, _conversation_id: Option<i64>) -> anyhow::Result<i64> {
        let mut inner = self.inner.lock().await;
        if inner.sessions.len() >= self.config.max_sessions {
            anyhow::bail!(
                "Max browser sessions ({}) reached",
                self.config.max_sessions
            );
        }
        let count = inner.sessions.len();
        let report = memory_monitor::current_report(count as u64 * 90, count);
        if !report.can_create_session {
            anyhow::bail!("Memory too low to create browser session");
        }

        let id = inner.next_id;
        inner.next_id += 1;
        let port = port_alloc::allocate_port();
        let profile_path = self.data_dir.join("browser-profiles").join(id.to_string());

        let (process, cdp_ws_url) =
            spawner::spawn_obscura(port, &profile_path, self.config.stealth).await?;

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

    pub async fn get_profile_path(&self, id: i64) -> Option<std::path::PathBuf> {
        self.inner
            .lock()
            .await
            .sessions
            .get(&id)
            .map(|s| s.profile_path.clone())
    }

    pub async fn shutdown(&self) {
        let mut inner = self.inner.lock().await;
        for (_, session) in inner.sessions.drain() {
            if let Some(ref cdp) = session.cdp {
                let _ = cdp.call("Page.stopScreencast", serde_json::json!({})).await;
                let _ = cdp.call("Browser.close", serde_json::json!({})).await;
            }
            if let Some(mut child) = session.process {
                let _ = child.kill().await;
            }
        }
    }

    pub async fn session_count(&self) -> usize {
        self.inner.lock().await.sessions.len()
    }
}
