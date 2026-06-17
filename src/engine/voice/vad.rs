use std::path::PathBuf;
use crate::db::get_overseer_data_dir;
use ort::session::Session;
use ort::value::Tensor;

pub struct SileroVad {
    session: Option<Session>,
    state: Vec<f32>,
}

impl SileroVad {
    pub fn new() -> Self {
        let model_dir = get_overseer_data_dir().join("assets").join("voice").join("models");
        let model_path = model_dir.join("silero_vad.onnx");
        if !model_path.exists() {
            let _ = std::fs::create_dir_all(&model_dir);
            if let Ok(mut resp) = reqwest::blocking::get("https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx") {
                if let Ok(mut file) = std::fs::File::create(&model_path) {
                    let _ = std::io::copy(&mut resp, &mut file);
                }
            }
        }
        let session = Session::builder()
            .and_then(|mut b| b.commit_from_file(&model_path))
            .ok();

        Self {
            session,
            state: vec![0.0; 2 * 1 * 128],
        }
    }

    pub fn is_speech(&mut self, audio_data: &[f32]) -> bool {
        let session = match &mut self.session {
            Some(s) => s,
            None => {
                if audio_data.is_empty() { return false; }
                let rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
                return rms > crate::engine::voice::wake_word::get_vad_threshold();
            }
        };

        let mut chunk = vec![0.0f32; 512];
        let copy_len = audio_data.len().min(512);
        chunk[..copy_len].copy_from_slice(&audio_data[..copy_len]);

        let input_tensor = Tensor::from_array(([1usize, 512], chunk));
        let state_tensor = Tensor::from_array(([2usize, 1, 128], self.state.clone()));
        let sr_tensor = Tensor::from_array(([0usize; 0], vec![16000i64]));

        if let (Ok(input_t), Ok(state_t), Ok(sr_t)) = (input_tensor, state_tensor, sr_tensor) {
            if let Ok(outputs) = session.run(ort::inputs![
                "input" => input_t,
                "state" => state_t,
                "sr" => sr_t
            ]) {
                if let Some(prob_val) = outputs.get("output") {
                    if let Ok(prob_tensor) = prob_val.try_extract_tensor::<f32>() {
                        let prob = *prob_tensor.1.first().unwrap_or(&0.0f32);
                        if let Some(state_val) = outputs.get("stateN").or_else(|| outputs.get("output_state")) {
                            if let Ok(state_tensor) = state_val.try_extract_tensor::<f32>() {
                                self.state = state_tensor.1.to_vec();
                            }
                        }
                        return prob > 0.5;
                    }
                }
            }
        }
        false
    }
}
