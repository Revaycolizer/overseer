use rig::providers::ollama;
use rig::client::Nothing;
use std::sync::Arc;
use std::collections::HashMap;

use crate::engine::agents::{
    master_planner::MasterPlanner,
    web::{planner::WebPlanner, supervisor::WebSupervisor},
    office::{planner::OfficePlanner, supervisor::OfficeSupervisor},
    desktop::{planner::DesktopPlanner, supervisor::AppSupervisor},
    neontune::{planner::NeontunePlanner, supervisor::NeontuneSupervisor},
    text_writer::supervisor::TextWriterSupervisor,
    overseer::OverseerAgent,
    chat::ChatAgent,
    stealth::StealthSupervisor,
};
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
    pub fn new() -> Self {
        let client = ollama::Client::new(Nothing).expect("Failed to create Ollama client");
        
        Self {
            ollama_client: client,
            perception_tool: StructuredPerceptionTool::new(),
        }
    }

    pub async fn orchestrate(&self, db: &Database, conversation_id: i64, query: &str) -> String {
        let master_instructions = self.get_agent_instructions(db, "orchestrator").await;
        let web_p_instructions = self.get_agent_instructions(db, "web_planner").await;
        let web_s_instructions = self.get_agent_instructions(db, "web_supervisor").await;
        let office_p_instructions = self.get_agent_instructions(db, "office_planner").await;
        let office_s_instructions = self.get_agent_instructions(db, "office_supervisor").await;
        let desktop_p_instructions = self.get_agent_instructions(db, "desktop_planner").await;
        let desktop_s_instructions = self.get_agent_instructions(db, "desktop_supervisor").await;
        let neontune_p_instructions = self.get_agent_instructions(db, "neontune_planner").await;
        let neontune_s_instructions = self.get_agent_instructions(db, "neontune_supervisor").await;
        let text_writer_instructions = self.get_agent_instructions(db, "text_writer").await;
        let overseer_instructions = self.get_agent_instructions(db, "overseer").await;
        let chat_instructions = self.get_agent_instructions(db, "chat").await;
        let stealth_instructions = self.get_agent_instructions(db, "stealth").await;

        let master_planner = MasterPlanner::new(&self.ollama_client, &master_instructions);
        let web_planner = WebPlanner::new(&self.ollama_client, &web_p_instructions); 
        let office_planner = OfficePlanner::new(&self.ollama_client, &office_p_instructions);
        let desktop_planner = DesktopPlanner::new(&self.ollama_client, &desktop_p_instructions);
        let neontune_planner = NeontunePlanner::new(&self.ollama_client, &neontune_p_instructions);
        
        let web_supervisor = WebSupervisor::new(&self.ollama_client, &web_s_instructions);
        let app_supervisor = AppSupervisor::new(&self.ollama_client, &desktop_s_instructions);
        let office_supervisor = OfficeSupervisor::new(&self.ollama_client, &office_s_instructions);
        let text_writer_supervisor = TextWriterSupervisor::new(&self.ollama_client, &text_writer_instructions);
        let neontune_supervisor = NeontuneSupervisor::new(&self.ollama_client, &neontune_s_instructions);
        let overseer = OverseerAgent::new(&self.ollama_client, &overseer_instructions);
        let chat_agent = ChatAgent::new(&self.ollama_client, &chat_instructions);
        let stealth_supervisor = StealthSupervisor::new(&self.ollama_client, &stealth_instructions);

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
            &web_planner,
            &office_planner,
            &desktop_planner,
            &neontune_planner,
            &web_supervisor,
            &app_supervisor,
            &office_supervisor,
            &text_writer_supervisor,
            &neontune_supervisor,
            &overseer,
            &chat_agent,
            &stealth_supervisor,
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
