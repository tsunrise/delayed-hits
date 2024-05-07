use ahash::AHashMap;

use super::ObjectId;

struct ObjectMetaData {
    /// Number of miss windows this object has experienced.
    num_windows: usize,
    /// The total delay this object has experienced.
    cumulative_delay: usize,
    /// The timestamp of last miss
    window_start_timestamp: usize,
    /// The timestamp of last access. Used to compute the TTNA. (TTNA = curr_timestamp - last_access_timestamp + 1)
    /// We need TTNA to compute the ranking function score = estimated aggregate delay / TTNA. Higher score means higher priority.
    last_access_timestamp: usize,
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

    fn update(&mut self, timestamp: usize, estimated_miss_latency: usize) {
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

    fn score(&self, timestamp: usize) -> f64 {
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
}
