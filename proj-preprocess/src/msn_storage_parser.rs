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

pub fn read_msn_storage_events<R: Read>(reader: R) -> csv::Result<Vec<RequestEvent<BlockId>>> {
    let mut reader = custom_reader_builder().from_reader(reader);

    skip_header(&mut reader)?;
    todo!()
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
}
