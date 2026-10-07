use rustpotter::{Rustpotter, RustpotterConfig, SampleFormat, AudioFmt, Endianness};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer}};
use std::sync::{atomic::{AtomicBool, AtomicU32, Ordering}, mpsc, Mutex};

pub static IS_LISTENING: AtomicBool = AtomicBool::new(false);
pub static EMERGENCY_PANIC_TRIGGERED: AtomicBool = AtomicBool::new(false);
static CALIBRATED_VAD_THRESHOLD: AtomicU32 = AtomicU32::new(1011681280); // Default 0.015f32.to_bits()

static WAKE_TX: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static WAKE_RX: Mutex<Option<mpsc::Receiver<()>>> = Mutex::new(None);

pub fn get_vad_threshold() -> f32 {
    f32::from_bits(CALIBRATED_VAD_THRESHOLD.load(Ordering::SeqCst))
}

pub fn set_vad_threshold(val: f32) {
    CALIBRATED_VAD_THRESHOLD.store(val.to_bits(), Ordering::SeqCst);
}

pub fn try_recv_panic() -> bool {
    EMERGENCY_PANIC_TRIGGERED.swap(false, Ordering::SeqCst)
}

pub fn try_recv_wake() -> bool {
    WAKE_RX.lock().ok().and_then(|g| g.as_ref().map(|rx| rx.try_recv().is_ok())).unwrap_or(false)
}

pub fn start_wake_word_listener() {
    if IS_LISTENING.swap(true, Ordering::SeqCst) { return; }
    let (tx, rx) = mpsc::channel();
    *WAKE_TX.lock().unwrap() = Some(tx);
    *WAKE_RX.lock().unwrap() = Some(rx);
    std::thread::spawn(move || {
        super::priority::set_thread_priority_and_affinity();
        let data_dir = crate::db::get_overseer_data_dir();
        let model_dir = data_dir.join("assets").join("voice").join("models");
        let _ = std::fs::create_dir_all(&model_dir);
        let model_path = model_dir.join("personal.rpw");
        let model_path_str = model_path.to_string_lossy().to_string();
        let _ = super::training::rebuild_model_if_needed(&model_path_str);
        let res = run_listener(&model_path_str);
        IS_LISTENING.store(false, Ordering::SeqCst);
        if let Err(e) = res { tracing::error!(?e, "Wake-word listener error"); }
    });
}

pub fn restart_wake_word_listener() {
    IS_LISTENING.store(false, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(100));
    start_wake_word_listener();
}

fn run_listener(model_path: &str) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No input device"))?;
    
    let native_config = device.default_input_config().map_err(|e| anyhow::anyhow!(e))?;
    let sample_format = native_config.sample_format();
    let stream_config = native_config.config();
    
    let native_rate = stream_config.sample_rate as f32;
    let target_rate = 16000.0f32;
    let step = native_rate / target_rate;
    let channels = stream_config.channels as usize;

    let mut rp_config = RustpotterConfig::default();
    rp_config.fmt = AudioFmt { sample_rate: 16000, sample_format: SampleFormat::F32, channels: 1, endianness: Endianness::Little };
    rp_config.detector.threshold = 0.35;
    rp_config.detector.min_scores = 2;
    rp_config.detector.avg_threshold = 0.22;
    rp_config.filters.gain_normalizer.enabled = true;
    let mut detector = Rustpotter::new(&rp_config).map_err(|e| anyhow::anyhow!(e))?;

    if std::path::Path::new(model_path).exists() {
        detector.add_wakeword_from_file("hey_overseer", model_path).map_err(|e| anyhow::anyhow!(e))?;
        tracing::info!("Loaded personal wake-word model");
    } else {
        tracing::warn!("No wake-word model found — voice activation disabled");
        return Ok(());
    }

    let rb = HeapRb::<f32>::new(16000 * 2);
    let (mut producer, mut consumer) = rb.split();

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
                        let _ = producer.try_push(interpolated);
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
                        let _ = producer.try_push(interpolated);
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
    tracing::info!(?native_rate, "Wake-word listener active");

    let mut audio_buffer = Vec::new();
    let mut speech_buffer = Vec::new();
    let mut last_print = std::time::Instant::now();
    let (mut max_amp, mut sample_count) = (0.0f32, 0);
    let mut last_trigger = std::time::Instant::now();
    let cooldown = std::time::Duration::from_secs(3);
    
    let recent_levels = ringbuf::HeapRb::<f32>::new(1600);
    let (mut vad_prod, mut vad_cons) = recent_levels.split();
    let mut is_speaking = false;
    let mut was_speaking = false;
    let mut speaking_cooldown = std::time::Instant::now();
    let speaking_gate = std::time::Duration::from_millis(300);

    let mut calibration_samples = Vec::new();
    let mut is_calibrated = false;
    let calibration_duration = std::time::Duration::from_millis(1500);
    let calibration_start = std::time::Instant::now();

    loop {
        if !IS_LISTENING.load(Ordering::SeqCst) { break Ok(()); }
        
        let tts_active = super::tts::is_tts_speaking();
        
        while let Some(s) = consumer.try_pop() {
            if !tts_active {
                audio_buffer.push(s);
                if s.abs() > max_amp { max_amp = s.abs(); }
                sample_count += 1;
                let _ = vad_prod.try_push(s.abs());

                if !is_calibrated {
                    calibration_samples.push(s);
                }

                if is_speaking {
                    // Limit buffer to 5 seconds to avoid memory issues
                    if speech_buffer.len() < 16000 * 5 {
                        speech_buffer.push(s);
                    }
                }
            }
        }

        if !is_calibrated && calibration_start.elapsed() >= calibration_duration {
            if !calibration_samples.is_empty() {
                let sum_sq: f32 = calibration_samples.iter().map(|&x| x * x).sum();
                let rms = (sum_sq / calibration_samples.len() as f32).sqrt();
                let val = (rms * 3.0).clamp(0.005, 0.15);
                set_vad_threshold(val);
                tracing::info!(?rms, threshold = ?val, "Audio calibration complete");
            }
            is_calibrated = true;
        }

        let vad_threshold = get_vad_threshold();

        if last_print.elapsed() >= std::time::Duration::from_secs(2) {
            tracing::debug!(?sample_count, ?max_amp, ?vad_threshold, ?is_speaking, "Audio telemetry");
            max_amp = 0.0; sample_count = 0; last_print = std::time::Instant::now();
        }

        let mut recent_avg = 0.0f32;
        let mut recent_count = 0;
        while let Some(l) = vad_cons.try_pop() {
            recent_avg += l;
            recent_count += 1;
        }
        if recent_count > 0 { recent_avg /= recent_count as f32; }

        if recent_avg > vad_threshold { 
            speaking_cooldown = std::time::Instant::now(); 
            is_speaking = true; 
        } else if speaking_cooldown.elapsed() > speaking_gate { 
            is_speaking = false; 
        }

        if is_speaking {
            was_speaking = true;
        } else if was_speaking {
            was_speaking = false;
            let samples = std::mem::take(&mut speech_buffer);
            if samples.len() > 16000 {
                std::thread::spawn(move || {
                    if let Ok(text) = super::local_stt::transcribe_samples_sync(&samples) {
                        let lower = text.to_lowercase();
                        if lower.contains("system reset") || lower.contains("exit overseer") {
                            tracing::warn!("Emergency voice panic triggered — exiting lock screen");
                            EMERGENCY_PANIC_TRIGGERED.store(true, Ordering::SeqCst);
                        }
                    }
                });
            }
        }

        while audio_buffer.len() >= detector.get_samples_per_frame() {
            let frame: Vec<f32> = audio_buffer.drain(0..detector.get_samples_per_frame()).collect();
            if let Some(detection) = detector.process_samples(frame) {
                if last_trigger.elapsed() >= cooldown {
                    tracing::info!(name = ?detection.name, "Wake word detected");
                    last_trigger = std::time::Instant::now();
                    if let Ok(guard) = WAKE_TX.lock() {
                        if let Some(tx) = guard.as_ref() { let _ = tx.send(()); }
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
