//! Parse the trace file format used by the authors of the paper.
//! Each line is in the format timestamp;key

use std::{io::BufRead, time::Duration};

use crate::models::RawRequestWithTimestamp;

fn line_to_request(line: &str) -> Option<RawRequestWithTimestamp<u128>> {
    // example line: 1500598765.959000000;131428753696874742190000;;0;
    let mut parts = line.split(';');

    let mut timestamp_it = parts.next()?.split('.');
    let second = timestamp_it.next()?.parse::<u64>().ok()?;
    let nsec = timestamp_it.next()?.parse::<u32>().ok()?;
    let timestamp = Duration::new(second, nsec);

    let object_id = parts.next()?.parse::<u128>().ok()?;
    Some(RawRequestWithTimestamp {
        request: object_id,
        timestamp,
    })
}

pub fn read_cdn_requests<R: BufRead>(
    reader: R,
) -> impl Iterator<Item = RawRequestWithTimestamp<u128>> {
    reader
        .lines()
        .filter_map(|line| line_to_request(&line.unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_to_request() {
        let line = "1500598765.959000000;131428753696874742190000;;0;";
        let request = line_to_request(line).unwrap();
        assert_eq!(request.request, 131428753696874742190000);
        assert_eq!(request.timestamp.as_secs(), 1500598765);
        assert_eq!(request.timestamp.subsec_nanos(), 959000000);
    }
}
