pub mod lru;
pub use lru::LruCache;

use crate::backend::Message;
use std::sync::{Arc, Mutex, OnceLock};

pub type MessageCache = LruCache<i64, Vec<Message>>;

static MESSAGES_CACHE: OnceLock<Arc<Mutex<MessageCache>>> = OnceLock::new();

pub fn messages() -> Arc<Mutex<MessageCache>> {
    MESSAGES_CACHE
        .get_or_init(|| Arc::new(Mutex::new(LruCache::new(32))))
        .clone()
}