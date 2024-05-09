//! Parse the example provided in https://github.com/cmu-snap/Delayed-Hits/blob/master/data/trace.csv
//! Each line is in the format timestamp;key

use std::io::{BufRead, Read};

use proj_models::RequestEvent;

fn line_to_event(line: &str) -> Option<RequestEvent<u32>> {
    let mut parts = line.split(';');
    let timestamp = parts.next()?.parse().ok()?;
    let key = parts.next()?.parse().ok()?;
    Some(RequestEvent { key, timestamp })
}

pub fn read_example_events<R: Read>(reader: R) -> Vec<RequestEvent<u32>> {
    std::io::BufReader::new(reader)
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| line_to_event(&line))
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
