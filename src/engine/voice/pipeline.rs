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
    if let Some(fast_result) = try_voice_fast_path(db, command).await {
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
    let response = agent.orchestrate(db, conversation_id, command, None).await;

    // Save the assistant response
    let _ = crate::backend::add_message(
        db, conversation_id, "assistant", &response, false,
    ).await;

    // Speak the response using Piper/TTS!
    crate::engine::voice::tts::speak_locally(&response);

    println!("✅ Voice command result: {}", &response[..response.len().min(200)]);
    Ok(response)
}

async fn try_voice_fast_path(db: &Database, command: &str) -> Option<String> {
    let lower = command.trim().trim_end_matches('.').to_lowercase();
    
    if let Some(vol_res) = super::fast_path_utils::handle_volume_fast_path(&lower).await {
        return Some(vol_res);
    }
    
    // Pattern 1: Open/Launch application
    // Examples: "open firefox", "launch chrome", "open terminal", "start spotify"
    // Note: exclude URLs here so they fall through to open_url
    let args = if (lower.starts_with("open ") || lower.starts_with("launch ") || lower.starts_with("start ")) 
        && !(lower.contains("http://") || lower.contains("https://") || lower.contains(".com") || lower.contains(".org") || lower.contains(".net") || lower.contains(".io") || lower.contains("www.")) 
    {
        let target = if lower.starts_with("open ") {
            command.trim().split_at(5).1.trim()
        } else if lower.starts_with("launch ") {
            command.trim().split_at(7).1.trim()
        } else {
            command.trim().split_at(6).1.trim()
        };
        serde_json::json!({
            "action": "launch",
            "target": target
        })
    }
    // Pattern 2: Open URL
    // Examples: "open url google.com", "open google.com", "go to github.com", "navigate to youtube.com"
    else if lower.starts_with("open url ") 
        || lower.starts_with("go to ") 
        || lower.starts_with("navigate to ")
        || (lower.starts_with("open ") && (lower.contains(".com") || lower.contains(".org") || lower.contains(".net") || lower.contains(".io") || lower.contains("www.")))
    {
        let mut target = if lower.starts_with("open url ") {
            command.trim().split_at(9).1.trim().to_string()
        } else if lower.starts_with("go to ") {
            command.trim().split_at(6).1.trim().to_string()
        } else if lower.starts_with("navigate to ") {
            command.trim().split_at(12).1.trim().to_string()
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
    // Examples: "take a screenshot", "capture screenshot", "screenshot", "capture my screen"
    else if lower.contains("screenshot") || lower.contains("capture the screen") || lower.contains("capture my screen") {
        serde_json::json!({
            "action": "screenshot"
        })
    }
    // Pattern 4: List processes
    // Examples: "what's running", "list processes", "show running apps", "show background tasks"
    else if lower.contains("what's running") || lower.contains("whats running") || lower.contains("list processes") || lower.contains("show processes") || lower.contains("background tasks") || lower.contains("running apps") {
        serde_json::json!({
            "action": "list_processes"
        })
    }
    // Pattern 5: Focus a window
    // Examples: "focus firefox", "switch to chrome", "bring up terminal"
    else if lower.starts_with("focus ") || lower.starts_with("switch to ") || lower.starts_with("bring up ") {
        let target = if lower.starts_with("focus ") {
            command.trim().split_at(6).1.trim()
        } else if lower.starts_with("switch to ") {
            command.trim().split_at(10).1.trim()
        } else {
            command.trim().split_at(9).1.trim()
        };
        serde_json::json!({
            "action": "focus",
            "target": target
        })
    }
    // Pattern 6: Type text
    // Examples: "type hello world", "type in hello"
    else if lower.starts_with("type ") || lower.starts_with("type in ") {
        let text = if lower.starts_with("type in ") {
            command.trim().split_at(8).1.trim()
        } else {
            command.trim().split_at(5).1.trim()
        };
        serde_json::json!({
            "action": "type_text",
            "text": text
        })
    }
    // Pattern 7: Scroll
    // Examples: "scroll down", "scroll up"
    else if lower.starts_with("scroll ") {
        let direction = if lower.contains("up") { "up" } else { "down" };
        serde_json::json!({
            "action": "scroll",
            "direction": direction
        })
    }
    // Pattern 8: Press key / Hotkey
    // Examples: "press enter", "press escape", "press ctrl+c"
    else if lower.starts_with("press ") || lower.starts_with("hit ") {
        let key_str = if lower.starts_with("press ") {
            command.trim().split_at(6).1.trim()
        } else {
            command.trim().split_at(4).1.trim()
        };
        if key_str.contains('+') {
            serde_json::json!({
                "action": "hotkey",
                "keys": key_str
            })
        } else {
            serde_json::json!({
                "action": "press_key",
                "key": key_str
            })
        }
    }
    // Pattern 9: Get desktop state
    // Examples: "what's on screen", "what windows are open", "show desktop state", "what do you see"
    else if lower.contains("what's on screen") || lower.contains("whats on screen") || lower.contains("windows are open") || lower.contains("desktop state") || lower.contains("what do you see") {
        serde_json::json!({
            "action": "get_state"
        })
    }
    // Pattern 10: Clipboard
    // Examples: "read clipboard", "what's in clipboard", "copy this: hello", "paste"
    else if lower.contains("read clipboard") || lower.contains("what's in clipboard") || lower.contains("whats in clipboard") || lower == "paste" {
        return match crate::engine::tools::clipboard::native_clipboard_read().await {
            Ok(content) => Some(format!("Clipboard contains: {}", if content.len() > 200 { &content[..200] } else { &content })),
            Err(e) => Some(format!("Could not read clipboard: {}", e)),
        };
    }
    else {
        return None;
    };

    println!("⚡ Fast-path matched voice command: \"{}\"", command);

    // Governance: the voice fast-path is user-triggerable, so its computer
    // action must pass kernel policy. The voice loop cannot wait for approval,
    // so it proceeds only on a deterministic Allow (observations like get_state
    // are allowed; mutating actions in approval mode are refused here).
    if !matches!(
        crate::engine::kernel::evaluate_named(
            db,
            "computer",
            overseer_kernel::ActionKind::ComputerControl,
            "os_tool",
            &args,
            &["os_tool".to_string()],
        )
        .await,
        overseer_kernel::PolicyDecision::Allow
    ) {
        return Some(
            "That action requires approval and can't be run by voice fast-path.".to_string(),
        );
    }

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
    if let Ok(convs) = crate::backend::get_conversations(db, 50).await {
        if let Some(c) = convs.iter().find(|c| c.title == "Voice Commands") {
            return c.id;
        }
    }
    crate::backend::add_conversation(db, "Voice Commands").await.unwrap_or(0)
}
