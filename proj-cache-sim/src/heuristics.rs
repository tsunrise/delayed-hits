//! Utilities for calculating the optimal cache size for a given workload.

use ahash::{AHashMap, AHashSet};
use proj_models::RequestEvent;

/// Get the maximum number of overlapping live ranges of objects in the workload at time. A live range is
/// an interval from the first access to the last access of an object.
pub fn maximum_active_objects(events: &[RequestEvent]) -> usize {
    let mut last_access = AHashMap::new();
    let mut last_event_timestamp = 0;
    for event in events {
        last_access.insert(event.key.clone(), event.timestamp);
        if event.timestamp < last_event_timestamp {
            panic!(
                "events are not in order: the event of key {:?} at timestamp {} is earlier than the last event at timestamp {}",
                event.key, event.timestamp, last_event_timestamp
            );
        }
        last_event_timestamp = event.timestamp;
    }
    let mut active_objects = AHashSet::new();
    let mut max_active_objects = 0;
    for event in events {
        active_objects.insert(event.key.clone());
        if let Some(last_access_time) = last_access.get(&event.key) {
            if event.timestamp == *last_access_time {
                active_objects.remove(&event.key);
                last_access.remove(&event.key);
            }
        }
        max_active_objects = max_active_objects.max(active_objects.len());
    }
    max_active_objects
}

/// Get the average time between the arrival of the same object in the workload.
pub fn mean_rearrive_interval(events: &[RequestEvent]) -> f64 {
    let mut total: u64 = 0;
    let mut count: u64 = 0;
    let mut last_access = AHashMap::new();

    for event in events {
        if let Some(last_access_time) = last_access.get(&event.key) {
            total += event
                .timestamp
                .checked_sub(*last_access_time)
                .expect("events are not in order");
            count += 1;
        }
        last_access.insert(event.key.clone(), event.timestamp);
    }

    if count == 0 {
        0.
    } else {
        total as f64 / count as f64
    }
}

pub struct IrtStatistics {
    /// buckets[i]: 10^i <= irt < 10^(i+1)
    pub buckets: [u64; 10],
    pub total: u128,
    pub count: u64,
}

impl IrtStatistics {
    fn new() -> Self {
        Self {
            buckets: [0; 10],
            total: 0,
            count: 0,
        }
    }

    fn add(&mut self, irt: u64) {
        self.total += irt as u128;
        self.count += 1;
        let mut c = irt;
        let mut bucket = 0;
        while c >= 10 && bucket < 9 {
            c /= 10;
            bucket += 1;
        }
        self.buckets[bucket as usize] += 1;
    }

    pub fn mean(&self) -> f64 {
        self.total as f64 / self.count as f64
    }
}

/// Get the inter-request time of the workload
pub fn get_irt(events: &[RequestEvent]) -> IrtStatistics {
    let mut irt = IrtStatistics::new();
    let mut last_timestamp = 0;
    for event in events {
        if event.timestamp < last_timestamp {
            panic!(
                "events are not in order: the event of key {:?} at timestamp {} is earlier than the last event at timestamp {}",
                event.key, event.timestamp, last_timestamp
            );
        }
        if last_timestamp != 0 {
            irt.add(event.timestamp - last_timestamp);
        }
        last_timestamp = event.timestamp;
    }
    irt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maximum_active_objects() {
        let events = [
            (1, 1),
            (2, 2),
            (3, 3),
            (1, 4),
            (2, 5),
            (3, 6),
            (1, 7),
            (2, 8),
            (3, 9),
            (4, 10),
            (5, 11),
            (4, 12),
        ]
        .iter()
        .map(|(key, timestamp)| RequestEvent {
            key: *key,
            timestamp: *timestamp,
        })
        .collect::<Vec<_>>();

        assert_eq!(maximum_active_objects(&events), 3);
    }

    #[test]
    fn test_avg_rearrive_interval() {
        let events = [(1, 1), (2, 2), (3, 3), (1, 4), (2, 5), (2, 6)]
            .iter()
            .map(|(key, timestamp)| RequestEvent {
                key: *key,
                timestamp: *timestamp,
            })
            .collect::<Vec<_>>();

        assert_eq!(mean_rearrive_interval(&events), (3 + 3 + 1) as f64 / 3.0);
    }
}
