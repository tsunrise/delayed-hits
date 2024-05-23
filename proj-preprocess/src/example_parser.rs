//! Parse the example provided in https://github.com/cmu-snap/Delayed-Hits/blob/master/data/trace.csv
//! Each line is in the format timestamp;key

use std::io::{BufRead, Read};

use proj_models::RequestEvent;

fn line_to_event(line: &str, timestamp: usize) -> Option<RequestEvent<u32>> {
    let mut parts = line.split(';');
    let _ = parts.next()?;
    let key = parts.next()?.parse().ok()?;
    Some(RequestEvent {
        key,
        timestamp: timestamp as u64,
    })
}

pub fn read_example_events<R: Read>(reader: R) -> Vec<RequestEvent<u32>> {
    std::io::BufReader::new(reader)
        .lines()
        .enumerate()
        .filter_map(|line| line.1.ok().map(|l| (line.0, l)))
        .filter_map(|(timestamp, line)| line_to_event(&line, timestamp))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_events() {
        let expected = vec![
            RequestEvent {
                key: 2,
                timestamp: 1,
            },
            RequestEvent {
                key: 4,
                timestamp: 3,
            },
        ];
        let cases = vec![
            "1;2\n3;4\n",
            "1;2\n3;4",
            "1;2\n3;4\n\n",
            "1;2\n3;4\n\n\n",
            "1;2\n\n3;4\n\n\n",
        ];

        for case in cases {
            let events = read_example_events(case.as_bytes());
            assert_eq!(events, expected);
        }
    }
}
