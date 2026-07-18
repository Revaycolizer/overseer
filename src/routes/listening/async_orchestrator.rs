use dioxus::prelude::*;
use crate::db::Database;
use crate::engine::system::AgentSystem;
use std::sync::Arc;

pub async fn run_orchestration_async(
    db: Arc<Database>,
    agent: Arc<AgentSystem>,
    cmd: String,
    mut voice_transcript: Signal<Option<String>>,
    mut voice_result: Signal<Option<String>>,
    mut is_listening: Signal<bool>,
    mut is_inhibited: Signal<bool>,
) {
    println!("🎙️ Starting asynchronous orchestration for command: \"{}\"", cmd);
    
    voice_transcript.set(Some(cmd.clone()));

    // 1. Immediately dismiss the blocking listening screens (Tier 3 asynchronous behavior)
    is_listening.set(false);
    is_inhibited.set(false);

    // 2. Play continuous hum loop in the background during AI planning
    crate::engine::voice::earcons::start_thinking_hum_loop();

    // 3. Spawn the orchestrator task
    spawn(async move {
        match crate::engine::voice::pipeline::execute_voice_command(&db, &agent, &cmd).await {
            Ok(res) => {
                voice_result.set(Some(res));
            }
            Err(e) => {
                eprintln!("❌ Async orchestration failed: {}", e);
                voice_result.set(Some(format!("Execution Error: {}", e)));
                crate::engine::voice::earcons::play_blocked_beep();
                crate::engine::voice::tts::speak_locally("Task execution failed.");
            }
        }
        
        crate::engine::voice::earcons::stop_thinking_hum_loop();
    });
}

pub async fn run_local_stt_fallback_async(
    db: Arc<Database>,
    agent: Arc<AgentSystem>,
    voice_transcript: Signal<Option<String>>,
    voice_result: Signal<Option<String>>,
    is_listening: Signal<bool>,
    is_inhibited: Signal<bool>,
) {
    let vad_threshold = crate::engine::voice::wake_word::get_vad_threshold();
    
    match crate::engine::voice::local_stt::record_and_transcribe(vad_threshold).await {
        Ok(cmd) => {
            println!("🎙️ Local offline STT transcribed: \"{}\"", cmd);
            run_orchestration_async(
                db,
                agent,
                cmd,
                voice_transcript,
                voice_result,
                is_listening,
                is_inhibited,
            ).await;
        }
        Err(err) => {
            eprintln!("🎙️ Local STT fallback failed: {}", err);
            crate::engine::voice::earcons::play_blocked_beep();
            crate::engine::voice::tts::speak_locally("Speech recognition failed.");
            let mut listening = is_listening;
            let mut inhibited = is_inhibited;
            listening.set(false);
            inhibited.set(false);
        }
    }
}
