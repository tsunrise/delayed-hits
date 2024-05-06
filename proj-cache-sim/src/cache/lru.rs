use linked_hash_map::LinkedHashMap;

use crate::types::Timestamp;

use super::{Cache, ObjectId};

pub struct LRU<K: ObjectId, V> {
    capacity: usize,
    store: LinkedHashMap<K, V, ahash::RandomState>,
}

impl<K: ObjectId, V> LRU<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity should be greater than 0");
        Self {
            capacity,
            store: LinkedHashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }
}

impl<K: ObjectId, V> Cache<K, V> for LRU<K, V> {
    fn write(&mut self, key: K, value: V, _timestamp: Timestamp) {
        if self.store.contains_key(&key) {
            self.store.insert(key, value);
        } else {
            debug_assert!(self.store.len() <= self.capacity);
            if self.store.len() == self.capacity {
                self.store.pop_front();
            }
            self.store.insert(key, value);
        }
    }

    fn get(&mut self, key: &K, _timestamp: Timestamp) -> Option<&V> {
        self.store.get_refresh(key).map(|v| &*v)
    }

    fn contains(&self, key: &K) -> bool {
        self.store.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use crate::simulator::{run_simulation, RequestResult};

    use super::*;

    #[test]
    fn test_lru() {
        let mut lru = LRU::new(2);
        lru.write(1, (), 0); // 1
        lru.write(2, (), 0); // 1 2
        assert_eq!(lru.contains(&1), true);
        assert_eq!(lru.contains(&2), true);
        lru.write(3, (), 0); // 2 3
        assert_eq!(lru.contains(&1), false);
        assert_eq!(lru.contains(&2), true);
        assert_eq!(lru.contains(&3), true);
        lru.write(2, (), 0); // 3 2
        lru.write(4, (), 0); // 2 4
        assert_eq!(lru.contains(&2), true);
        assert_eq!(lru.contains(&3), false);
        assert_eq!(lru.contains(&4), true);
    }

    #[test]
    fn test_lru_cache_simulator() {
        let mut cache = LRU::new(2);
        let delay = 5;
        const A: u32 = 0;
        const B: u32 = 1;
        const C: u32 = 2;
        let requests = vec![
            // comment is complete timestamp, and cache
            (B, 0), // 5 [] // cache if only written upon completion
            (A, 1), // 6 []
            (A, 4), // 6 []
            (A, 5), // 6 []
            // B complete at 5 [B]
            // A complete at 6 [B A]
            (B, 7), // 7 [A B]
            (C, 8), // 13 [A B] // still, cache if only written upon completion
            (A, 9), // 9 [B A]
            // C complete at 13 [A C]
            (B, 14), // 19 [A C]
            (C, 15), // 15 [A C]
            // B complete at 19 [C B]
            (A, 19), // 19 [B A] // subtle: if request and completion event are at the same time, we fulfill request first and then process completion.
            (C, 20), // 25 [B A]
        ];
        let mut results = run_simulation(&mut cache, requests, delay);
        results.sort_by_key(|r| r.request_timestamp);
        assert_eq!(
            results,
            [
                (B, 0, 5),
                (A, 1, 6),
                (A, 4, 6),
                (A, 5, 6),
                (B, 7, 7),
                (C, 8, 13),
                (A, 9, 9),
                (B, 14, 19),
                (C, 15, 15),
                (A, 19, 19),
                (C, 20, 25)
            ]
            .into_iter()
            .map(
                |(key, request_timestamp, completion_timestamp)| RequestResult {
                    key,
                    request_timestamp,
                    completion_timestamp
                }
            )
            .collect::<Vec<_>>()
        );
    }
}
