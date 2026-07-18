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