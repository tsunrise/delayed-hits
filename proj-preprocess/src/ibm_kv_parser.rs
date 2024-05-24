//! Parse IBM key-value traces
//!
//! Example Input:
//! ```
//! 1219008 REST.PUT.OBJECT 8d4fcda3d675bac9 1056
//! 1221974 REST.HEAD.OBJECT 39d177fb735ac5df 528
//! 1232437 REST.HEAD.OBJECT 3b8255e0609a700d 1456
//! ```
//!
//! Example Output:
//! ```rust
//! RequestEvent {key: 0x8d4fcda3d675bac9, timestamp: 1219008} // timestamp in ms
//! RequestEvent {key: 0x39d177fb735ac5df, timestamp: 1221974}
//! RequestEvent {key: 0x3b8255e0609a700d, timestamp: 1232437}
//! ```

use std::io::{BufRead, BufReader, Read};

use proj_models::{storage::KVObjectId, RequestEvent};

pub fn read_ibm_kv_events<R: Read>(reader: R) -> Vec<RequestEvent<KVObjectId>> {
    let reader = BufReader::new(reader);
    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let mut parts = line.split_whitespace();
            let timestamp = parts.next()?.parse().ok()?;
            let _ = parts.next()?;
            let key = u64::from_str_radix(parts.next()?, 16).ok()?;
            Some(RequestEvent { key, timestamp })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_ibm_kv_events() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-resources/IBMObjectStoreSample");
        let file = std::fs::File::open(path).unwrap();
        let events = read_ibm_kv_events(file);
        assert_eq!(
            events[0],
            RequestEvent {
                key: 0x8d4fcda3d675bac9,
                timestamp: 1219008
            }
        );
        assert_eq!(
            events[1],
            RequestEvent {
                key: 0x39d177fb735ac5df,
                timestamp: 1221974
            }
        );
        assert_eq!(
            events[2],
            RequestEvent {
                key: 0x3b8255e0609a700d,
                timestamp: 1232437
            }
        );
    }
}
