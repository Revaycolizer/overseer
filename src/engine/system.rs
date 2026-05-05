// src/engine/system.rs
use crate::db::Database;
use crate::engine::orchestrator::native::media;
use crate::engine::orchestrator::planning::master::MasterPlanner;
use crate::engine::tools::StructuredPerceptionTool;
use rig::completion::{Message, AssistantContent, Prompt};
use rig::message::UserContent;
use rig::providers::ollama::{self, OllamaExt};
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
        let messages = crate::backend::get_messages(db, conversation_id)
            .await
            .unwrap_or_default();

        let mut history = Vec::new();
        for m in messages {
            if m.role == "user" {
                let cleaned_content = strip_internal_message_markers(&m.content);
                let content_text = if cleaned_content.trim().is_empty() {
                    "(user message)".to_string()
                } else {
                    cleaned_content
                };
                let mut contents = vec![UserContent::text(content_text)];

                for att in m.attachments {
                    if att.content_type.starts_with("image/") {
                        match std::fs::read(&att.file_path) {
                            Ok(bytes) if !bytes.is_empty() => {
                                use base64::{engine::general_purpose, Engine as _};
                                let b64 = general_purpose::STANDARD.encode(&bytes);
                                let media_type = match att.content_type.as_str() {
                                    "image/png" => Some(rig::message::ImageMediaType::PNG),
                                    "image/webp" => Some(rig::message::ImageMediaType::WEBP),
                                    "image/gif" => Some(rig::message::ImageMediaType::GIF),
                                    _ => Some(rig::message::ImageMediaType::JPEG),
                                };
                                contents.push(UserContent::Image(rig::completion::message::Image {
                                    data: rig::message::DocumentSourceKind::Base64(b64),
                                    media_type,
                                    detail: None,
                                    additional_params: None,
                                }));
                            }
                            _ => {
                                contents.push(UserContent::text(format!("\n[Image Error: {}]", att.filename)));
                            }
                        }
                    } else {
                        let media_content = if let Some(cached) = att.extracted_content {
                            cached
                        } else {
                            let extracted = media::process_attachment(db, &self.ollama_client, &att.file_path).await;
                            let _ = crate::backend::update_attachment_content(db, att.id, &extracted).await;
                            extracted
                        };

                        contents.push(UserContent::text(format!(
                            "\n[Attached File: {}]\n<shadow_context>\nNATIVE_GROUNDING_DATA:\n{}\n</shadow_context>",
                            att.filename, media_content
                        )));
                    }
                }
                history.push(Message::User { 
                    content: rig::OneOrMany::many(contents).expect("Non-empty contents") 
                });
            } else if m.role == "assistant" {
                history.push(Message::Assistant { 
                    id: Some(m.id.to_string()), 
                    content: rig::OneOrMany::one(AssistantContent::text(m.content.clone())) 
                });
            }
        }

        let all_agents = crate::backend::get_agents(db).await.unwrap_or_default();
        let planner_model = crate::backend::resolve_model_name(db, None, "planner_model", "qwen2.5:7b-instruct").await;
        let instructions = crate::backend::get_setting(db, "master_planner_instructions").await;
        
        let master_planner = MasterPlanner::new(
            &self.ollama_client,
            &planner_model,
            &instructions,
            &all_agents,
        );

        let perception_tool = StructuredPerceptionTool::new(db.clone());
        let memory_context = crate::backend::get_setting(db, "system_memory_context").await;

        crate::engine::orchestrator::core::main_loop::orchestrate_request(
            db,
            conversation_id,
            &master_planner,
            &all_agents,
            user_query,
            history,
            &perception_tool,
            &memory_context,
            &self.ollama_client,
        ).await
    }

    pub async fn generate_title(&self, db: &Database, query: &str) -> String {
        let model_name = crate::backend::resolve_model_name(
            db, 
            None, 
            "title_generator_model", 
            "qwen2.5:7b-instruct"
        ).await;

        let agent = rig::agent::AgentBuilder::new(self.ollama_client.completion_model(&model_name))
            .preamble("You are a concise title generator. Summarize the user's query into a 3-5 word title. No emojis. No quotes. No asterisks.")
            .build();

        match agent.prompt(query).await {
            Ok(title) => title.trim().trim_matches('"').to_string(),
            Err(_) => query.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
        }
    }
}

impl PartialEq for AgentSystem {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Clone for AgentSystem {
    fn clone(&self) -> Self {
        Self {
            ollama_client: self.ollama_client.clone(),
        }
    }
}

impl fmt::Debug for AgentSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentSystem").finish()
    }
}

fn strip_internal_message_markers(content: &str) -> String {
    content.replace("<shadow_context>", "").replace("</shadow_context>", "").trim().to_string()
}
