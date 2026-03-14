use rig::providers::ollama;
use rig::client::{Nothing, CompletionClient};
use rig::completion::Prompt;
use std::sync::Arc;
use std::collections::HashMap;

use crate::engine::agents::master_planner::MasterPlanner;
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::orchestrator::orchestrate_request;
use crate::db::Database;

#[derive(Clone)]
pub struct AgentSystem {
    pub ollama_client: ollama::Client,
    pub perception_tool: StructuredPerceptionTool,
}

impl PartialEq for AgentSystem {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl AgentSystem {
    pub fn new(db: crate::db::Database) -> Self {
        let client = ollama::Client::new(Nothing).expect("Failed to create Ollama client");
        
        Self {
            ollama_client: client,
            perception_tool: StructuredPerceptionTool::new(db),
        }
    }

    pub async fn orchestrate(&self, db: &Database, conversation_id: i64, query: &str) -> String {
        // Refresh transient runtime environment for tools
        if let Err(e) = crate::engine::utils::discovery::prepare_runtime_env(db).await {
            println!("⚠️  WARNING: Failed to prepare tool runtime env: {}", e);
        }

        // Fetch all agents from DB for dynamic orchestration
        let all_agents = crate::backend::get_agents(db).await.unwrap_or_default();
        
        // Find orchestrator instructions
        let master_instructions = all_agents.iter()
            .find(|a| a.domain == "orchestrator")
            .map(|a| a.instructions.clone())
            .unwrap_or_else(|| {
                println!("⚠️  WARNING: No orchestrator instructions found in DB.");
                String::new()
            });

        let master_planner = MasterPlanner::new(&self.ollama_client, &master_instructions, &all_agents);
        
        // Fetch history
        let history = crate::backend::get_messages(db, conversation_id).await.unwrap_or_default()
            .into_iter()
            .map(|m| {
                if m.role == "user" {
                    rig::completion::Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(m.content)),
                    }
                } else {
                    rig::completion::Message::Assistant {
                        id: None,
                        content: rig::OneOrMany::one(rig::message::AssistantContent::text(m.content)),
                    }
                }
            })
            .collect();

        // Retrieve relevant memories
        let memory_table = crate::engine::memory::setup_memory_table(&db.lancedb).await;
        let memory_context = if let Some(ref table) = memory_table {
            let memories = crate::engine::memory::retrieve_memories(table, &self.ollama_client, query, 3).await;
            if !memories.is_empty() {
                memories.join("\n---\n")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let response = orchestrate_request(
            db,
            &master_planner,
            &all_agents,
            query,
            history,
            &self.perception_tool,
            &memory_context,
            &self.ollama_client,
        ).await;

        // Store result in memory if successful
        if let Some(ref table) = memory_table {
            if !response.contains("HALTED") && !response.contains("Error") {
                crate::engine::memory::store_memory(table, &self.ollama_client, query, &response).await;
            }
        }

        response
    }

    pub async fn generate_title(&self, db: &Database, query: &str) -> String {
        let model_name = crate::backend::get_setting(db, "title_generator_model").await;
        let model_name = if model_name.is_empty() { "qwen2.5:7b-instruct".to_string() } else { model_name };

        let agent = rig::agent::AgentBuilder::new(self.ollama_client.completion_model(&model_name))
            .preamble("Summarize this user request in exactly 3-5 words for a conversation title. Output ONLY the title, no quotes or punctuation.")
            .build();
        
        match agent.prompt(query).await {
            Ok(resp) => {
                let title = crate::engine::utils::clean_response(resp);
                title.trim_matches(|c| c == '"' || c == '*' || c == '.').to_string()
            },
            Err(_) => "New Conversation".to_string(),
        }
    }

    async fn get_agent_instructions(&self, db: &Database, domain: &str) -> String {
        match crate::backend::get_agent_by_domain(db, domain).await {
            Ok(Some(agent)) => agent.instructions,
            _ => {
                println!("⚠️  WARNING: No instructions found in DB for domain '{}'. Using empty preamble.", domain);
                String::new()
            }
        }
    }
}
