pub async fn record_and_transcribe(vad_threshold: f32) -> Result<String, String> {
    let samples = super::local_stt_rec::record_samples(vad_threshold).await?;
    super::earcons::start_thinking_hum_loop();
    let res = transcribe_samples_sync(&samples);
    super::earcons::stop_thinking_hum_loop();
    res
}

pub fn transcribe_samples_sync(recorded_data: &[f32]) -> Result<String, String> {
    super::local_stt_trans::transcribe_samples_sync(recorded_data)
}
