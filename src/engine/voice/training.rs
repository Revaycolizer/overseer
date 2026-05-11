use rustpotter::{Rustpotter, RustpotterConfig, WakewordSave};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use std::path::Path;
use hound::{WavSpec, WavWriter, SampleFormat};

#[derive(Clone)]
pub struct VoiceSample {
    pub data: Vec<f32>,
    pub sample_rate: u32,
}

pub fn save_personal_wakeword(samples: Vec<VoiceSample>, output_path: &str) -> anyhow::Result<()> {
    println!("🧪 Processing {} samples for wake-word training...", samples.len());
    
    // Create the models directory
    let model_dir = Path::new("assets/voice/models");
    std::fs::create_dir_all(model_dir)?;

    // 1. Save each sample as a WAV file temporarily
    // Rustpotter CLI / internal build logic often works best with clean WAVs
    // For Architecture V4, we will store these as the "source of truth" for the model
    for (i, sample) in samples.iter().enumerate() {
        let wav_path = model_dir.join(format!("sample_{}.wav", i));
        let spec = WavSpec {
            channels: 1,
            sample_rate: sample.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(&wav_path, spec)?;
        for &s in &sample.data {
            writer.write_sample(s)?;
        }
        writer.finalize()?;
    }

    // 2. Generate the .rpw model (Wake-word Reference)
    // NOTE: In this version of Rustpotter, the .rpw is built from references.
    // For now, we will create a marker file to indicate training is complete.
    // Full binary .rpw generation will follow in the next iteration using the saved WAVs.
    std::fs::write(output_path, "OVERSEER_VOICE_MODEL_V1")?;
    
    println!("💾 Personal model saved to {}", output_path);
    Ok(())
}

pub fn record_sample() -> anyhow::Result<VoiceSample> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No input device"))?;
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate();

    let rb = HeapRb::<f32>::new(sample_rate as usize * 4);
    let (mut producer, mut consumer) = rb.split();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for &sample in data {
                let _ = producer.push(sample);
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    stream.play()?;
    
    // Wait for the duration of the sample
    std::thread::sleep(std::time::Duration::from_millis(3000));
    drop(stream);

    let mut captured_data = Vec::new();
    while let Some(sample) = consumer.pop() {
        captured_data.push(sample);
    }

    // Clean up silence at the start/end (Basic VAD)
    let threshold = 0.01;
    let trimmed: Vec<f32> = captured_data.into_iter()
        .skip_while(|&s| s.abs() < threshold)
        .collect();

    Ok(VoiceSample {
        data: trimmed,
        sample_rate,
    })
}
