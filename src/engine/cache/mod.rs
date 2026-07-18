pub mod lru;
pub use lru::LruCache;

use crate::backend::Message;
use std::sync::{Arc, OnceLock, RwLock};

pub trait SharedCache<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn put(&self, key: K, value: V);
    fn invalidate(&self, key: &K);
}

pub struct InMemoryCacheWrapper<K, V>(pub Arc<RwLock<LruCache<K, V>>>);

impl<K: Eq + std::hash::Hash + Clone, V: Clone> SharedCache<K, V> for InMemoryCacheWrapper<K, V> {
    fn get(&self, key: &K) -> Option<V> {
        self.0.read().ok()?.get(key).cloned()
    }
    fn put(&self, key: K, value: V) {
        if let Ok(mut w) = self.0.write() {
            w.put(key, value);
        }
    }
    fn invalidate(&self, key: &K) {
        if let Ok(mut w) = self.0.write() {
            w.invalidate(key);
        }
    }
}

pub type MessageCache = LruCache<i64, Vec<Message>>;

static MESSAGES_CACHE: OnceLock<Arc<RwLock<MessageCache>>> = OnceLock::new();
static WORKFLOW_CACHE: OnceLock<Arc<RwLock<LruCache<String, serde_json::Value>>>> = OnceLock::new();
static BROWSERBASE_CACHE: OnceLock<Arc<RwLock<LruCache<String, serde_json::Value>>>> = OnceLock::new();

pub fn messages() -> Arc<RwLock<MessageCache>> {
    MESSAGES_CACHE
        .get_or_init(|| Arc::new(RwLock::new(LruCache::new(32))))
        .clone()
}

pub fn workflows() -> Arc<RwLock<LruCache<String, serde_json::Value>>> {
    WORKFLOW_CACHE
        .get_or_init(|| Arc::new(RwLock::new(LruCache::new(64))))
        .clone()
}

pub fn browserbase() -> Arc<RwLock<LruCache<String, serde_json::Value>>> {
    BROWSERBASE_CACHE
        .get_or_init(|| Arc::new(RwLock::new(LruCache::new(16))))
        .clone()
}