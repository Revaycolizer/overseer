use std::sync::atomic::{AtomicU64, Ordering};

/// Per-app monotonic counter bumped by any conversation-mutating call
/// (add_message, update_conversation_title, add_conversation). The
/// TUI main loop watches this counter instead of polling on a fixed
/// interval so idle CPU drops and writes are reflected immediately.
///
/// Engine-side background writes (LLM-refined title, future async DB
/// writes) live in a separate `EngineActivity` counter that the TUI
/// also watches, so this counter is purely TUI-side.
#[derive(Default)]
pub struct ActivityCounter(AtomicU64);

impl ActivityCounter {
    pub fn bump(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }

    pub fn current(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn changed_since(&self, last_seen: u64) -> bool {
        self.0.load(Ordering::Relaxed) != last_seen
    }
}