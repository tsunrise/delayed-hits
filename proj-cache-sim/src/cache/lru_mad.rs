use ahash::AHashMap;
use proj_models::TimeUnit;

use super::{Cache, ObjectId};

struct ObjectMetaData {
    /// Whether the metadata is not updated yet
    /// Credit: we initially do not include this flag and thus our MAD has subpar performance. We referenced original authors' C++ code
    /// and discovered this bug. Refernece: https://github.com/cmu-snap/Delayed-Hits/blob/4f21d4c5bea26262715b88c97cd66ece7cdb965e/caching/src/cache_lru_aggdelay.cpp#L35
    new: bool,
    /// Number of miss windows this object has experienced.
    num_windows: usize,
    /// The total delay this object has experienced.
    cumulative_delay: TimeUnit,
    /// The timestamp of last miss
    window_start_timestamp: TimeUnit,
    /// The timestamp of last access. Used to compute the TTNA. (TTNA = curr_timestamp - last_access_timestamp + 1)
    /// We need TTNA to compute the ranking function score = estimated aggregate delay / TTNA. Higher score means higher priority.
    last_access_timestamp: TimeUnit,
}

impl ObjectMetaData {
    fn new() -> Self {
        Self {
            new: true,
            num_windows: 0,
            cumulative_delay: 0,
            window_start_timestamp: 0,
            last_access_timestamp: 0,
        }
    }

    fn update(&mut self, timestamp: TimeUnit, estimated_miss_latency: TimeUnit) {
        let tssw = timestamp - self.window_start_timestamp;

        if self.new || tssw >= estimated_miss_latency {
            self.num_windows += 1;
            self.window_start_timestamp = timestamp;
            self.cumulative_delay += estimated_miss_latency;
        } else {
            self.cumulative_delay += estimated_miss_latency - tssw;
        }

        self.last_access_timestamp = timestamp;
        self.new = false;
    }

    fn score(&self, timestamp: TimeUnit) -> f64 {
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
    estimated_miss_latency: TimeUnit,
}

impl<K: ObjectId, V> LRUMinAD<K, V> {
    pub fn new(capacity: usize, estimated_miss_latency: TimeUnit) -> Self {
        Self {
            capacity,
            value_store: AHashMap::new(),
            metadata_store: AHashMap::new(),
            estimated_miss_latency,
        }
    }
}

impl<K: ObjectId, V> Cache<K, V> for LRUMinAD<K, V> {
    fn write(&mut self, key: K, value: V, timestamp: TimeUnit) {
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

    fn get(&mut self, key: &K, timestamp: TimeUnit) -> Option<&V> {
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
