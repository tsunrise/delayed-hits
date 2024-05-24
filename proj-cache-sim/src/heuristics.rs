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

/// Get the average time between the arrival of the same object in the workload.
pub fn median_rearrive_interval<T: ObjectId>(events: &[RequestEvent<T>]) -> f64 {
    let mut intervals = Vec::new();
    let mut last_access = AHashMap::new();

    for event in events {
        if let Some(last_access_time) = last_access.get(&event.key) {
            intervals.push(
                event
                    .timestamp
                    .checked_sub(*last_access_time)
                    .expect("events are not in order"),
            );
        }
        last_access.insert(event.key.clone(), event.timestamp);
    }

    if intervals.is_empty() {
        0.
    } else {
        let index = intervals.len() / 2;
        let (_, v, _) = intervals.select_nth_unstable(index);
        *v as f64
    }
}

/// Get the median inter-request time of the workload
pub fn irt<T: ObjectId>(events: &[RequestEvent<T>]) -> f64 {
    let mut intervals = events
        .windows(2)
        .map(|pair| pair[1].timestamp.checked_sub(pair[0].timestamp).unwrap())
        .collect::<Vec<_>>();

    if intervals.is_empty() {
        0.
    } else {
        let index = intervals.len() / 2;
        let (_, v, _) = intervals.select_nth_unstable(index);
        *v as f64
    }
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

        assert_eq!(median_rearrive_interval(&events), (3. + 3. + 1.) / 3.);
    }
}
