use rig::client::{CompletionClient, Nothing};
use rig::completion::Prompt;
use rig::providers::ollama;

use crate::db::Database;
use crate::engine::orchestrator::orchestrate_request;
use crate::engine::orchestrator::MasterPlanner;
use crate::engine::tools::StructuredPerceptionTool;

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

        // Find orchestrator agent info
        let orchestrator_agent = all_agents.iter().find(|a| a.domain == "orchestrator");
        let master_instructions = orchestrator_agent
            .map(|a| a.instructions.clone())
            .unwrap_or_default();
        let master_model = crate::backend::resolve_model_name(
            db,
            orchestrator_agent.map(|a| a.model.as_str()),
            "default_chat_model",
            "qwen2.5:7b-instruct",
        )
        .await;

        let master_planner = MasterPlanner::new(
            &self.ollama_client,
            &master_model,
            &master_instructions,
            &all_agents,
        );

        // Fetch conversation history (scoped to this conversation only)
        let messages = crate::backend::get_messages(db, conversation_id)
            .await
            .unwrap_or_default();

        // Pre-compute compact summary before consuming messages in the loop below
        let recent_summary: String = if messages.len() > 2 {
            let recent: Vec<String> = messages
                .iter()
                .rev()
                .take(6)
                .rev()
                .map(|m| {
                    let cleaned = strip_internal_message_markers(&m.content);
                    format!(
                        "[{}]: {}",
                        m.role.to_uppercase(),
                        &cleaned[..cleaned.len().min(300)]
                    )
                })
                .collect();
            format!(
                "Recent conversation context (current session only):\n{}",
                recent.join("\n")
            )
        } else {
            String::new()
        };

        let mut history = Vec::new();
        for m in messages {
            if m.role == "user" {
                let cleaned_content = strip_internal_message_markers(&m.content);
                let content_text = if cleaned_content.trim().is_empty() {
                    "(user message)".to_string()
                } else {
                    cleaned_content
                };
                let mut contents = vec![rig::message::UserContent::text(content_text)];

                // Reuse attachments already hydrated by get_messages instead of re-querying them.
                for att in m.attachments {
                        if att.content_type.starts_with("image/") {
                            // Read the file from disk and encode as base64 for vision models
                            match std::fs::read(&att.file_path) {
                                Ok(bytes) if !bytes.is_empty() => {
                                    use base64::{engine::general_purpose, Engine as _};
                                    let b64 = general_purpose::STANDARD.encode(&bytes);
                                    let media_type = match att.content_type.as_str() {
                                        "image/png" => Some(rig::message::ImageMediaType::PNG),
                                        "image/webp" => Some(rig::message::ImageMediaType::WEBP),
                                        "image/gif" => Some(rig::message::ImageMediaType::GIF),
                                        _ => Some(rig::message::ImageMediaType::JPEG), // default JPEG
                                    };
                                    contents.push(rig::message::UserContent::Image(
                                        rig::completion::message::Image {
                                            data: rig::message::DocumentSourceKind::Base64(b64),
                                            media_type,
                                            detail: None,
                                            additional_params: None,
                                        },
                                    ));
                                }
                                Ok(_) => {
                                    contents.push(rig::message::UserContent::text(format!(
                                        "\n[Attached Image: {} — file appears empty]",
                                        att.filename
                                    )));
                                }
                                Err(e) => {
                                    contents.push(rig::message::UserContent::text(format!(
                                        "\n[Attached Image: {} — could not read: {}]",
                                        att.filename, e
                                    )));
                                }
                            }
                        } else {
                            // Non-image: include as a text description so the model knows about it
                            contents.push(rig::message::UserContent::text(format!(
                                "\n[Attached File: {} (Path: {})]",
                                att.filename, att.file_path
                            )));
                        }
                }

                history.push(rig::completion::Message::User {
                    content: rig::OneOrMany::many(contents).unwrap(),
                });
            } else {
                let cleaned_content = strip_internal_message_markers(&m.content);
                let content_text = if cleaned_content.trim().is_empty() {
                    "(assistant message)".to_string()
                } else {
                    cleaned_content
                };
                history.push(rig::completion::Message::Assistant {
                    id: None,
                    content: rig::OneOrMany::one(rig::message::AssistantContent::text(
                        content_text,
                    )),
                });
            }
        }

        // Use the pre-computed per-conversation context (not global LanceDB memories)
        let memory_context = recent_summary;

        println!("📜 History has {} messages.", history.len());
        for (i, m) in history.iter().enumerate() {
            match m {
                rig::completion::Message::User { content } => {
                    println!("   [M{}] USER: ({} items)", i, content.len())
                }
                rig::completion::Message::Assistant { content, .. } => {
                    println!("   [M{}] ASST: {}", i, content.len())
                }
            }
        }

        let response = orchestrate_request(
            db,
            conversation_id,
            &master_planner,
            &all_agents,
            query,
            history,
            &self.perception_tool,
            &memory_context,
            &self.ollama_client,
        )
        .await;

        response
    }

    pub async fn generate_title(&self, db: &Database, query: &str) -> String {
        let model_name = crate::backend::resolve_model_name(
            db,
            None,
            "title_generator_model",
            "qwen2.5:7b-instruct",
        )
        .await;
        let model_timeout_ms =
            crate::backend::get_setting_i64(db, "model_timeout_ms", 90_000).await as u64;

        let agent = rig::agent::AgentBuilder::new(self.ollama_client.completion_model(&model_name))
            .preamble("Summarize this user request in exactly 3-5 words for a conversation title. Output ONLY the title, no quotes or punctuation.")
            .build();

        match tokio::time::timeout(
            std::time::Duration::from_millis(model_timeout_ms),
            agent.prompt(query),
        )
        .await
        {
            Ok(Ok(resp)) => {
                let title = crate::engine::utils::clean_response(resp);
                title
                    .trim_matches(|c| c == '"' || c == '*' || c == '.')
                    .to_string()
            }
            Ok(Err(_)) | Err(_) => "New Conversation".to_string(),
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

fn strip_internal_message_markers(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed == "ARTIFACT_CONTEXT:" || trimmed == "WORK_ITEM_CONTEXT:" {
                return false;
            }
            !((trimmed.starts_with("[[ATTACH_IMAGE:")
                || trimmed.starts_with("[[ARTIFACT_REF:")
                || trimmed.starts_with("[[WORK_ITEM_REF:"))
                && trimmed.ends_with("]]"))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
