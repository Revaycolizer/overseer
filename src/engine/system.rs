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
                    let cleaned = extract_message_content(&m.content);
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
                        // NATIVE MEDIA PERCEPTION (Tiered Fallback)
                        let media_content = crate::engine::orchestrator::native::media::process_attachment(
                            db,
                            &self.ollama_client,
                            &att.file_path,
                        ).await;
                        
                        // Use <shadow_context> to signal to the synthesizer that this is technical grounding
                        // and should not be echoed back to the user bubble.
                        contents.push(rig::message::UserContent::text(format!(
                            "\n[Attached File: {}]\n<shadow_context>\nNATIVE_GROUNDING_DATA:\n{}\n</shadow_context>",
                            att.filename, media_content
                        )));
                    }


                }

                history.push(rig::completion::Message::User {
                    content: rig::OneOrMany::many(contents).unwrap(),
                });
            } else {
                let cleaned_content = extract_message_content(&m.content);
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

fn extract_message_content(content: &str) -> String {
    let cleaned = strip_internal_message_markers(content);
    
    // If it looks like JSON, try to extract the actual message text and metadata
    if cleaned.trim().starts_with('{') && cleaned.trim().ends_with('}') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&cleaned) {
            let mut final_text = String::new();

            // 1. Extract Primary Messages (StructuredResponse)
            if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
                for m in msgs {
                    if let Some(msg) = m.get("message").and_then(|s| s.as_str()) {
                        if !final_text.is_empty() { final_text.push_str("\n"); }
                        final_text.push_str(msg);
                    }
                }
            } else if let Some(summary) = v.get("summary").and_then(|s| s.as_str()) {
                // Fallback: SupervisorExecutionResult
                final_text = summary.to_string();
            }

            // 2. METADATA INJECTION: Append artifacts and sources for context awareness
            let mut metadata = Vec::new();

            if let Some(artifacts) = v.get("artifacts").and_then(|a| a.as_array()) {
                for a in artifacts {
                    if let Some(path) = a.as_str() {
                        let filename = path.split('/').last().unwrap_or(path);
                        metadata.push(format!("[Generated Artifact: {}]", filename));
                    }
                }
            }

            if let Some(sources) = v.get("sources").and_then(|s| s.as_array()) {
                for (i, s) in sources.iter().enumerate() {
                    let title = s.get("source").and_then(|v| v.as_str()).unwrap_or("Unknown Source");
                    metadata.push(format!("[Source #{}: {}]", i + 1, title));
                }
            } else if let Some(evidence) = v.get("evidence").and_then(|e| e.as_array()) {
                for (i, e) in evidence.iter().enumerate() {
                    if let Some(text) = e.as_str() {
                        let snippet = if text.len() > 60 { format!("{}...", &text[..60]) } else { text.to_string() };
                        metadata.push(format!("[Evidence #{}: {}]", i + 1, snippet));
                    }
                }
            }

            if !metadata.is_empty() {
                if !final_text.is_empty() { final_text.push_str("\n\n"); }
                final_text.push_str("CONTEXTUAL_METADATA:\n");
                final_text.push_str(&metadata.join("\n"));
            }

            if !final_text.is_empty() { return final_text; }
        }
    }
    
    cleaned
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
