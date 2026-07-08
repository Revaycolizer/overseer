#[cfg(feature = "whisper")]
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

pub struct SpeechOptimizer {
    #[cfg(feature = "whisper")]
    whisper_ctx: Option<WhisperContext>,
    #[allow(dead_code)]
    custom_vocabulary: String,
}

impl SpeechOptimizer {
    pub fn new(model_path: &str) -> Self {
        let vocab = "Overseer, Rust, Dioxus, Postgres, PgBouncer, Actix, Elysia, Bun, Dar es Salaam".to_string();
        #[cfg(feature = "whisper")]
        {
            let path = std::path::Path::new(model_path);
            let ctx = WhisperContext::new_with_params(
                &path.to_string_lossy(),
                WhisperContextParameters::default(),
            ).ok();
            Self {
                whisper_ctx: ctx,
                custom_vocabulary: vocab,
            }
        }
        #[cfg(not(feature = "whisper"))]
        {
            let _ = model_path;
            Self {
                custom_vocabulary: vocab,
            }
        }
    }

    pub fn transcribe_stream(&self, audio_data: &[f32]) -> String {
        #[cfg(feature = "whisper")]
        {
            if let Some(ctx) = &self.whisper_ctx {
                let mut params = FullParams::new(SamplingStrategy::Greedy);
                params.set_initial_prompt(&self.custom_vocabulary);
                if let Ok(mut state) = ctx.create_state() {
                    if state.full(params, audio_data).is_ok() {
                        let num_segments = state.full_n_segments().unwrap_or(0);
                        return (0..num_segments)
                            .map(|i| state.full_get_segment_text(i).unwrap_or_default())
                            .collect::<Vec<String>>()
                            .join("");
                    }
                }
            }
        }

        if let Ok(res) = crate::engine::voice::local_stt_trans::transcribe_samples_sync(audio_data) {
            return res;
        }
        String::new()
    }
}
