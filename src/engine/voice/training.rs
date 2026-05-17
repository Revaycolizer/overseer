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

fn build_and_save_model(sample_paths: Vec<String>, output_path: &str) -> anyhow::Result<()> {
    let wakeword_ref = WakewordRef::new_from_sample_files(
        "hey_overseer".to_string(), None, None, sample_paths, 13,
    ).map_err(|e| anyhow::anyhow!(e))?;
    wakeword_ref.save_to_file(output_path).map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}

pub fn save_personal_wakeword(samples: Vec<VoiceSample>, output_path: &str) -> anyhow::Result<()> {
    println!("🧪 Processing {} samples for wake-word training...", samples.len());
    let model_dir = Path::new("assets/voice/models");
    std::fs::create_dir_all(model_dir)?;

    for (i, sample) in samples.iter().enumerate() {
        let wav_path = model_dir.join(format!("sample_{}.wav", i));
        let spec = WavSpec {
            channels: 1, sample_rate: sample.sample_rate,
            bits_per_sample: 32, sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(&wav_path, spec)?;
        for &s in &sample.data { writer.write_sample(s)?; }
        writer.finalize()?;
    }

    let paths: Vec<String> = (0..samples.len())
        .map(|i| model_dir.join(format!("sample_{}.wav", i)).to_string_lossy().to_string())
        .collect();
    build_and_save_model(paths, output_path)?;
    println!("💾 Personal wake-word model saved to {}", output_path);
    Ok(())
}

/// Rebuild the .rpw model from existing WAV samples if they exist
pub fn rebuild_model_if_needed(output_path: &str) -> anyhow::Result<bool> {
    let model_dir = Path::new("assets/voice/models");
    if !model_dir.exists() { return Ok(false); }
    let paths: Vec<String> = (0..5)
        .map(|i| model_dir.join(format!("sample_{}.wav", i)))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    if paths.len() < 3 { return Ok(false); }
    println!("🔄 Rebuilding wake-word model from {} samples...", paths.len());
    build_and_save_model(paths, output_path)?;
    println!("✅ Wake-word model rebuilt successfully");
    Ok(true)
}

pub fn record_sample() -> anyhow::Result<VoiceSample> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No input device"))?;
    // Force 16kHz mono to match the wake-word detection pipeline
    let stream_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: 16000,
        buffer_size: cpal::BufferSize::Default,
    };
    let sample_rate = 16000u32;
    let rb = HeapRb::<f32>::new(sample_rate as usize * 4);
    let (mut producer, mut consumer) = rb.split();
    let stream = device.build_input_stream(
        &stream_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for &sample in data { let _ = producer.push(sample); }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;
    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(3000));
    drop(stream);
    let mut captured_data = Vec::new();
    while let Some(sample) = consumer.pop() { captured_data.push(sample); }
    let threshold = 0.01;
    let trimmed: Vec<f32> = captured_data.into_iter()
        .skip_while(|&s| s.abs() < threshold)
        .collect();
    Ok(VoiceSample { data: trimmed, sample_rate })
}
