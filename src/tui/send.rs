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
        if self.input.trim().is_empty() || self.selected_conv_id.is_none() {
            return Ok(());
        }
        let active_conv_id = self.selected_conv_id.unwrap();
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
            // Snapshot the engine-activity counter before orchestrate
            // starts so we can detect when the LLM-refined title fires
            // its own bump.
            let initial_engine = crate::engine::activity::engine_activity().current();

            // V4.17.1e: per-token streaming for the TUI. Create a
            // channel, pass the sender into orchestrate, and bump
            // engine_activity on each token so the main loop's
            // change-driven refresh sees live progress. (The TUI
            // doesn't render the partial message body — that's
            // V4.17.1d's Dioxus path — but per-bump UI updates let
            // the TUI show a "typing…" indicator or other live state.)
            let (token_tx, mut token_rx) =
                tokio::sync::mpsc::unbounded_channel::<String>();
            let activity_for_tokens = activity.clone();
            let _consumer = tokio::spawn(async move {
                while let Some(_) = token_rx.recv().await {
                    activity_for_tokens.bump();
                    crate::engine::activity::engine_activity().bump();
                }
            });

            match client.orchestrate(active_conv_id, &query, Some(token_tx)).await {
                Ok(response) => {
                    let _ = client.add_message(active_conv_id, "assistant", &response, false).await;
                    activity.bump();

                    // Wait for the LLM-refined title to land. The
                    // spawn of `agent_bg.generate_title(...)` runs
                    // inside agent_system::orchestrate's LLM refinement
                    // block and bumps engine_activity once it has
                    // written the refined title. Polling every 250ms
                    // caps the worst-case latency at the LLM time
                    // itself (~1-3 s) rather than the previous 1-5 s
                    // post-orchestrate poll window.
                    for _ in 0..20u32 {
                        if crate::engine::activity::engine_activity().current() != initial_engine {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    }
                    // Backup activity bump in case the main loop
                    // missed the engine_activity change.
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