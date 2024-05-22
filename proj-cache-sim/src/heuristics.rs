//! Utilities for calculating the optimal cache size for a given workload.

use ahash::{AHashMap, AHashSet};
use proj_models::RequestEvent;

use crate::cache::ObjectId;

/// Get the maximum number of overlapping live ranges of objects in the workload at time. A live range is
/// an interval from the first access to the last access of an object.
pub fn maximum_active_objects<T: ObjectId>(events: &[RequestEvent<T>]) -> usize {
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
}
