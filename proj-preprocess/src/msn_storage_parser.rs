#![allow(dead_code)]
use std::io::Read;

use proj_models::{storage::BlockId, RequestEvent};

fn skip_header<R: Read>(reader: &mut csv::Reader<R>) -> csv::Result<()> {
    loop {
        let mut record = csv::StringRecord::new();
        reader.read_record(&mut record)?;
        if &record[0] == "EndHeader" {
            break;
        }
    }
    Ok(())
}

fn custom_reader_builder() -> csv::ReaderBuilder {
    let mut builder = csv::ReaderBuilder::new();
    builder.flexible(true).trim(csv::Trim::All);
    builder
}

pub fn read_msn_storage_events<R: Read>(reader: R) -> Vec<RequestEvent<BlockId>> {
    let mut reader = custom_reader_builder().from_reader(reader);

    skip_header(&mut reader).unwrap();
    reader
        .records()
        .filter_map(|record| {
            let record = record.ok()?;
            match record.get(0) {
                Some("DiskRead") | Some("DiskWrite") => {
                    let timestamp = record.get(1)?.parse().ok()?;
                    let irp_ptr =
                        u64::from_str_radix(record.get(4)?.strip_prefix("0x")?, 16).ok()?;
                    let disk_num = record.get(8)?.parse().ok()?;
                    Some(RequestEvent {
                        key: BlockId { irp_ptr, disk_num },
                        timestamp,
                    })
                }
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_header() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-resources/MSNStorageFileServer-sample.csv");
        let file = std::fs::File::open(path).unwrap();
        let mut reader = custom_reader_builder().from_reader(file);
        skip_header(&mut reader).unwrap();
    }

    #[test]
    fn test_read() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-resources/MSNStorageFileServer-sample.csv");
        let file = std::fs::File::open(path).unwrap();
        let events = read_msn_storage_events(file);
        assert_eq!(
            events[0],
            RequestEvent {
                key: BlockId {
                    irp_ptr: 0xfffffadf39860010,
                    disk_num: 4
                },
                timestamp: 260959
            }
        );

        assert_eq!(
            events[1],
            RequestEvent {
                key: BlockId {
                    irp_ptr: 0xfffffadf3b9ca010,
                    disk_num: 4
                },
                timestamp: 261233
            }
        );

        assert_eq!(
            events[2],
            RequestEvent {
                key: BlockId {
                    irp_ptr: 0xfffffadf39c0a930,
                    disk_num: 5
                },
                timestamp: 263262
            }
        );
    }
}
