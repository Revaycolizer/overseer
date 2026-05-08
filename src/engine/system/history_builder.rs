// src/engine/system/history_builder.rs
use crate::db::Database;
use crate::engine::orchestrator::native::media;
use rig::completion::{Message, AssistantContent};
use rig::message::UserContent;
use rig::providers::ollama;

pub async fn build_history(
    db: &Database,
    ollama_client: &ollama::Client,
    messages: Vec<crate::backend::Message>,
    user_query: &str,
) -> Vec<Message> {
    let mut history = Vec::new();
    let msg_count = messages.len();
    
    for (i, m) in messages.into_iter().enumerate() {
        let is_current = i == msg_count - 1;
        
        if m.role == "user" {
            let content_text = strip_internal_message_markers(&m.content);
            let mut contents = vec![UserContent::text(
                if content_text.is_empty() { "(user message)".to_string() } else { content_text }
            )];

            for att in m.attachments {
                let media_content = if is_current {
                    if att.extracted_content.is_none() {
                        let generic = media::process_attachment(db, ollama_client, &att.file_path, None).await;
                        let _ = crate::backend::update_attachment_content(db, att.id, &generic).await;
                    }
                    media::process_attachment(db, ollama_client, &att.file_path, Some(user_query)).await
                } else {
                    if let Some(cached) = att.extracted_content {
                        cached
                    } else {
                        let extracted = media::process_attachment(db, ollama_client, &att.file_path, None).await;
                        let _ = crate::backend::update_attachment_content(db, att.id, &extracted).await;
                        extracted
                    }
                };

                contents.push(UserContent::text(format!(
                    "\n[Attached File: {}]\n<shadow_context>\nNATIVE_GROUNDING_DATA:\n{}\n</shadow_context>",
                    att.filename, media_content
                )));

                if att.content_type.starts_with("image/") {
                    if let Ok(bytes) = std::fs::read(&att.file_path) {
                        if !bytes.is_empty() {
                            add_image_to_contents(&mut contents, bytes, &att.content_type);
                        }
                    }
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
    history
}

fn add_image_to_contents(contents: &mut Vec<UserContent>, bytes: Vec<u8>, content_type: &str) {
    use base64::{engine::general_purpose, Engine as _};
    let b64 = general_purpose::STANDARD.encode(&bytes);
    let media_type = match content_type {
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

fn strip_internal_message_markers(content: &str) -> String {
    content.replace("<shadow_context>", "").replace("</shadow_context>", "").trim().to_string()
}
