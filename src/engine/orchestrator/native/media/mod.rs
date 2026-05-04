// src/engine/orchestrator/native/media/mod.rs
pub mod pdf;
pub mod word;
pub mod complexity;

use std::path::Path;
use crate::db::Database;
use crate::engine::orchestrator::discovery::dynamic::VisionGeneralAgent;

/// Orchestrates tiered media perception: Native -> Vision Assistant.
pub async fn process_attachment(
    db: &Database,
    ollama_client: &rig::providers::ollama::Client,
    path_str: &str,
) -> String {
    let path = Path::new(path_str);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    // 1. Try Native Extraction first for documents
    let native_text = match ext.as_str() {
        "pdf" => pdf::extract_text(path).ok(),
        "docx" => word::extract_text(path).ok(),
        _ => None,
    };

    // 2. Check if we need the Vision Assistant
    if complexity::is_complex(path, native_text.as_deref()) {
        return invoke_vision_assistant(db, ollama_client, path_str).await;
    }

    native_text.unwrap_or_else(|| format!("[Attached File: {}]", path_str))
}

async fn invoke_vision_assistant(
    db: &Database,
    ollama_client: &rig::providers::ollama::Client,
    path_str: &str,
) -> String {
    println!("👁️  [Vision] Routing complex document to Vision Assistant: {}", path_str);
    
    // Fetch Vision Assistant from DB
    let agents = crate::backend::get_agents(db).await.unwrap_or_default();
    let vision_agent_info = agents.iter().find(|a| a.domain == "vision");
    
    if let Some(info) = vision_agent_info {
        let model = crate::backend::resolve_model_name(
            db,
            Some(&info.model),
            "vision_model",
            "qwen2-vl:7b-instruct",
        ).await;
        let agent = VisionGeneralAgent::new(ollama_client, &model, &info.instructions);

        // For now, we just pass the path and a prompt. 
        // In system.rs, the actual image/PDF data is handled by Rig/Ollama.
        format!("[Vision Analysis of {} using {}]", path_str, info.model)
    } else {
        format!("[Attached File (Complex): {} - Vision Assistant not configured in DB]", path_str)
    }
}
