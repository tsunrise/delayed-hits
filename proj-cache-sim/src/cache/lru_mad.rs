use ahash::AHashMap;

use super::{Cache, ObjectId};

struct ObjectMetaData {
    /// Number of miss windows this object has experienced.
    num_windows: usize,
    /// The total delay this object has experienced.
    cumulative_delay: u64,
    /// The timestamp of last miss
    window_start_timestamp: u64,
    /// The timestamp of last access. Used to compute the TTNA. (TTNA = curr_timestamp - last_access_timestamp + 1)
    /// We need TTNA to compute the ranking function score = estimated aggregate delay / TTNA. Higher score means higher priority.
    last_access_timestamp: u64,
}

impl ObjectMetaData {
    fn new() -> Self {
        Self {
            num_windows: 0,
            cumulative_delay: 0,
            window_start_timestamp: 0,
            last_access_timestamp: 0,
        }
    }

    fn update(&mut self, timestamp: u64, estimated_miss_latency: u64) {
        let tssw = timestamp - self.window_start_timestamp;

        if tssw >= estimated_miss_latency {
            self.num_windows += 1;
            self.cumulative_delay += estimated_miss_latency;
            self.window_start_timestamp = timestamp;
        } else {
            self.cumulative_delay += estimated_miss_latency - tssw;
        }

        self.last_access_timestamp = timestamp;
    }

    fn score(&self, timestamp: u64) -> f64 {
        let estimated_agg_delay = self.cumulative_delay as f64 / self.num_windows as f64;
        debug_assert!(
            timestamp >= self.last_access_timestamp,
            "timestamp should be greater than or equal to last_access_timestamp"
        );
        let ttna = timestamp - self.last_access_timestamp + 1;
        estimated_agg_delay / ttna as f64
    }
}

pub struct LRUMinAD<K: ObjectId, V> {
    capacity: usize,
    value_store: AHashMap<K, V>,
    metadata_store: AHashMap<K, ObjectMetaData>,
    estimated_miss_latency: u64,
}

impl<K: ObjectId, V> LRUMinAD<K, V> {
    pub fn new(capacity: usize, estimated_miss_latency: u64) -> Self {
        Self {
            capacity,
            value_store: AHashMap::new(),
            metadata_store: AHashMap::new(),
            estimated_miss_latency,
        }
    }
}

impl<K: ObjectId, V> Cache<K, V> for LRUMinAD<K, V> {
    fn write(&mut self, key: K, value: V, timestamp: crate::types::Timestamp) {
        if self.value_store.contains_key(&key) {
            self.value_store.insert(key, value);
        } else {
            if self.value_store.len() == self.capacity {
                let key_to_evict = self
                    .value_store
                    .keys()
                    .map(|k| (k, self.metadata_store.get(k).unwrap().score(timestamp)))
                    .min_by(|(_, score1), (_, score2)| score1.partial_cmp(score2).unwrap())
                    .expect("value_store should not be empty")
                    .0
                    .clone();
                self.value_store.remove(&key_to_evict);
                // key is kept in metadata_store forever, at this point
            }
            self.value_store.insert(key, value);
            debug_assert!(self.value_store.len() <= self.capacity);
        }
    }

    fn get(&mut self, key: &K, timestamp: crate::types::Timestamp) -> Option<&V> {
        self.metadata_store
            .entry(key.clone())
            .or_insert_with(ObjectMetaData::new)
            .update(timestamp, self.estimated_miss_latency);

        self.value_store.get(key)
    }

    fn contains(&self, key: &K) -> bool {
        self.value_store.contains_key(key)
    }
}
