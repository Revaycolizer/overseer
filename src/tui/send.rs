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

        // No 4-word optimistic title write: the LLM refinement via
        // agent_system.orchestrate writes the proper title within
        // 1-3 s. The 4-word placeholder was racing with the LLM and
        // sometimes overwriting the refined title when the LLM
        // returned empty / errored.
        self.refresh_conversations().await?;

        self.status_message = "Orchestrating agent run...".to_string();
        self.is_orchestrating.store(true, Ordering::SeqCst);

        let client = self.client.clone();
        let is_orch = self.is_orchestrating.clone();
        let err_clone = self.orchestrate_error.clone();
        let activity = self.activity.clone();
        let last_orch_finished = self.last_orchestrate_finished.clone();
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
            *last_orch_finished.lock().unwrap() = Some(std::time::Instant::now());
        });

        Ok(())
    }
}