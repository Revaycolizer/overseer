use super::pool::BrowserPool;
use std::sync::Arc;
use std::sync::OnceLock;

static POOL: OnceLock<Arc<BrowserPool>> = OnceLock::new();

pub fn init(pool: Arc<BrowserPool>) {
    let _ = POOL.set(pool);
}

pub fn get() -> Option<&'static Arc<BrowserPool>> {
    POOL.get()
}
