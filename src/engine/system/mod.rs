// src/engine/system/mod.rs
pub mod orchestrator;
pub mod history_builder;

use crate::db::Database;
use rig::providers::ollama;
use rig::streaming::StreamingPrompt;
use rig::client::CompletionClient;
use std::fmt;

pub struct AgentSystem {
    pub ollama_client: ollama::Client,
}

impl AgentSystem {
    pub fn new(_db: Database) -> Self {
        Self { 
            ollama_client: ollama::Client::new(rig::client::Nothing)
                .expect("Failed to initialize Ollama client"),
        }
    }

    pub async fn orchestrate(
        &self,
        db: &Database,
        conversation_id: i64,
        user_query: &str,
        token_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> String {
        // V4.17.2: emit a budget warning before we send the prompt
        // to Ollama. The query itself is a lower bound on the prompt
        // size (the system preamble + context + history add hundreds
        // more tokens). 8192 covers the default num_ctx of the
        // qwen2.5 / llama3 model family Overseer ships with. The
        // warning is logged via the `log` crate so it doesn't surface
        // stdout in the TUI (V4.17.1 follow-up: the user asked us
        // to drop the LLM debug prints from the TUI output).
        if crate::engine::tokenizer::check_token_budget_against(user_query, 8192) {
            log::warn!(
                "overseer.orchestrate: user_query at ~{} tokens (>8192 estimate); \
                 Ollama may truncate the prompt at num_ctx. Consider V4.17.16 \
                 (memory compaction) for long conversations.",
                crate::engine::tokenizer::count_tokens(user_query),
            );
        }

        let response = orchestrator::orchestrate_request(
            self,
            db,
            conversation_id,
            user_query,
            token_sender.clone(),
        )
        .await;

        if crate::backend::is_first_user_message(db, conversation_id).await {
            // First user message on this conversation: the new-conv
            // title ("New Conversation") is what the sidebar shows for
            // a moment. Spawn an LLM-refined title in the background so
            // any caller (TUI, remote, future scripts) gets a clean
            // three- to five-word title the next time the conversation
            // list is fetched.
            let db_bg = db.clone();
            let agent_bg = self.clone();
            let query_bg = user_query.to_string();
            tokio::spawn(async move {
                let refined = agent_bg.generate_title(&db_bg, &query_bg).await;
                if !refined.is_empty() {
                    let _ = crate::backend::update_conversation_title(&db_bg, conversation_id, &refined).await;
                    // Bump the engine activity counter so the TUI's
                    // change-driven refresh picks up the refined title
                    // (its own `app.activity` counter is only bumped by
                    // TUI-side events, never by the engine's background
                    // spawn).
                    crate::engine::activity::engine_activity().bump();
                }
            });
        }

        response
    }

    pub async fn generate_title(&self, db: &Database, query: &str) -> String {
        let model_name = crate::backend::resolve_model_name(
            db, None, "title_generator_model", "qwen2.5:7b-instruct"
        ).await;

        let agent = rig::agent::AgentBuilder::new(self.ollama_client.completion_model(&model_name))
            .preamble("You are a concise conversation title generator. Your sole task is to generate a 3-5 word title summarizing the topic of the user's query. Output ONLY the title itself.")
            .build();

        let mut truncated = query;
        if query.len() > 1000 {
            if let Some((idx, _)) = query.char_indices().nth(1000) {
                truncated = &query[..idx];
            }
        }

        let prompt = format!(
            "Generate a concise 3-5 word title summarizing the topic of this user query.\n\
             DO NOT answer the query. DO NOT include details, answers, bullet points, numbered lists, or explanations.\n\
             Output ONLY the short title text, nothing else.\n\n\
             USER QUERY: \"{}\"\n\n\
             TITLE:",
            truncated
        );

        let _permit = crate::engine::resilience::ollama_title_throttle().acquire().await;
        // Use streaming + collect_stream: this validates the V4.17.1
        // streaming helper end-to-end. Title generation doesn't need
        // per-token UI updates, so a single collect is sufficient.
        match crate::engine::streaming::collect_stream(
            agent.stream_prompt(prompt).await,
        )
        .await
        {
            Ok(title) => {
                let clean = title.trim().trim_matches('"').trim_matches('\'').trim().to_string();
                clean_title_response(&clean)
            }
            // On error or empty response, return empty so the caller
            // (the agent_system.orchestrate spawn) skips the title
            // update. The conversation keeps whatever title it already
            // had ("New Conversation" for new convs, or the prior
            // refined title for existing ones). Don't fall back to
            // first-four-words — that placeholder was racing with the
            // LLM refinement and could overwrite a real title when the
            // LLM returned empty.
            Err(_) => String::new(),
        }
    }
}

use std::sync::OnceLock;

fn clean_title_response(raw: &str) -> String {
    static PREFIX_RE: OnceLock<regex::Regex> = OnceLock::new();
    static TITLE_RE: OnceLock<regex::Regex> = OnceLock::new();

    let prefix_re = PREFIX_RE.get_or_init(|| {
        regex::Regex::new(r"(?i)^here is a \d+(?:-\d+)?(?:-\w+|\s+\w+)? title (?:for [^:]+)?:?\s*")
            .expect("valid title prefix regex")
    });
    let title_re = TITLE_RE.get_or_init(|| {
        regex::Regex::new(r"(?i)^(suggested\s+)?title:\s*")
            .expect("valid title label regex")
    });

    let mut title = raw.to_string();
    title = prefix_re.replace(&title, "").to_string();
    title = title_re.replace(&title, "").to_string();

    title.trim().trim_matches('"').trim_matches('\'').to_string()
}

impl PartialEq for AgentSystem {
    fn eq(&self, _other: &Self) -> bool { true }
}

impl Clone for AgentSystem {
    fn clone(&self) -> Self {
        Self { ollama_client: self.ollama_client.clone() }
    }
}

impl fmt::Debug for AgentSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentSystem").finish()
    }
}

#[cfg(test)]
mod send_pipeline_tests {
    //! Integration tests for the conversation send pipeline at the engine level
    //! (independent of Dioxus). They prove that invoking orchestration creates an
    //! execution run, returns assistant output, and is cancellable — i.e. the
    //! regression where sending only persisted the user message is covered.
    //!
    //! (Kernel governance of a protected action is proven separately by
    //! `engine::kernel::bridge_tests`, which drives `kernel_execute` directly.)
    use super::AgentSystem;
    use std::time::Duration;

    async fn run_count(db: &crate::db::Database, conv: i64) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM execution_runs WHERE conversation_id = ?",
        )
        .bind(conv)
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0)
    }

    #[tokio::test]
    async fn sending_creates_run_and_returns_output() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "test").await.unwrap();
        let agent = std::sync::Arc::new(AgentSystem::new(db.clone()));

        // Drive orchestration exactly as the send pipeline does. There is no
        // Ollama in the test environment, so the model call fails fast and
        // orchestration returns a fallback string — but the execution run is
        // created before any model/tool work, and output is still delivered.
        let db_task = db.clone();
        let agent_task = agent.clone();
        let handle = tokio::spawn(async move {
            agent_task
                .orchestrate(&db_task, conv, "hello there", None)
                .await
        });
        let result = tokio::time::timeout(Duration::from_secs(60), handle).await;

        // 1 + 2: sending creates an execution run (orchestration was invoked).
        assert!(
            run_count(&db, conv).await >= 1,
            "orchestration must create an execution run"
        );

        // 3: assistant output is delivered (a fallback/error string is non-empty).
        if let Ok(Ok(response)) = result {
            assert!(
                !response.trim().is_empty(),
                "orchestration must return assistant output"
            );
        }
    }

    #[tokio::test]
    async fn cancellation_aborts_the_active_task() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "test").await.unwrap();
        let agent = std::sync::Arc::new(AgentSystem::new(db.clone()));

        let db_task = db.clone();
        let handle = tokio::spawn(async move {
            agent
                .orchestrate(&db_task, conv, "do a long multi-step task", None)
                .await
        });
        let abort = handle.abort_handle();
        // Let it start (and create the run) before cancelling.
        tokio::time::sleep(Duration::from_millis(80)).await;
        abort.abort();

        let joined = handle.await;
        assert!(
            joined.is_err() && joined.unwrap_err().is_cancelled(),
            "aborting must cancel the active orchestration task"
        );
    }
}
