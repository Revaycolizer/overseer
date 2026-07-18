use std::sync::atomic::{AtomicU64, Ordering};

/// Global monotonic counter for engine-side mutations that happen
/// outside the caller's task (LLM-refined title writes, future
/// background DB writes, etc). TUI and any other watcher bumps
/// the counter on every change so its change-driven refresh picks
/// up the update on the next loop iteration.
pub struct EngineActivity {
    counter: AtomicU64,
}

impl EngineActivity {
    pub const fn new() -> Self {
        Self { counter: AtomicU64::new(0) }
    }

    pub fn bump(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }

    pub fn current(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }

    pub fn changed_since(&self, last_seen: u64) -> bool {
        self.current() != last_seen
    }
}

static ENGINE_ACTIVITY: EngineActivity = EngineActivity::new();

pub fn engine_activity() -> &'static EngineActivity {
    &ENGINE_ACTIVITY
}

static RUN_CANCELLATIONS: std::sync::OnceLock<std::sync::RwLock<std::collections::HashMap<i64, tokio::sync::watch::Sender<bool>>>> = std::sync::OnceLock::new();

pub fn register_cancellation(run_id: i64) -> tokio::sync::watch::Receiver<bool> {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let lock = RUN_CANCELLATIONS.get_or_init(|| std::sync::RwLock::new(std::collections::HashMap::new()));
    if let Ok(mut map) = lock.write() {
        map.insert(run_id, tx);
    }
    rx
}

pub fn trigger_cancellation(run_id: i64) {
    if let Some(lock) = RUN_CANCELLATIONS.get() {
        if let Ok(map) = lock.read() {
            if let Some(tx) = map.get(&run_id) {
                let _ = tx.send(true);
            }
        }
    }
}

pub fn remove_cancellation(run_id: i64) {
    if let Some(lock) = RUN_CANCELLATIONS.get() {
        if let Ok(mut map) = lock.write() {
            map.remove(&run_id);
        }
    }
}

pub async fn force_unload_ollama_model(model_name: &str) {
    let base = std::env::var("OLLAMA_API_BASE")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let client = reqwest::Client::new();
    let _ = client.post(format!("{}/api/chat", base))
        .json(&serde_json::json!({
            "model": model_name,
            "keep_alive": 0
        }))
        .send()
        .await;
}