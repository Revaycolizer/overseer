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
        let response = orchestrator::orchestrate_request(self, db, conversation_id, user_query).await;

        if crate::backend::maybe_set_initial_title(db, conversation_id, user_query).await.is_some() {
            // First user message on this conversation: the optimistic
            // first-four-words title was just written. Spawn an LLM-
            // refined title in the background so any caller (TUI, remote,
            // future scripts) gets a clean three- to five-word title the
            // next time the conversation list is fetched. The Dioxus
            // chat_input path runs its own refinement via a channel; the
            // worst case is two API calls with the second no-op writing
            // the same string back.
            let db_bg = db.clone();
            let agent_bg = self.clone();
            let query_bg = user_query.to_string();
            tokio::spawn(async move {
                let refined = agent_bg.generate_title(&db_bg, &query_bg).await;
                if !refined.is_empty() {
                    let _ = crate::backend::update_conversation_title(&db_bg, conversation_id, &refined).await;
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

        let _permit = crate::engine::resilience::ollama_throttle().acquire().await;
        match agent.prompt(&prompt).await {
            Ok(title) => {
                let clean = title.trim().trim_matches('"').trim_matches('\'').trim().to_string();
                clean_title_response(&clean)
            }
            Err(_) => query.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
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
