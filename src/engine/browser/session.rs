use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::process::Child;
use super::cdp::CdpConnection;
use super::recorder::FrameRecorder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Active,
    Idle,
    Suspended,
    Closed,
}

pub struct BrowserSession {
    pub id: i64,
    pub port: u16,
    pub profile_path: PathBuf,
    pub state: SessionState,
    pub process: Option<Child>,
    pub cdp_ws_url: String,
    pub cdp: Option<Arc<CdpConnection>>,
    pub recorder: Option<FrameRecorder>,
    pub action_count: Arc<AtomicU32>,
    pub navigation_depth: Arc<AtomicU32>,
    pub created_at: chrono::NaiveDateTime,
    pub conversation_id: Option<i64>,
}

impl BrowserSession {
    pub fn new(id: i64, port: u16, profile_path: PathBuf, process: Option<Child>, cdp_ws_url: String) -> Self {
        Self {
            id,
            port,
            profile_path,
            state: SessionState::Active,
            process,
            cdp_ws_url,
            cdp: None,
            recorder: None,
            action_count: Arc::new(AtomicU32::new(0)),
            navigation_depth: Arc::new(AtomicU32::new(0)),
            created_at: chrono::Utc::now().naive_utc(),
            conversation_id: None,
        }
    }
}
