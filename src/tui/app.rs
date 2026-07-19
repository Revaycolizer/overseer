use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::backend::client::BackendClient;
use crate::backend::{Conversation, Message, SystemLoad};

use crate::engine::utils::discovery::AgentInfo;
pub use super::activity::ActivityCounter;
pub use super::tab::ActiveTab;

pub struct App {
    pub client: Arc<dyn BackendClient + Send + Sync>,
    pub conversations: Vec<Conversation>,
    /// The database ID of the currently selected conversation.
    /// Tracking by ID (not by list index) means that background
    /// `refresh_conversations` calls never accidentally drift the
    /// selection to a different conversation just because the list
    /// was re-fetched and positions shifted.
    pub selected_conv_id: Option<i64>,
    pub messages: Vec<Message>,
    pub system_load: Option<SystemLoad>,
    pub active_tab: ActiveTab,
    pub input: String,
    /// Cursor position in `input` (UTF-8 byte index). Backspace
    /// deletes the char before this index; Left/Right arrows move
    /// it. Insertion is at this index. Clamped to `input.len()`.
    pub input_cursor: usize,
    pub status_message: String,
    pub is_orchestrating: Arc<AtomicBool>,
    pub orchestrate_error: Arc<std::sync::Mutex<Option<String>>>,
    pub activity: Arc<ActivityCounter>,
    pub last_orchestrate_finished: Arc<std::sync::Mutex<Option<Instant>>>,
    /// V4.17.14: handle to the in-flight orchestrate tokio task.
    /// The handlers check this to wire Esc / a Stop key to abort the
    /// current LLM stream (drops the futures, which closes the
    /// underlying HTTP request to Ollama).
    pub current_orchestrate: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub search_results: Option<Vec<crate::backend::ConversationSearchResult>>,
    pub search_query: String,

    // Phase 2: Management panel state
    pub agents: Vec<AgentInfo>,
    pub selected_agent_id: Option<i64>,
}

impl App {
    pub async fn new(client: Arc<dyn BackendClient + Send + Sync>) -> anyhow::Result<Self> {
        let mut app = Self {
            client,
            conversations: vec![],
            selected_conv_id: None,
            messages: vec![],
            system_load: None,
            active_tab: ActiveTab::Chat,
            input: String::new(),
            input_cursor: 0,
            status_message: "TAB: Switch | ENTER: Send | Ctrl+N / F2: New Conv | q: Quit".to_string(),
            is_orchestrating: Arc::new(AtomicBool::new(false)),
            orchestrate_error: Arc::new(std::sync::Mutex::new(None)),
            activity: Arc::new(ActivityCounter::default()),
            last_orchestrate_finished: Arc::new(std::sync::Mutex::new(None)),
            current_orchestrate: Arc::new(std::sync::Mutex::new(None)),
            search_results: None,
            search_query: String::new(),
            agents: vec![],
            selected_agent_id: None,
        };
        app.refresh_conversations().await?;
        app.refresh_metrics().await?;
        app.activity.bump();
        Ok(app)
    }

    pub fn selected_conv_idx(&self) -> Option<usize> {
        let id = self.selected_conv_id?;
        self.conversations.iter().position(|c| c.id == id)
    }

    pub fn cancel_current_orchestrate(&self) {
        if let Some(handle) = self.current_orchestrate.lock().unwrap().take() {
            handle.abort();
        }
    }
}
