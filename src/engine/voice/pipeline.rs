use crate::db::Database;
use crate::engine::system::AgentSystem;
use std::sync::Arc;

pub async fn execute_voice_command(
    db: &Arc<Database>,
    agent: &Arc<AgentSystem>,
    command: &str,
) -> anyhow::Result<String> {
    println!("🧠 Processing voice command: \"{}\"", command);

    // Create or reuse a voice conversation
    let conversation_id = get_voice_conversation(db).await;

    // 1. Voice Fast-path check
    if let Some(fast_result) = try_voice_fast_path(command).await {
        // Save the user message
        let _ = crate::backend::add_message(
            db, conversation_id, "user", command, false,
        ).await;
        // Save the assistant response
        let _ = crate::backend::add_message(
            db, conversation_id, "assistant", &fast_result, false,
        ).await;
        // Speak the response using Piper/TTS!
        crate::engine::voice::tts::speak_locally(&fast_result);
        println!("✅ Voice command fast-path result: {}", &fast_result);
        return Ok(fast_result);
    }

    // Save the user message
    let _ = crate::backend::add_message(
        db, conversation_id, "user", command, false,
    ).await;

    // Run through the AI orchestrator (uses existing tools: os_tool, app_launcher, etc.)
    let response = agent.orchestrate(db, conversation_id, command).await;

    // Save the assistant response
    let _ = crate::backend::add_message(
        db, conversation_id, "assistant", &response, false,
    ).await;

    // Speak the response using Piper/TTS!
    crate::engine::voice::tts::speak_locally(&response);

    println!("✅ Voice command result: {}", &response[..response.len().min(200)]);
    Ok(response)
}

async fn try_voice_fast_path(command: &str) -> Option<String> {
    let lower = command.trim().trim_end_matches('.').to_lowercase();
    
    // Pattern 1: Open/Launch application
    // Examples: "open firefox", "launch chrome", "open terminal"
    // Note: exclude URLs here so they fall through to open_url
    let args = if (lower.starts_with("open ") || lower.starts_with("launch ")) 
        && !(lower.contains("http://") || lower.contains("https://") || lower.contains(".com") || lower.contains(".org") || lower.contains(".net") || lower.contains(".io") || lower.contains("www.")) 
    {
        let target = if lower.starts_with("open ") {
            command.trim().split_at(5).1.trim()
        } else {
            command.trim().split_at(7).1.trim()
        };
        serde_json::json!({
            "action": "launch",
            "target": target
        })
    }
    // Pattern 2: Open URL
    // Examples: "open url google.com", "open google.com", "go to github.com"
    else if lower.starts_with("open url ") 
        || lower.starts_with("go to ") 
        || (lower.starts_with("open ") && (lower.contains(".com") || lower.contains(".org") || lower.contains(".net") || lower.contains(".io") || lower.contains("www.")))
    {
        let mut target = if lower.starts_with("open url ") {
            command.trim().split_at(9).1.trim().to_string()
        } else if lower.starts_with("go to ") {
            command.trim().split_at(6).1.trim().to_string()
        } else {
            command.trim().split_at(5).1.trim().to_string()
        };
        // Normalize URL
        if !target.starts_with("http://") && !target.starts_with("https://") {
            target = format!("https://{}", target);
        }
        serde_json::json!({
            "action": "open_url",
            "target": target
        })
    }
    // Pattern 3: Screenshot
    // Examples: "take a screenshot", "capture screenshot", "screenshot"
    else if lower.contains("screenshot") || lower.contains("capture the screen") {
        serde_json::json!({
            "action": "screenshot"
        })
    }
    // Pattern 4: List processes
    // Examples: "what's running", "list processes", "show running apps"
    else if lower.contains("what's running") || lower.contains("whats running") || lower.contains("list processes") || lower.contains("show processes") {
        serde_json::json!({
            "action": "list_processes"
        })
    }
    else {
        return None;
    };

    println!("⚡ Fast-path matched voice command: \"{}\"", command);
    match crate::engine::tools::native_os_tool(&args).await {
        Ok(res_json) => {
            // Parse response to find a human-readable description
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&res_json) {
                if let Some(summary) = val.get("summary").and_then(|s| s.as_str()) {
                    return Some(summary.to_string());
                }
                
                // Fallbacks if summary is missing
                let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("");
                let ok = val.get("ok").and_then(|o| o.as_bool())
                    .or_else(|| val.get("success").and_then(|s| s.as_bool()))
                    .unwrap_or(false);
                    
                if ok {
                    match action {
                        "launch" => {
                            let app = val.get("app").and_then(|a| a.as_str()).unwrap_or("application");
                            return Some(format!("Launched {}.", app));
                        }
                        "open_url" => {
                            let url = val.get("url").and_then(|u| u.as_str()).unwrap_or("URL");
                            return Some(format!("Opened {}.", url));
                        }
                        "screenshot" => {
                            return Some("Screenshot captured successfully.".to_string());
                        }
                        "list_processes" => {
                            return Some("Listed running processes.".to_string());
                        }
                        _ => return Some("Action completed successfully.".to_string()),
                    }
                }
            }
            Some("Done.".to_string())
        }
        Err(e) => {
            eprintln!("❌ Fast-path tool failed: {}", e);
            Some(format!("Sorry, I couldn't complete that action. Error: {}", e))
        }
    }
}

async fn get_voice_conversation(db: &Database) -> i64 {
    if let Ok(convs) = crate::backend::get_conversations(db).await {
        if let Some(c) = convs.iter().find(|c| c.title == "Voice Commands") {
            return c.id;
        }
    }
    crate::backend::add_conversation(db, "Voice Commands").await.unwrap_or(0)
}
