// src/engine/system/mod.rs
pub mod orchestrator;
pub mod history_builder;

use crate::db::Database;
use rig::providers::ollama;
use rig::completion::Prompt;
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
    ) -> String {
        let query_tokens = crate::engine::tokenizer::count_tokens(user_query);
        println!(
            "📊 [orchestrate] conv={} query_tokens=~{}",
            conversation_id, query_tokens
        );
        let response = orchestrator::orchestrate_request(self, db, conversation_id, user_query).await;
        println!(
            "📊 [orchestrate] conv={} response_tokens=~{}",
            conversation_id,
            crate::engine::tokenizer::count_tokens(&response)
        );

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
        match agent.prompt(&prompt).await {
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
