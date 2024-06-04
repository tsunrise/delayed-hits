use std::{collections::BTreeMap, io::Read, path::PathBuf, time::Duration};

use lazy_static::lazy_static;

use crate::models::RawRequestWithTimestamp;

/// - key: 11-??-2007.??-??-PM
/// - returned value: time in nanoseconds
fn get_start_time(key: &str) -> u64 {
    lazy_static! {
        static ref MAP: BTreeMap<String, u64> = {
            let raw_text = include_str!("ms_prod_map.txt");
            raw_text
                .lines()
                .map(|line| {
                    let mut parts = line.split(',');
                    let time = parts.next().unwrap().parse().unwrap();
                    let key = parts.next().unwrap().to_string();
                    (key, time)
                })
                .collect()
        };
    }

    *MAP.get(key).unwrap()
}

/// 24.hour.BuildServer.11-28-2007.07-24-PM.trace.csv -> 11-28-2007.07-24-PM
fn extract_key_from_filename(filename: &str) -> String {
    let parts: Vec<&str> = filename.split('.').collect();
    parts[3..=4].join(".")
}

fn skip_header<R: Read>(reader: &mut csv::Reader<R>) {
    loop {
        let mut record = csv::StringRecord::new();
        reader.read_record(&mut record).unwrap();
        if &record[0] == "EndHeader" {
            break;
        }
    }
}

fn custom_reader_builder() -> csv::ReaderBuilder {
    let mut builder = csv::ReaderBuilder::new();
    builder.flexible(true).trim(csv::Trim::All);
    builder
}

pub fn parse_lines<R: Read>(
    reader: R,
    start_time_us: u64,
) -> Vec<RawRequestWithTimestamp<(u64, u64)>> {
    let mut reader = custom_reader_builder().from_reader(reader);
    skip_header(&mut reader);
    reader
        .records()
        .filter_map(|record| {
            let record = record.ok()?;
            match record.get(0) {
                Some("DiskRead") | Some("DiskWrite") => {
                    let mut timestamp_us = record
                        .get(1)?
                        .parse::<u64>()
                        .expect("failed to parse timestamp to u64");
                    timestamp_us += start_time_us;
                    let timestamp = Duration::from_micros(timestamp_us as u64);
                    let irp_ptr =
                        u64::from_str_radix(record.get(4)?.strip_prefix("0x")?, 16).ok()?;
                    let disk_num = record.get(8)?.parse().ok()?;
                    Some(RawRequestWithTimestamp {
                        request: (irp_ptr, disk_num),
                        timestamp,
                    })
                }
                _ => None,
            }
        })
        .collect()
}

pub fn parse_files(
    paths: Vec<PathBuf>,
) -> impl Iterator<Item = RawRequestWithTimestamp<(u64, u64)>> {
    let mut paths_with_start_time = paths
        .into_iter()
        .map(|path| {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let key = extract_key_from_filename(filename);
            let start_time = get_start_time(&key);
            (path, start_time)
        })
        .collect::<Vec<_>>();

    paths_with_start_time.sort_by_key(|(_, start_time)| *start_time);
    paths_with_start_time
        .into_iter()
        .flat_map(|(path, start_time)| {
            let file = std::fs::File::open(path.clone()).unwrap();
            println!("Processing {:?}", path);
            parse_lines(file, start_time)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_start_time() {
        assert_eq!(get_start_time("11-28-2007.08-40-PM"), 128407848315052239);
        assert_eq!(get_start_time("11-28-2007.08-55-PM"), 128407857426623047);
        assert_eq!(get_start_time("11-28-2007.09-10-PM"), 128407866558800898);
        assert_eq!(get_start_time("11-28-2007.07-24-PM"), 128407802885377025);
    }

    #[test]
    fn test_extract_key_from_filename() {
        assert_eq!(
            extract_key_from_filename("24.hour.BuildServer.11-28-2007.07-24-PM.trace.csv"),
            "11-28-2007.07-24-PM"
        );
        assert_eq!(
            extract_key_from_filename("24.hour.BuildServer.11-28-2007.08-40-PM.trace.csv"),
            "11-28-2007.08-40-PM"
        );
        assert_eq!(
            extract_key_from_filename("24.hour.BuildServer.11-28-2007.08-55-PM.trace.csv"),
            "11-28-2007.08-55-PM"
        );
        assert_eq!(
            extract_key_from_filename("24.hour.BuildServer.11-28-2007.09-10-PM.trace.csv"),
            "11-28-2007.09-10-PM"
        );
    }

    #[test]
    fn test_read() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-resources/MSNStorageFileServer-sample.csv");
        let file = std::fs::File::open(path).unwrap();
        let events = parse_lines(file, 17);
        assert_eq!(
            events[0],
            RawRequestWithTimestamp {
                request: (0xfffffadf39860010, 4),
                timestamp: Duration::from_micros(260959 + 17)
            }
        );

        assert_eq!(
            events[1],
            RawRequestWithTimestamp {
                request: (0xfffffadf3b9ca010, 4),
                timestamp: Duration::from_micros(261233 + 17)
            }
        );

        assert_eq!(
            events[2],
            RawRequestWithTimestamp {
                request: (0xfffffadf39c0a930, 5),
                timestamp: Duration::from_micros(263262 + 17)
            }
        )
    }
}
