use std::sync::atomic::Ordering;

use super::app::App;

impl App {
    /// Sends the buffered input as a user message, writes the
    /// optimistic first-four-words title to the DB so the sidebar
    /// updates immediately, and spawns the orchestrate task in the
    /// background. The orchestrator's own LLM-refined title write
    /// bumps the engine activity counter so the main loop picks it up
    /// (see `crate::engine::activity`).
    pub async fn send_message(&mut self) -> anyhow::Result<()> {
        if self.input.trim().is_empty() || self.conversations.is_empty() {
            return Ok(());
        }
        let active_conv_id = self.conversations[self.selected_conv_idx].id;
        let query = self.input.clone();
        self.input.clear();

        let _ = self.client.add_message(active_conv_id, "user", &query, false).await?;
        self.activity.bump();

        // Mirror the Dioxus chat_input optimistic-title pattern: write
        // the first-four-words to the DB so the sidebar updates the
        // title immediately on the next refresh, before the agent
        // starts streaming its response. The LLM-refined title still
        // lands later via the agent_system.orchestrate spawn.
        let optimistic_title: String = query
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        if !optimistic_title.is_empty() {
            let _ = self.client.update_conversation_title(active_conv_id, &optimistic_title).await;
            self.activity.bump();
        }

        self.refresh_conversations().await?;

        self.status_message = "Orchestrating agent run...".to_string();
        self.is_orchestrating.store(true, Ordering::SeqCst);

        let client = self.client.clone();
        let is_orch = self.is_orchestrating.clone();
        let err_clone = self.orchestrate_error.clone();
        let activity = self.activity.clone();
        *err_clone.lock().unwrap() = None;

        tokio::spawn(async move {
            match client.orchestrate(active_conv_id, &query).await {
                Ok(response) => {
                    let _ = client.add_message(active_conv_id, "assistant", &response, false).await;
                    activity.bump();
                }
                Err(e) => {
                    *err_clone.lock().unwrap() = Some(e.to_string());
                }
            }
            is_orch.store(false, Ordering::SeqCst);
        });

        Ok(())
    }
}