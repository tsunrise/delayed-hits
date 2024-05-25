use std::{hash::Hash, io::Write};

use ahash::AHashMap;
use proj_models::{codec::Codec, RequestEvent};

use crate::models::RawRequestWithTimestamp;

/// remap the requests objects to [`proj_models::RequestEvent`] objects, and represent the time in nanoseconds, and serialize them to a byte stream.
pub fn post_process_requests<K, I, W>(requests: I, mut writer: W) -> std::io::Result<()>
where
    K: Eq + Hash + Clone,
    I: IntoIterator<Item = RawRequestWithTimestamp<K>>,
    W: Write,
{
    let mut map = AHashMap::new();
    let mut next_object_id: u64 = 0;
    let mut relative_time: u64 = 0;
    let mut last_timestamp: Option<u128> = None;
    for RawRequestWithTimestamp { request, timestamp } in requests {
        let timestamp = timestamp.as_nanos();
        if let Some(last) = last_timestamp {
            if timestamp < last {
                eprintln!(
                    "Warning: event not in order is ignored: the event at timestamp {} is earlier than the last request at timestamp {}",
                    timestamp, last
                );
                continue;
            } else {
                relative_time += (timestamp - last as u128) as u64;
            }
        }
        last_timestamp = Some(timestamp);
        let remapped = map
            .entry(request)
            .or_insert_with(|| {
                let id = next_object_id;
                next_object_id += 1;
                id
            })
            .clone();

        let request_event = RequestEvent {
            key: remapped,
            timestamp: relative_time,
        };

        request_event.to_bytes(&mut writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, time::Duration};

    use super::*;

    #[test]
    fn test_post_process_requests() {
        let requests = [
            ("a", 1),
            ("b", 2),
            ("a", 3),
            ("c", 4),
            ("b", 6),
            ("d", 5),
            ("a", 7),
        ]
        .iter()
        .map(|(k, t)| RawRequestWithTimestamp {
            request: k.to_string(),
            timestamp: Duration::from_secs(*t),
        });

        let mut buffer = Vec::new();
        post_process_requests(requests, &mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let processed = RequestEvent::repeat_read_till_end(&mut cursor)
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            processed,
            [(0, 0), (1, 1), (0, 2), (2, 3), (1, 5), (0, 6)]
                .iter()
                .map(|(k, t)| RequestEvent {
                    key: *k,
                    timestamp: *t * 1_000_000_000
                })
                .collect::<Vec<_>>()
        )
    }
}
