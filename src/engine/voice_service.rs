//! Always-on voice service (opt-in).
//!
//! Runs the wake-word listener, and on each wake records + transcribes an
//! utterance and hands the transcript to the existing voice pipeline
//! (`execute_voice_command`, which orchestrates + speaks the reply). Spawned by
//! the daemon only when the `voice_always_on` setting is enabled — audio capture
//! needs a real input device, so it stays off by default (e.g. on headless
//! servers).
//!
//! NOTE: this reuses the current voice pipeline (orchestrator + TTS). Routing
//! voice through the governed agent loop (`agent_loop::run_agent`) is a follow-up
//! that depends on surfacing approvals to a voice/remote channel.

use crate::db::Database;
use crate::engine::system::AgentSystem;
use crate::engine::voice::{local_stt, pipeline, wake_word};
use std::sync::Arc;
use std::time::Duration;

pub async fn run_voice_service(db: Arc<Database>, agent: Arc<AgentSystem>) {
    tracing::info!("voice service starting (always-on wake word)");
    wake_word::start_wake_word_listener();

    loop {
        if wake_word::try_recv_panic() {
            tracing::warn!("wake-word listener panicked; restarting");
            wake_word::restart_wake_word_listener();
        }

        if wake_word::try_recv_wake() {
            let threshold = wake_word::get_vad_threshold();
            match local_stt::record_and_transcribe(threshold).await {
                Ok(transcript) if !transcript.trim().is_empty() => {
                    let cmd = transcript.trim();
                    tracing::info!(command = %cmd, "voice command received");
                    if let Err(e) = pipeline::execute_voice_command(&db, &agent, cmd).await {
                        tracing::warn!(?e, "voice command failed");
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "voice STT failed"),
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
