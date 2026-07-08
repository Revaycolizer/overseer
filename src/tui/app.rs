use std::sync::Arc;
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
            status_message: "Press TAB to switch, ENTER to send message, q to quit".to_string(),
        };
        app.refresh_conversations().await?;
        app.refresh_metrics().await?;
        Ok(app)
    }

    pub async fn refresh_conversations(&mut self) -> anyhow::Result<()> {
        self.conversations = self.client.get_conversations(100).await?;
        if !self.conversations.is_empty() {
            let active_conv_id = self.conversations[self.selected_conv_idx].id;
            self.messages = self.client.get_messages(active_conv_id).await?;
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
        self.status_message = "Sending and orchestrating agent run...".to_string();
        
        let _ = self.client.add_message(active_conv_id, "user", &query, false).await?;
        let _ = self.client.orchestrate(active_conv_id, &query).await;
        self.refresh_conversations().await?;
        self.status_message = "Message sent successfully.".to_string();
        Ok(())
    }
}
