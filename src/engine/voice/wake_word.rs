use rustpotter::{Rustpotter, RustpotterConfig, SampleFormat, AudioFmt, Endianness};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use ringbuf::HeapRb;
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Mutex};

pub static IS_LISTENING: AtomicBool = AtomicBool::new(false);
static WAKE_TX: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static WAKE_RX: Mutex<Option<mpsc::Receiver<()>>> = Mutex::new(None);

/// Check if a wake word was detected (non-blocking).
pub fn try_recv_wake() -> bool {
    if let Ok(guard) = WAKE_RX.lock() {
        if let Some(rx) = guard.as_ref() {
            return rx.try_recv().is_ok();
        }
    }
    false
}

pub fn start_wake_word_listener() {
    if IS_LISTENING.load(Ordering::SeqCst) { return; }
    IS_LISTENING.store(true, Ordering::SeqCst);

    let (tx, rx) = mpsc::channel();
    *WAKE_TX.lock().unwrap() = Some(tx);
    *WAKE_RX.lock().unwrap() = Some(rx);

    std::thread::spawn(move || {
        let model_path = "assets/voice/models/personal.rpw";
        let _ = super::training::rebuild_model_if_needed(model_path);
        let res = run_listener(model_path);
        IS_LISTENING.store(false, Ordering::SeqCst);
        if let Err(e) = res {
            eprintln!("❌ Wake-word listener error: {}", e);
        }
    });
}

fn run_listener(model_path: &str) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No input device"))?;

    // Force mono 16kHz — required for Rustpotter MFCC to match training samples
    let stream_config = StreamConfig {
        channels: 1,
        sample_rate: 16000,
        buffer_size: cpal::BufferSize::Default,
    };

    let mut rp_config = RustpotterConfig::default();
    rp_config.fmt = AudioFmt {
        sample_rate: 16000,
        sample_format: SampleFormat::F32,
        channels: 1,
        endianness: Endianness::Little,
    };
    let mut detector = Rustpotter::new(&rp_config).map_err(|e| anyhow::anyhow!(e))?;

    if std::path::Path::new(model_path).exists() {
        detector.add_wakeword_from_file("hey_overseer", model_path)
            .map_err(|e| anyhow::anyhow!(e))?;
        println!("🎙️ Loaded personal wake-word model");
    } else {
        println!("⚠️ No wake-word model found — voice activation disabled");
        return Ok(());
    }

    let rb = HeapRb::<f32>::new(16000 * 2);
    let (mut producer, mut consumer) = rb.split();

    // Keep stream alive with a named binding (NOT _stream — underscore drops immediately!)
    let stream = device.build_input_stream(
        &stream_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for &sample in data { let _ = producer.push(sample); }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;
    stream.play()?;
    println!("🎙️ Wake-word listener active and monitoring...");

    let mut audio_buffer = Vec::new();
    loop {
        while let Some(s) = consumer.pop() { audio_buffer.push(s); }
        while audio_buffer.len() >= detector.get_samples_per_frame() {
            let frame: Vec<f32> = audio_buffer.drain(0..detector.get_samples_per_frame()).collect();
            if let Some(detection) = detector.process_samples(frame) {
                println!("🚀 WAKE WORD DETECTED: {:?}", detection.name);
                if let Ok(guard) = WAKE_TX.lock() {
                    if let Some(tx) = guard.as_ref() { let _ = tx.send(()); }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
