use rustpotter::{Rustpotter, RustpotterConfig, SampleFormat, AudioFmt, Endianness};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use std::sync::atomic::{AtomicBool, Ordering};

pub static IS_LISTENING: AtomicBool = AtomicBool::new(false);

pub fn start_wake_word_listener() {
    if IS_LISTENING.load(Ordering::SeqCst) { return; }
    IS_LISTENING.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        if let Err(e) = run_listener() {
            eprintln!("❌ Wake-word listener error: {}", e);
            IS_LISTENING.store(false, Ordering::SeqCst);
        }
    });
}

fn run_listener() -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No input device"))?;
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate();

    let mut rp_config = RustpotterConfig::default();
    rp_config.fmt = AudioFmt {
        sample_rate: sample_rate as usize,
        sample_format: SampleFormat::F32,
        channels: 1,
        endianness: Endianness::Little,
    };
    let mut detector = Rustpotter::new(&rp_config).map_err(|e| anyhow::anyhow!(e))?;

    // Load the personal model if it exists
    let model_path = "assets/voice/models/personal.rpw";
    if std::path::Path::new(model_path).exists() {
        println!("🎙️ Loading personal wake-word model...");
    }

    let rb = HeapRb::<f32>::new(sample_rate as usize * 2);
    let (mut producer, mut consumer) = rb.split();

    let _stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for &sample in data {
                let _ = producer.push(sample);
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    println!("🎙️ Wake-word listener active and monitoring...");

    let mut audio_buffer = Vec::new();
    loop {
        while let Some(sample) = consumer.pop() {
            audio_buffer.push(sample);
        }

        while audio_buffer.len() >= detector.get_samples_per_frame() {
            let frame: Vec<f32> = audio_buffer.drain(0..detector.get_samples_per_frame()).collect();
            if let Some(_detection) = detector.process_samples(frame) {
                println!("🚀 WAKE WORD DETECTED!");
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
