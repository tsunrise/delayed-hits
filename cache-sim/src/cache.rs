use std::{fmt::Debug, hash::{Hash, Hasher as _}};

use ahash::AHasher;

use crate::types::Timestamp;

pub trait ObjectId: Hash + Eq + PartialEq + Clone + Debug{
    fn get_hash(&self) -> u64{
        let mut hasher = AHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn get_cache<'a, C>(&self, caches: &'a mut [C]) -> &'a mut C{
        let index = (self.get_hash() as usize) % caches.len();
        &mut caches[index]
    }
}

impl<T: Hash + Eq + PartialEq + Clone + Debug> ObjectId for T{}


/// A cache store with no values stored.
pub trait Cache<K: ObjectId>{
    /// Write or revisit a key in the cache. Evict will happen if the cache is full.
    /// `write` should be called when
    /// - A key is accessed and there is a miss and the miss has been fetched from the backing store.
    /// - A key is accessed and cache hit occurs.
    /// 
    /// `timestamp` is only used for heuristics for the eviction policy (to compute the estimated TTNA)
    fn write(&mut self, key: K, timestamp: Timestamp);

    /// Check if a key is in the cache.
    fn contains(&self, key: &K) -> bool;

    /// Report an access to a key. This is only used as heuristics for the eviction policy. 
    /// `report_access` should be called when a key is accessed.
    fn report_access(&mut self, key: K, timestamp: Timestamp);
    
}