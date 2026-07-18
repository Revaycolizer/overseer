use std::sync::Arc;
use std::sync::OnceLock;
use super::pool::BrowserPool;

static POOL: OnceLock<Arc<BrowserPool>> = OnceLock::new();

pub fn init(pool: Arc<BrowserPool>) {
    let _ = POOL.set(pool);
}

pub fn get() -> Option<&'static Arc<BrowserPool>> {
    POOL.get()
}
