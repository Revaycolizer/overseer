use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + std::hash::Hash + Clone, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "LruCache capacity must be > 0");
        Self { capacity, map: HashMap::with_capacity(capacity), order: VecDeque::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize { self.map.len() }
    pub fn contains(&self, key: &K) -> bool { self.map.contains_key(key) }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if !self.map.contains_key(key) { return None; }
        self.order.retain(|k| k == key);
        self.order.push_back(key.clone());
        self.map.get_mut(key)
    }

    pub fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.map.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    pub fn invalidate(&mut self, key: &K) {
        self.map.remove(key);
        self.order.retain(|k| k != key);
    }
}

static MESSAGES_CACHE: OnceLock<Arc<Mutex<LruCache<i64, Vec<crate::backend::Message>>>>> = OnceLock::new();

pub fn messages_cache() -> Arc<Mutex<LruCache<i64, Vec<crate::backend::Message>>>> {
    MESSAGES_CACHE.get_or_init(|| Arc::new(Mutex::new(LruCache::new(32)))).clone()
}