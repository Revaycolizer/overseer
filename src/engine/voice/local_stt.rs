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
    let res = transcribe_samples_sync(&samples);
    super::earcons::stop_thinking_hum_loop();
    match res {
        // An empty transcript is also "no speech" — re-listen, don't error out.
        Ok(t) => Ok(t.trim().to_string()),
        // A genuine tool failure (mic device / STT binary) stays an error.
        Err(e) => Err(e),
    }
}

pub fn transcribe_samples_sync(recorded_data: &[f32]) -> Result<String, String> {
    let data_dir = crate::db::get_overseer_data_dir();
    let model_path = data_dir.join("assets").join("voice").join("models").join("ggml-tiny.en.bin");
    let optimizer = super::stt_optimizer::SpeechOptimizer::new(&model_path.to_string_lossy());
    let res = optimizer.transcribe_stream(recorded_data);
    if res.trim().is_empty() {
        super::local_stt_trans::transcribe_samples_sync(recorded_data)
    } else {
        Ok(res)
    }
}
