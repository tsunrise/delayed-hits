pub mod lru;
pub mod lru_mad;

use std::{
    fmt::Debug,
    hash::{Hash, Hasher as _},
};

use ahash::AHasher;
use proj_models::TimeUnit;

pub trait ObjectId: Hash + Eq + PartialEq + Clone + Debug {
    fn get_hash(&self) -> u64 {
        let mut hasher = AHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Hash + Eq + PartialEq + Clone + Debug> ObjectId for T {}

/// A cache store with no values stored.
pub trait Cache<K: ObjectId, V> {
    const NAME: &'static str;
    /// Write a key in the cache. Evict will happen if the cache is full.
    /// `write` should be called only when
    /// - A key is accessed and there is a miss and the miss has been fetched from the backing store.
    /// - We need to update the value of a key in the cache.
    ///
    /// `timestamp` is only used for heuristics for the eviction policy (to compute the estimated TTNA)
    fn write(&mut self, key: K, value: V, timestamp: TimeUnit);

    /// Get the value of a key in the cache, and the cache might update its internal state corresponding to the access.
    fn get(&mut self, key: &K, timestamp: TimeUnit) -> Option<&V>;

    /// Check if the key is in the cache.
    fn contains(&self, key: &K) -> bool;
}

pub struct MultiCache<K: ObjectId, V, C: Cache<K, V>> {
    caches: Vec<C>,
    _phantom: std::marker::PhantomData<(K, V)>,
}

pub fn construct_k_way_cache<K: ObjectId, V, C: Cache<K, V>>(
    k: usize,
    constructor: impl Fn(usize) -> C,
) -> MultiCache<K, V, C> {
    MultiCache {
        caches: (0..k).map(constructor).collect(),
        _phantom: std::marker::PhantomData,
    }
}

fn get_cache_idx<K: ObjectId>(k: usize, key: &K) -> usize {
    let hash = key.get_hash();
    hash as usize % k
}

impl<K: ObjectId, V, C: Cache<K, V>> Cache<K, V> for MultiCache<K, V, C> {
    const NAME: &'static str = C::NAME;

    fn write(&mut self, key: K, value: V, timestamp: TimeUnit) {
        let idx = get_cache_idx(self.caches.len(), &key);
        self.caches[idx].write(key, value, timestamp);
    }

    fn get(&mut self, key: &K, timestamp: TimeUnit) -> Option<&V> {
        let idx = get_cache_idx(self.caches.len(), key);
        self.caches[idx].get(key, timestamp)
    }

    fn contains(&self, key: &K) -> bool {
        let idx = get_cache_idx(self.caches.len(), key);
        self.caches[idx].contains(key)
    }
}
