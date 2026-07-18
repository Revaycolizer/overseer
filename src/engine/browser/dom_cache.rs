use std::time::Instant;

pub struct DomCache {
    pub text: String,
    pub cached_at: Instant,
}

impl DomCache {
    pub fn is_fresh(&self, ttl_secs: u64) -> bool {
        self.cached_at.elapsed().as_secs() < ttl_secs
    }
}
