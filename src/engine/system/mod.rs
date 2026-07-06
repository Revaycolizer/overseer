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
        orchestrator::orchestrate_request(self, db, conversation_id, user_query).await
    }

    pub async fn generate_title(&self, db: &Database, query: &str) -> String {
        let model_name = crate::backend::resolve_model_name(
            db, None, "title_generator_model", "qwen2.5:7b-instruct"
        ).await;

        let agent = rig::agent::AgentBuilder::new(self.ollama_client.completion_model(&model_name))
            .preamble("You are a concise title generator. Summarize the user's query into a 3-5 word title. No emojis. Output ONLY the title text. Do NOT include introductory phrases, quotation marks, conversational explanations, or punctuation.")
            .build();

        match agent.prompt(query).await {
            Ok(title) => {
                let clean = title.trim().trim_matches('"').trim_matches('\'').trim().to_string();
                clean_title_response(&clean)
            }
            Err(_) => query.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
        }
    }
}

fn clean_title_response(raw: &str) -> String {
    let mut title = raw.to_string();
    
    // Regex matching "Here is a 4 word/4-word title for [your query]:"
    if let Ok(re) = regex::Regex::new(r"(?i)^here is a \d+(?:-\d+)?(?:-\w+|\s+\w+)? title (?:for [^:]+)?:?\s*") {
        title = re.replace(&title, "").to_string();
    }
    
    // Regex matching "Title: " or "Suggested Title: "
    if let Ok(re) = regex::Regex::new(r"(?i)^(suggested\s+)?title:\s*") {
        title = re.replace(&title, "").to_string();
    }
    
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
