use rustpotter::{WakewordRef, WakewordRefBuildFromFiles, WakewordSave};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use hound::{WavSpec, WavWriter, SampleFormat};

#[derive(Clone)]
pub struct VoiceSample {
    pub data: Vec<f32>,
    pub sample_rate: u32,
}

fn build_and_save_model(paths: Vec<String>, out: &str) -> anyhow::Result<()> {
    WakewordRef::new_from_sample_files("hey_overseer".to_string(), None, None, paths, 13)
        .map_err(|e| anyhow::anyhow!(e))?.save_to_file(out).map_err(|e| anyhow::anyhow!(e))
}

pub fn save_personal_wakeword(samples: Vec<VoiceSample>, out: &str) -> anyhow::Result<()> {
    tracing::info!(count = samples.len(), "Processing samples for wake-word training");
    let data_dir = crate::db::get_overseer_data_dir();
    let model_dir = data_dir.join("assets").join("voice").join("models");
    std::fs::create_dir_all(&model_dir)?;
    let paths: Vec<String> = samples.iter().enumerate().map(|(i, s)| {
        let max_amp = s.data.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        tracing::debug!(?i, len = s.data.len(), ?max_amp, "Processing training sample");
        let p = model_dir.join(format!("sample_{}.wav", i));
        let spec = WavSpec { channels: 1, sample_rate: s.sample_rate, bits_per_sample: 32, sample_format: SampleFormat::Float };
        let mut writer = WavWriter::create(&p, spec).unwrap();
        for &val in &s.data { let _ = writer.write_sample(val); }
        let _ = writer.finalize();
        p.to_string_lossy().to_string()
    }).collect();
    build_and_save_model(paths, out)
}

pub fn rebuild_model_if_needed(out: &str) -> anyhow::Result<bool> {
    let data_dir = crate::db::get_overseer_data_dir();
    let model_dir = data_dir.join("assets").join("voice").join("models");
    if !model_dir.exists() { return Ok(false); }
    let paths: Vec<String> = (0..5)
        .map(|i| model_dir.join(format!("sample_{}.wav", i)))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    if paths.len() < 3 { return Ok(false); }
    build_and_save_model(paths, out)?;
    Ok(true)
}

pub fn record_sample() -> anyhow::Result<VoiceSample> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No input device"))?;
    
    let native_config = device.default_input_config().map_err(|e| anyhow::anyhow!(e))?;
    let sample_format = native_config.sample_format();
    let stream_config = native_config.config();
    
    let native_rate = stream_config.sample_rate as f32;
    let target_rate = 16000.0f32;
    let step = native_rate / target_rate;
    let channels = stream_config.channels as usize;
    
    let rb = HeapRb::<f32>::new(16000 * 4);
    let (mut prod, mut cons) = rb.split();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let mut resample_index = 0.0f32;
            let mut prev_sample = 0.0f32;
            device.build_input_stream(&stream_config, move |data: &[f32], _: &_| {
                let mono: Vec<f32> = if channels > 1 {
                    data.chunks_exact(channels)
                        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                        .collect()
                } else {
                    data.to_vec()
                };
                for sample in mono {
                    while resample_index < 1.0 {
                        let interpolated = prev_sample + resample_index * (sample - prev_sample);
                        let _ = prod.push(interpolated);
                        resample_index += step;
                    }
                    resample_index -= 1.0;
                    prev_sample = sample;
                }
            }, |err| tracing::error!(?err, "Audio stream error"), None)?
        }
        cpal::SampleFormat::I16 => {
            let mut resample_index = 0.0f32;
            let mut prev_sample = 0.0f32;
            device.build_input_stream(&stream_config, move |data: &[i16], _: &_| {
                let mono: Vec<f32> = if channels > 1 {
                    data.chunks_exact(channels)
                        .map(|chunk| chunk.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32)
                        .collect()
                } else {
                    data.iter().map(|&s| s as f32 / 32768.0).collect()
                };
                for sample in mono {
                    while resample_index < 1.0 {
                        let interpolated = prev_sample + resample_index * (sample - prev_sample);
                        let _ = prod.push(interpolated);
                        resample_index += step;
                    }
                    resample_index -= 1.0;
                    prev_sample = sample;
                }
            }, |err| tracing::error!(?err, "Audio stream error"), None)?
        }
        _ => return Err(anyhow::anyhow!("Unsupported audio format")),
    };

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(3000));
    drop(stream);

    let mut captured_data = Vec::new();
    while let Some(s) = cons.pop() { captured_data.push(s); }
    if captured_data.is_empty() { return Ok(VoiceSample { data: captured_data, sample_rate: 16000 }); }

    // 1. Remove DC offset
    let mean: f32 = captured_data.iter().sum::<f32>() / captured_data.len() as f32;
    for s in &mut captured_data { *s -= mean; }

    // 2. Double-ended silence trimming with 150ms padding
    let first = captured_data.iter().position(|&s| s.abs() >= 0.010).unwrap_or(0);
    let last = captured_data.iter().rposition(|&s| s.abs() >= 0.010).unwrap_or(captured_data.len() - 1);
    let trimmed = if first < last {
        captured_data[first.saturating_sub(2400)..=std::cmp::min(last + 2400, captured_data.len() - 1)].to_vec()
    } else {
        captured_data
    };
    Ok(VoiceSample { data: trimmed, sample_rate: 16000 })
}
