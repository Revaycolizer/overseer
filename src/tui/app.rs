use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::backend::client::BackendClient;
use crate::backend::{Conversation, Message, SystemLoad};

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
        };
        app.refresh_conversations().await?;
        app.refresh_metrics().await?;
        app.activity.bump();
        Ok(app)
    }

    /// Returns the current list index for the selected conversation, or
    /// `None` if `selected_conv_id` is unset or not found in the list.
    pub fn selected_conv_idx(&self) -> Option<usize> {
        let id = self.selected_conv_id?;
        self.conversations.iter().position(|c| c.id == id)
    }

    pub async fn refresh_conversations(&mut self) -> anyhow::Result<()> {
        self.conversations = self.client.get_conversations(100).await?;

        // If no conversation is selected yet, default to the first one.
        if self.selected_conv_id.is_none() {
            self.selected_conv_id = self.conversations.first().map(|c| c.id);
        }

        // Load messages for whichever conversation is currently selected.
        // Because we track by ID the position of the selected conversation
        // in the list doesn't matter — even if a background title update or
        // a new-conversation insert re-sorted the list, we still load the
        // right messages.
        if let Some(id) = self.selected_conv_id {
            // Verify the ID still exists in the list (e.g. it wasn't
            // deleted from another session). If it doesn't, fall back to
            // the first conversation.
            if !self.conversations.iter().any(|c| c.id == id) {
                self.selected_conv_id = self.conversations.first().map(|c| c.id);
            }
            if let Some(active_id) = self.selected_conv_id {
                self.messages = self.client.get_messages_page(active_id, None, 100).await?;
            }
        }

        Ok(())
    }

    pub async fn refresh_metrics(&mut self) -> anyhow::Result<()> {
        if let Ok(load) = self.client.get_system_load().await {
            self.system_load = Some(load);
        }
        Ok(())
    }
}
