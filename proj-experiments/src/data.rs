use proj_models::RequestEvents;
use serde::de::DeserializeOwned;

pub fn load_example_events(path: &str) -> RequestEvents<u32> {
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    bincode::deserialize_from(reader).unwrap()
}

/// Load network traces from a preprocessed events file.
pub fn load_events<T>(path: &str) -> RequestEvents<T>
where
    T: DeserializeOwned,
{
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    bincode::deserialize_from(reader).unwrap()
}
