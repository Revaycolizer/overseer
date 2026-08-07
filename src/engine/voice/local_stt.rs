pub async fn record_and_transcribe(vad_threshold: f32) -> Result<String, String> {
    let samples = super::local_stt_rec::record_samples(vad_threshold).await?;
    let peak = samples.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
    tracing::info!(samples = samples.len(), peak, "voice: captured audio");

    // Distinguish a silent/absent mic from a transcription failure — otherwise
    // both look like the same unhelpful "all fallbacks failed" error.
    if samples.len() < 1600 || peak < 0.01 {
        return Err(format!(
            "No speech captured (mic level {:.3}). Check that your microphone is the default input device and unmuted.",
            peak
        ));
    }

    super::earcons::start_thinking_hum_loop();
    let res = transcribe_samples_sync(&samples);
    super::earcons::stop_thinking_hum_loop();
    match res {
        Ok(t) if !t.trim().is_empty() => Ok(t),
        Ok(_) => Err("Heard audio but transcription came back empty — try speaking a bit louder/closer.".to_string()),
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
