use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;

pub async fn record_samples(vad_threshold: f32) -> Result<Vec<f32>, String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    let stream_config = config.config();
    let native_rate = stream_config.sample_rate as f32;
    let step = native_rate / 16000.0f32;
    let channels = stream_config.channels as usize;
    let rb = HeapRb::<f32>::new(16000 * 10);
    let (mut prod, mut cons) = rb.split();
    let is_rec = Arc::new(AtomicBool::new(true));
    let is_rec_clone = is_rec.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let (mut res_idx, mut prev) = (0.0f32, 0.0f32);
            device.build_input_stream(&config.config(), move |data: &[f32], _| {
                if !is_rec_clone.load(Ordering::SeqCst) { return; }
                let mono: Vec<f32> = if channels > 1 {
                    data.chunks_exact(channels).map(|c| c.iter().sum::<f32>() / channels as f32).collect()
                } else { data.to_vec() };
                for sample in mono {
                    while res_idx < 1.0 {
                        let _ = prod.push(prev + res_idx * (sample - prev));
                        res_idx += step;
                    }
                    res_idx -= 1.0; prev = sample;
                }
            }, |err| eprintln!("Audio stream error: {}", err), None)
        }
        cpal::SampleFormat::I16 => {
            let (mut res_idx, mut prev) = (0.0f32, 0.0f32);
            device.build_input_stream(&config.config(), move |data: &[i16], _| {
                if !is_rec_clone.load(Ordering::SeqCst) { return; }
                let mono: Vec<f32> = if channels > 1 {
                    data.chunks_exact(channels).map(|c| c.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32).collect()
                } else { data.iter().map(|&s| s as f32 / 32768.0).collect() };
                for sample in mono {
                    while res_idx < 1.0 {
                        let _ = prod.push(prev + res_idx * (sample - prev));
                        res_idx += step;
                    }
                    res_idx -= 1.0; prev = sample;
                }
            }, |err| eprintln!("Audio stream error: {}", err), None)
        }
        _ => return Err("Unsupported format".to_string()),
    }.map_err(|e| format!("Failed to build stream: {}", e))?;

    stream.play().map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    let (mut last_speech, mut speech_detected) = (std::time::Instant::now(), false);
    let mut temp = Vec::new();

    while start.elapsed() < std::time::Duration::from_secs(8) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        while let Some(s) = cons.pop() { temp.push(s); }
        let rms_check_len = 160;
        if temp.len() >= rms_check_len {
            let rms = (temp.iter().rev().take(rms_check_len).map(|&x| x * x).sum::<f32>() / rms_check_len as f32).sqrt();
            if rms > vad_threshold {
                speech_detected = true;
                last_speech = std::time::Instant::now();
            }
        }
        if speech_detected && last_speech.elapsed() > std::time::Duration::from_millis(1500) { break; }
    }

    is_rec.store(false, Ordering::SeqCst);
    let _ = stream.pause();
    drop(stream);
    while let Some(s) = cons.pop() { temp.push(s); }
    Ok(temp)
}
