// src/db/seed_core.rs — seed_all, ensure_core_registry_seeded, ensure_setting_defaults
use super::Database;

impl Database {
    pub(super) async fn seed_all(&self) -> anyhow::Result<()> {
        //println!("🌱 Initializing v10 Database Environment...");
        self.ensure_core_registry_seeded().await?;
        self.ensure_setting_defaults().await?;
        self.backfill_ids().await?;

        #[cfg(feature = "memory")]
        {
            //println!("🧠 Syncing agents to vector database for semantic discovery...");
            let client = reqwest::Client::new();
            if let Err(e) = self.sync_agents_to_vector_db(&client).await {
                //eprintln!("⚠️ Failed to sync agents to vector DB: {}", e);
            }
        }

        Ok(())
    }

    async fn ensure_core_registry_seeded(&self) -> anyhow::Result<()> {
        let agents_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await?;
        if agents_count.0 == 0 {
            self.seed_agents().await?;
        }

        let tools_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tools")
            .fetch_one(&self.pool)
            .await?;
        if tools_count.0 == 0 {
            self.seed_tools().await?;
        }

        let agent_tools_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_tools")
            .fetch_one(&self.pool)
            .await?;
        if agent_tools_count.0 == 0 {
            self.seed_agent_tools().await?;
        }

        let supervisors_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM supervisors")
            .fetch_one(&self.pool)
            .await?;
        if supervisors_count.0 == 0 {
            self.seed_supervisors().await?;
        }

        let planners_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM planners")
            .fetch_one(&self.pool)
            .await?;
        if planners_count.0 == 0 {
            self.seed_planners().await?;
        }

        Ok(())
    }

    async fn ensure_setting_defaults(&self) -> anyhow::Result<()> {
        let default_model = crate::backend::get_available_models()
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
        let defaults = [
            ("title_generator_model", default_model.as_str()),
            ("default_chat_model", default_model.as_str()),
            ("max_iterations", "10"),
            ("tool_timeout_ms", "45000"),
            ("model_timeout_ms", "90000"),
            ("computer_supervision_mode", "approval_required"),
            ("computer_supervision_max_actions", "25"),
            ("computer_supervision_session_timeout_ms", "600000"),
            ("computer_supervision_allowlist", ""),
            ("computer_session_state", "idle"),
            (
                "computer_session_note",
                "No supervised computer session is active.",
            ),
            ("computer_approval_state", "idle"),
            ("computer_pending_tool", ""),
            ("computer_pending_action", ""),
            ("computer_pending_domain", ""),
            ("computer_session_action_count", "0"),
            ("computer_session_started_at", ""),
            (
                "artifact_routing_hint",
                "[ARTIFACTS ATTACHED] The user has selected local file artifacts as context for this request. Route to 'chat' or 'vision' domain so the agent can answer from the attached content. Do NOT route to 'stealth' (search) unless the user explicitly asks to search the web. The attached artifacts contain the information needed to answer.",
            ),
            (
                "artifact_context_header",
                "ATTACHED ARTIFACTS (the user explicitly selected these as context — use them to answer):",
            ),
            ("artifact_default_domain", "chat"),
            ("artifact_media_domain", "computer"),
            ("runtime_registry_status", "unknown"),
            ("runtime_registry_refreshed_at", ""),
            ("runtime_registry_error", ""),
        ];

        for (key, value) in defaults {
            let id = self.next_id().await;
            sqlx::query(
                "INSERT INTO settings (id, key, value) VALUES (?, ?, ?) \
                 ON CONFLICT(key) DO NOTHING",
            )
            .bind(id)
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        }
        // Note: ensure_ system functions have been consolidated into main seed_ functions for v10.
        Ok(())
    }
}
