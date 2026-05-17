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

async fn get_voice_conversation(db: &Database) -> i64 {
    if let Ok(convs) = crate::backend::get_conversations(db).await {
        if let Some(c) = convs.iter().find(|c| c.title == "Voice Commands") {
            return c.id;
        }
    }
    crate::backend::add_conversation(db, "Voice Commands").await.unwrap_or(0)
}
