// src/engine/orchestrator/native/media/mod.rs
pub mod pdf;
pub mod word;
pub mod image;
pub mod complexity;

use std::path::Path;
use crate::db::Database;
use crate::engine::orchestrator::discovery::dynamic::VisionGeneralAgent;

/// Orchestrates tiered media perception: Native -> Vision Assistant.
pub async fn process_attachment(
    db: &Database,
    ollama_client: &rig::providers::ollama::Client,
    path_str: &str,
    query: Option<&str>,
) -> String {
    let path = Path::new(path_str);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    // 1. Try Native Extraction (Fast Path)
    let native_text = match ext.as_str() {
        "pdf" => pdf::extract_text(path).ok(),
        "docx" => word::extract_text(path).ok(),
        "png" | "jpg" | "jpeg" | "webp" => image::extract_text(path).ok(),
        _ => None,
    };

    // 2. Parallel Perception: If complex (like photos), trigger Vision tier
    if complexity::is_complex(path, native_text.as_deref()) {
        let vision_insights = invoke_vision_assistant(db, ollama_client, path_str, query).await;
        
        return match native_text {
            Some(text) => format!("[Native OCR Text]\n{}\n\n[Vision Insights]\n{}", text, vision_insights),
            None => vision_insights,
        };
    }

    native_text.unwrap_or_else(|| format!("[Attached File: {}]", path_str))
}

async fn invoke_vision_assistant(
    db: &Database,
    ollama_client: &rig::providers::ollama::Client,
    path_str: &str,
    query: Option<&str>,
) -> String {
    let path = Path::new(path_str);
    let agents = crate::backend::get_agents(db).await.unwrap_or_default();
    let vision_agent_info = agents.iter().find(|a| a.domain == "vision");
    
    if let Some(info) = vision_agent_info {
        let model = crate::backend::resolve_model_name(
            db, Some(&info.model), "vision_model", "qwen2-vl:7b-instruct",
        ).await;
        
        let agent = VisionGeneralAgent::new(ollama_client, &model, &info.instructions);
        
        tracing::info!(?path_str, "Vision: executing deep perception");
        
        // ARCHITECTURE V3.6: GUIDED PERCEPTION
        let prompt = if let Some(q) = query {
            format!("Analyze the image to provide context for this user request: '{}'. Describe relevant parts of the image that help answer the question, but remain a descriptive observer. Avoid answering the user directly if possible; provide the grounding data instead.", q)
        } else {
            "Describe this image in detail and identify key data points.".to_string()
        };
        
        agent.perceive(&prompt, path).await
    } else {
        format!("[Attached File (Complex): {} - Vision Assistant not configured]", path_str)
    }
}
