pub mod lru;
pub use lru::LruCache;

use crate::backend::Message;
use std::sync::{Arc, OnceLock, RwLock};

pub type MessageCache = LruCache<i64, Vec<Message>>;

static MESSAGES_CACHE: OnceLock<Arc<RwLock<MessageCache>>> = OnceLock::new();

pub fn messages() -> Arc<RwLock<MessageCache>> {
    MESSAGES_CACHE
        .get_or_init(|| Arc::new(RwLock::new(LruCache::new(32))))
        .clone()
}