//! Hands-free wake service: watches for a wake trigger (double-clap and/or the
//! "Hey Overseer" wake word) and broadcasts a wake signal that connected clients
//! (the floating widget) use to summon themselves.
//!
//! Opt-in and off by default. Enable with the `OVERSEER_WAKE` env var or the
//! `wake_mode` setting: "clap", "word", or "both".
//!   OVERSEER_WAKE=both ./overseer server

use crate::db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

pub async fn run_wake_service(db: Arc<Database>, wake_tx: broadcast::Sender<()>) {
    let mode = match std::env::var("OVERSEER_WAKE") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_lowercase(),
        _ => crate::backend::get_setting_or(&db, "wake_mode", "").await.to_lowercase(),
    };
    if mode.is_empty() || mode == "off" {
        tracing::info!("wake service disabled (set OVERSEER_WAKE=clap|word|both to enable)");
        return;
    }

    let clap = mode == "clap" || mode == "both";
    let word = mode == "word" || mode == "both";
    tracing::info!(%mode, "wake service starting");

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    if clap {
        crate::engine::voice::clap_detect::start(tx);
    }
    if word {
        crate::engine::voice::wake_word::start_wake_word_listener();
    }

    loop {
        // Clap detections arrive on the mpsc channel.
        while rx.try_recv().is_ok() {
            let _ = wake_tx.send(());
        }
        // Wake-word detections are polled from the listener.
        if word && crate::engine::voice::wake_word::try_recv_wake() {
            let _ = wake_tx.send(());
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
}
