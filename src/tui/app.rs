use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::backend::client::BackendClient;
use crate::backend::{Conversation, Message, SystemLoad};

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ActiveTab {
    Chat,
    Metrics,
}

pub struct App {
    pub client: Arc<dyn BackendClient + Send + Sync>,
    pub conversations: Vec<Conversation>,
    pub selected_conv_idx: usize,
    pub messages: Vec<Message>,
    pub system_load: Option<SystemLoad>,
    pub active_tab: ActiveTab,
    pub input: String,
    pub status_message: String,
    pub is_orchestrating: Arc<AtomicBool>,
    pub orchestrate_error: Arc<std::sync::Mutex<Option<String>>>,
}

impl App {
    pub async fn new(client: Arc<dyn BackendClient + Send + Sync>) -> anyhow::Result<Self> {
        let mut app = Self {
            client,
            conversations: vec![],
            selected_conv_idx: 0,
            messages: vec![],
            system_load: None,
            active_tab: ActiveTab::Chat,
            input: String::new(),
            status_message: "TAB: Switch | ENTER: Send | Ctrl+N / F2: New Conv | q: Quit".to_string(),
            is_orchestrating: Arc::new(AtomicBool::new(false)),
            orchestrate_error: Arc::new(std::sync::Mutex::new(None)),
        };
        app.refresh_conversations().await?;
        app.refresh_metrics().await?;
        Ok(app)
    }

    pub async fn refresh_conversations(&mut self) -> anyhow::Result<()> {
        self.conversations = self.client.get_conversations(100).await?;
        if !self.conversations.is_empty() {
            let active_conv_id = self.conversations[self.selected_conv_idx].id;
            self.messages = self.client.get_messages_page(active_conv_id, None, 100).await?;
        }
        Ok(())
    }

    pub async fn refresh_metrics(&mut self) -> anyhow::Result<()> {
        if let Ok(load) = self.client.get_system_load().await {
            self.system_load = Some(load);
        }
        Ok(())
    }

    pub async fn send_message(&mut self) -> anyhow::Result<()> {
        if self.input.trim().is_empty() || self.conversations.is_empty() {
            return Ok(());
        }
        let active_conv_id = self.conversations[self.selected_conv_idx].id;
        let query = self.input.clone();
        self.input.clear();
        
        let _ = self.client.add_message(active_conv_id, "user", &query, false).await?;
        self.refresh_conversations().await?;
        
        self.status_message = "Orchestrating agent run...".to_string();
        self.is_orchestrating.store(true, std::sync::atomic::Ordering::SeqCst);
        
        let client = self.client.clone();
        let is_orch = self.is_orchestrating.clone();
        let err_clone = self.orchestrate_error.clone();
        *err_clone.lock().unwrap() = None;

        tokio::spawn(async move {
            match client.orchestrate(active_conv_id, &query).await {
                Ok(response) => {
                    let _ = client.add_message(active_conv_id, "assistant", &response, false).await;
                }
                Err(e) => {
                    *err_clone.lock().unwrap() = Some(e.to_string());
                }
            }
            is_orch.store(false, std::sync::atomic::Ordering::SeqCst);
        });
        
        Ok(())
    }
}
