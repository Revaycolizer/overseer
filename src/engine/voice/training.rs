use rustpotter::{WakewordRef, WakewordRefBuildFromFiles, WakewordSave};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use std::path::Path;
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
    println!("🧪 Processing {} samples for wake-word training...", samples.len());
    let model_dir = Path::new("assets/voice/models");
    std::fs::create_dir_all(model_dir)?;
    let paths: Vec<String> = samples.iter().enumerate().map(|(i, s)| {
        let max_amp = s.data.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        println!("  - Sample {}: Len = {} samples | Max Amplitude = {:.5}", i, s.data.len(), max_amp);
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
    let model_dir = Path::new("assets/voice/models");
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
    let stream_config = cpal::StreamConfig { channels: 1, sample_rate: 16000, buffer_size: cpal::BufferSize::Default };
    let sample_rate = 16000u32;
    let rb = HeapRb::<f32>::new(sample_rate as usize * 4);
    let (mut prod, mut cons) = rb.split();
    let stream = device.build_input_stream(&stream_config, move |data: &[f32], _: &_| {
        for &s in data { let _ = prod.push(s); }
    }, |err| eprintln!("Audio stream error: {}", err), None)?;
    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(3000));
    drop(stream);
    let mut captured_data = Vec::new();
    while let Some(s) = cons.pop() { captured_data.push(s); }
    if captured_data.is_empty() { return Ok(VoiceSample { data: captured_data, sample_rate }); }

    // 1. Remove DC offset
    let mean: f32 = captured_data.iter().sum::<f32>() / captured_data.len() as f32;
    for s in &mut captured_data { *s -= mean; }

    // 2. Double-ended silence trimming with 100ms padding
    let first = captured_data.iter().position(|&s| s.abs() >= 0.015).unwrap_or(0);
    let last = captured_data.iter().rposition(|&s| s.abs() >= 0.015).unwrap_or(captured_data.len() - 1);
    let trimmed = if first < last {
        captured_data[first.saturating_sub(1600)..=std::cmp::min(last + 1600, captured_data.len() - 1)].to_vec()
    } else {
        captured_data
    };
    Ok(VoiceSample { data: trimmed, sample_rate })
}
