//! Double-clap detector (no model required — pure signal processing).
//!
//! Opens the default input and watches short windows for a sharp transient
//! (a clap: loud peak right after quiet). Two claps within a time window fire the
//! wake callback. This is heuristic and can false-trigger on other loud, sharp
//! sounds; thresholds are conservative and it is opt-in.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;

pub static IS_CLAP_LISTENING: AtomicBool = AtomicBool::new(false);

/// Stop any running detector.
pub fn stop() {
    IS_CLAP_LISTENING.store(false, Ordering::SeqCst);
}

/// Start listening for double-claps; `tx.send(())` fires once per detected pair.
pub fn start(tx: mpsc::Sender<()>) {
    if IS_CLAP_LISTENING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = run(tx) {
            tracing::warn!(?e, "clap detector stopped");
        }
        IS_CLAP_LISTENING.store(false, Ordering::SeqCst);
    });
}

fn run(tx: mpsc::Sender<()>) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("no input device"))?;
    let native = device.default_input_config()?;
    let cfg = native.config();
    let channels = cfg.channels as usize;

    let rb = HeapRb::<f32>::new(48_000);
    let (mut prod, mut cons) = rb.split();

    let stream = match native.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &cfg,
            move |data: &[f32], _: &_| {
                for c in data.chunks(channels.max(1)) {
                    let _ = prod.push(c[0].abs());
                }
            },
            |err| tracing::debug!(?err, "clap stream error"),
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &cfg,
            move |data: &[i16], _: &_| {
                for c in data.chunks(channels.max(1)) {
                    let _ = prod.push((c[0] as f32 / 32768.0).abs());
                }
            },
            |err| tracing::debug!(?err, "clap stream error"),
            None,
        )?,
        _ => return Err(anyhow::anyhow!("unsupported audio format")),
    };
    stream.play()?;
    tracing::info!("clap detector active");

    // Transient detection over ~20ms windows.
    const WIN: usize = 320; // ~20ms @ 16k-ish (native rate is fine for peaks)
    const LOUD: f32 = 0.20; // clap peak threshold (lower = more sensitive)
    const QUIET: f32 = 0.06; // must fall back below this between claps
    const MIN_GAP: Duration = Duration::from_millis(120);
    const MAX_GAP: Duration = Duration::from_millis(650);
    const COOLDOWN: Duration = Duration::from_millis(1200);

    let mut win: Vec<f32> = Vec::with_capacity(WIN);
    let mut armed = true; // ready to accept a (first) clap once it's quiet
    let mut first_clap: Option<Instant> = None;
    let mut last_fire = Instant::now() - COOLDOWN;

    loop {
        if !IS_CLAP_LISTENING.load(Ordering::SeqCst) {
            break;
        }
        // Skip while speaking (TTS) or while a voice utterance is being recorded,
        // so we don't fight the recorder for the mic or mishear our own output.
        if super::tts::is_tts_speaking() || super::local_stt_rec::RECORDING.load(Ordering::SeqCst) {
            while cons.pop().is_some() {}
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        while let Some(s) = cons.pop() {
            win.push(s);
            if win.len() < WIN {
                continue;
            }
            let peak = win.iter().cloned().fold(0.0f32, f32::max);
            win.clear();

            if peak < QUIET {
                armed = true;
                // A first clap that isn't followed by a second in time expires.
                if let Some(t) = first_clap {
                    if t.elapsed() > MAX_GAP {
                        first_clap = None;
                    }
                }
            } else if peak >= LOUD && armed {
                armed = false;
                tracing::info!(peak, "clap: transient detected"); // tune LOUD from this
                let now = Instant::now();
                match first_clap {
                    Some(t) if now.duration_since(t) >= MIN_GAP && now.duration_since(t) <= MAX_GAP => {
                        if last_fire.elapsed() >= COOLDOWN {
                            let _ = tx.send(());
                            last_fire = now;
                            tracing::info!("double-clap detected");
                        }
                        first_clap = None;
                    }
                    _ => {
                        first_clap = Some(now);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
