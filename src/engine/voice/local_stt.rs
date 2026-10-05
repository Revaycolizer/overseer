pub async fn record_and_transcribe(vad_threshold: f32) -> Result<String, String> {
    let samples = super::local_stt_rec::record_samples(vad_threshold).await?;
    let peak = samples.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
    tracing::info!(samples = samples.len(), peak, "voice: captured audio");

    // No real speech (silence / too quiet / no VAD hit) → return empty so the
    // caller simply re-listens rather than replying to nothing.
    if samples.is_empty() || samples.len() < 1600 || peak < 0.01 {
        return Ok(String::new());
    }

    super::earcons::start_thinking_hum_loop();
    let res = tokio::task::spawn_blocking(move || transcribe_samples_sync(&samples))
        .await
        .map_err(|e| format!("Speech transcription task failed: {e}"))
        .and_then(|result| result);
    super::earcons::stop_thinking_hum_loop();
    match res {
        // An empty transcript is also "no speech" — re-listen, don't error out.
        Ok(t) => Ok(t.trim().to_string()),
        // A genuine tool failure (mic device / STT binary) stays an error.
        Err(e) => Err(e),
    }
}

pub fn transcribe_samples_sync(recorded_data: &[f32]) -> Result<String, String> {
    let model_path = super::local_stt_trans::whisper_model_path();
    let optimizer = super::stt_optimizer::SpeechOptimizer::new(&model_path.to_string_lossy());
    // Preserve failures instead of treating them as silence and running STT twice.
    optimizer.transcribe(recorded_data)
}
